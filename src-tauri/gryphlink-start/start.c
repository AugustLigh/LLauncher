/* gryphlink-start.exe <Games.exe> <Endfield.exe> [game arguments...]
 *
 * Since 1.6 the game only runs when the official launcher started it: at
 * startup it looks up its parent process and checks the image's name and
 * Authenticode signature. Under Proton the parent is `steam.exe`, natively it
 * is LLauncher, so the check fails and the game tries to start GRYPHLINK
 * instead. GRYPHLINK's own starter is `Games.exe`.
 *
 * So start `Games.exe` suspended — it never runs a line of code, no window
 * shows — and create the game with that process as its parent
 * (PROC_THREAD_ATTRIBUTE_PARENT_PROCESS). Then wait for the game, take the
 * suspended starter down with it and exit with the game's exit code, so
 * whoever watches this process sees the game's lifetime.
 *
 * Everything after the second argument is passed to the game verbatim. */
typedef unsigned short WCHAR;
typedef unsigned long DWORD;
typedef int BOOL;
typedef void *HANDLE;
typedef unsigned long long SIZE_T;
typedef unsigned long long DWORD_PTR;

typedef struct {
    DWORD cb;
    WCHAR *lpReserved, *lpDesktop, *lpTitle;
    DWORD dwX, dwY, dwXSize, dwYSize, dwXCountChars, dwYCountChars, dwFillAttribute, dwFlags;
    unsigned short wShowWindow, cbReserved2;
    unsigned char *lpReserved2;
    HANDLE hStdInput, hStdOutput, hStdError;
} STARTUPINFOW;

typedef struct {
    STARTUPINFOW StartupInfo;
    void *lpAttributeList;
} STARTUPINFOEXW;

typedef struct {
    HANDLE hProcess, hThread;
    DWORD dwProcessId, dwThreadId;
} PROCESS_INFORMATION;

#define CREATE_SUSPENDED 0x00000004
#define EXTENDED_STARTUPINFO_PRESENT 0x00080000
#define PROC_THREAD_ATTRIBUTE_PARENT_PROCESS 0x00020000
#define INFINITE 0xFFFFFFFF

__declspec(dllimport) BOOL __stdcall CreateProcessW(const WCHAR *, WCHAR *, void *, void *, BOOL,
                                                    DWORD, void *, const WCHAR *, STARTUPINFOW *,
                                                    PROCESS_INFORMATION *);
__declspec(dllimport) BOOL __stdcall InitializeProcThreadAttributeList(void *, DWORD, DWORD, SIZE_T *);
__declspec(dllimport) BOOL __stdcall UpdateProcThreadAttribute(void *, DWORD, DWORD_PTR, void *, SIZE_T,
                                                               void *, SIZE_T *);
__declspec(dllimport) void __stdcall DeleteProcThreadAttributeList(void *);
__declspec(dllimport) DWORD __stdcall WaitForSingleObject(HANDLE, DWORD);
__declspec(dllimport) BOOL __stdcall GetExitCodeProcess(HANDLE, DWORD *);
__declspec(dllimport) BOOL __stdcall TerminateProcess(HANDLE, unsigned);
__declspec(dllimport) BOOL __stdcall CloseHandle(HANDLE);
__declspec(dllimport) DWORD __stdcall GetLastError(void);
__declspec(dllimport) HANDLE __stdcall GetProcessHeap(void);
__declspec(dllimport) void *__stdcall HeapAlloc(HANDLE, DWORD, SIZE_T);
__declspec(dllimport) const WCHAR *__stdcall GetCommandLineW(void);
__declspec(dllimport) HANDLE __stdcall GetStdHandle(DWORD);
__declspec(dllimport) BOOL __stdcall WriteFile(HANDLE, const void *, DWORD, DWORD *, void *);
__declspec(dllimport) __attribute__((noreturn)) void __stdcall ExitProcess(unsigned);
__declspec(dllimport) WCHAR **__stdcall CommandLineToArgvW(const WCHAR *, int *);

static void say(const char *s) {
    DWORD n = 0, w;
    while (s[n]) n++;
    WriteFile(GetStdHandle((DWORD)-11), s, n, &w, 0);
}

static void say_error(const char *what, DWORD code) {
    char buf[16];
    int i = sizeof buf - 1;
    buf[i] = 0;
    do buf[--i] = '0' + code % 10; while ((code /= 10) && i > 0);
    say("gryphlink-start: ");
    say(what);
    say(" failed, error ");
    say(buf + i);
    say("\n");
}

static int is_space(WCHAR c) { return c == ' ' || c == '\t'; }

/* Step over one argument of a command line, with the quoting rules of
 * CommandLineToArgvW (2n backslashes + quote: n backslashes and a quote that
 * toggles quoting; 2n+1: n backslashes and a literal quote), and the space
 * after it. The program name is the exception: quoted or not, nothing inside
 * it is escaped. */
static const WCHAR *skip_arg(const WCHAR *p, int program_name) {
    int quoted = 0;
    if (program_name) {
        if (*p == '"') {
            for (p++; *p && *p != '"'; p++);
            if (*p) p++;
        } else {
            while (*p && !is_space(*p)) p++;
        }
    } else {
        while (*p && (quoted || !is_space(*p))) {
            if (*p == '\\') {
                int n = 0;
                while (*p == '\\') p++, n++;
                if (*p == '"') {
                    if (n % 2 == 0) quoted = !quoted;
                    p++;
                }
            } else {
                if (*p == '"') quoted = !quoted;
                p++;
            }
        }
    }
    while (is_space(*p)) p++;
    return p;
}

void start(void) {
    int argc;
    const WCHAR *cmdline = GetCommandLineW();
    WCHAR **argv = CommandLineToArgvW(cmdline, &argc);
    if (!argv || argc < 3) {
        say("usage: gryphlink-start <Games.exe> <Endfield.exe> [arguments...]\n");
        ExitProcess(2);
    }

    /* "<starter>" — its own command line, the way the official launcher
     * would leave it. */
    SIZE_T n = 0;
    while (argv[1][n]) n++;
    WCHAR *starter_cmd = HeapAlloc(GetProcessHeap(), 0, (n + 3) * sizeof(WCHAR));
    starter_cmd[0] = '"';
    for (SIZE_T i = 0; i < n; i++) starter_cmd[i + 1] = argv[1][i];
    starter_cmd[n + 1] = '"';
    starter_cmd[n + 2] = 0;

    STARTUPINFOW si = {0};
    si.cb = sizeof si;
    PROCESS_INFORMATION starter;
    if (!CreateProcessW(argv[1], starter_cmd, 0, 0, 0, CREATE_SUSPENDED, 0, 0, &si, &starter)) {
        say_error("starting Games.exe", GetLastError());
        ExitProcess(3);
    }

    /* "<game>" <the rest of our own command line, untouched> */
    const WCHAR *rest = skip_arg(skip_arg(skip_arg(cmdline, 1), 0), 0);
    SIZE_T glen = 0, rlen = 0;
    while (argv[2][glen]) glen++;
    while (rest[rlen]) rlen++;
    WCHAR *game_cmd = HeapAlloc(GetProcessHeap(), 0, (glen + rlen + 4) * sizeof(WCHAR));
    SIZE_T k = 0;
    game_cmd[k++] = '"';
    for (SIZE_T i = 0; i < glen; i++) game_cmd[k++] = argv[2][i];
    game_cmd[k++] = '"';
    if (rlen) {
        game_cmd[k++] = ' ';
        for (SIZE_T i = 0; i < rlen; i++) game_cmd[k++] = rest[i];
    }
    game_cmd[k] = 0;

    SIZE_T size = 0;
    InitializeProcThreadAttributeList(0, 1, 0, &size);
    void *attrs = HeapAlloc(GetProcessHeap(), 0, size);
    HANDLE parent = starter.hProcess;
    if (!attrs || !InitializeProcThreadAttributeList(attrs, 1, 0, &size) ||
        !UpdateProcThreadAttribute(attrs, 0, PROC_THREAD_ATTRIBUTE_PARENT_PROCESS, &parent,
                                   sizeof parent, 0, 0)) {
        say_error("setting the parent process", GetLastError());
        TerminateProcess(starter.hProcess, 1);
        ExitProcess(3);
    }

    STARTUPINFOEXW six = {0};
    six.StartupInfo.cb = sizeof six;
    six.lpAttributeList = attrs;
    PROCESS_INFORMATION game;
    if (!CreateProcessW(argv[2], game_cmd, 0, 0, 0, EXTENDED_STARTUPINFO_PRESENT, 0, 0,
                        &six.StartupInfo, &game)) {
        DWORD err = GetLastError();
        say_error("starting the game", err);
        TerminateProcess(starter.hProcess, 1);
        /* 740 is ERROR_ELEVATION_REQUIRED: pass it on so it stays
         * recognisable. */
        ExitProcess(err == 740 ? 740 : 3);
    }
    DeleteProcThreadAttributeList(attrs);
    say("gryphlink-start: game started\n");

    WaitForSingleObject(game.hProcess, INFINITE);
    DWORD code = 1;
    GetExitCodeProcess(game.hProcess, &code);
    TerminateProcess(starter.hProcess, 0);
    ExitProcess(code);
}

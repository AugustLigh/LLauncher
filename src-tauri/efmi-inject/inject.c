/* efmi-inject.exe <3dmloader.dll> <d3d11.dll> <process.exe> <timeout_s> */
typedef unsigned short WCHAR;
typedef unsigned long DWORD;
typedef int BOOL;
typedef void *HANDLE;
typedef unsigned long long SIZE_T;

typedef struct {
    DWORD dwSize, cntUsage, th32ProcessID;
    SIZE_T th32DefaultHeapID;
    DWORD th32ModuleID, cntThreads, th32ParentProcessID;
    long pcPriClassBase;
    DWORD dwFlags;
    WCHAR szExeFile[260];
} PROCESSENTRY32W;

__declspec(dllimport) HANDLE __stdcall CreateToolhelp32Snapshot(DWORD, DWORD);
__declspec(dllimport) BOOL __stdcall Process32FirstW(HANDLE, PROCESSENTRY32W *);
__declspec(dllimport) BOOL __stdcall Process32NextW(HANDLE, PROCESSENTRY32W *);
__declspec(dllimport) BOOL __stdcall CloseHandle(HANDLE);
__declspec(dllimport) void __stdcall Sleep(DWORD);
__declspec(dllimport) void *__stdcall LoadLibraryW(const WCHAR *);
__declspec(dllimport) void *__stdcall GetProcAddress(void *, const char *);
__declspec(dllimport) const WCHAR *__stdcall GetCommandLineW(void);
__declspec(dllimport) HANDLE __stdcall GetStdHandle(DWORD);
__declspec(dllimport) BOOL __stdcall WriteFile(HANDLE, const void *, DWORD, DWORD *, void *);
__declspec(dllimport) __attribute__((noreturn)) void __stdcall ExitProcess(unsigned);
__declspec(dllimport) WCHAR **__stdcall CommandLineToArgvW(const WCHAR *, int *);

typedef int (*InjectFn)(DWORD pid, const WCHAR *dll, int timeout);

static void say(const char *s) {
    DWORD n = 0, w;
    while (s[n]) n++;
    WriteFile(GetStdHandle((DWORD)-11), s, n, &w, 0);
}

static int wieq(const WCHAR *a, const WCHAR *b) {
    for (;; a++, b++) {
        WCHAR x = *a >= 'A' && *a <= 'Z' ? *a + 32 : *a;
        WCHAR y = *b >= 'A' && *b <= 'Z' ? *b + 32 : *b;
        if (x != y) return 0;
        if (!x) return 1;
    }
}

static DWORD find_pid(const WCHAR *name) {
    PROCESSENTRY32W e;
    DWORD pid = 0;
    HANDLE snap = CreateToolhelp32Snapshot(2, 0);
    if (snap == (HANDLE)-1) return 0;
    e.dwSize = sizeof e;
    for (BOOL ok = Process32FirstW(snap, &e); ok; ok = Process32NextW(snap, &e))
        if (wieq(e.szExeFile, name)) {
            pid = e.th32ProcessID;
            break;
        }
    CloseHandle(snap);
    return pid;
}

void start(void) {
    int argc;
    WCHAR **argv = CommandLineToArgvW(GetCommandLineW(), &argc);
    if (argc < 5) { say("usage: efmi-inject <3dmloader.dll> <d3d11.dll> <process.exe> <timeout_s>\n"); ExitProcess(2); }
    int timeout = 0;
    for (const WCHAR *p = argv[4]; *p >= '0' && *p <= '9' && timeout < 86400; p++)
        timeout = timeout * 10 + (*p - '0');

    void *lib = LoadLibraryW(argv[1]);
    InjectFn inject = lib ? (InjectFn)GetProcAddress(lib, "Inject") : 0;
    if (!inject) { say("efmi-inject: cannot load Inject from 3dmloader\n"); ExitProcess(3); }

    for (int i = 0; i < timeout * 10; i++, Sleep(100)) {
        DWORD pid = find_pid(argv[3]);
        if (pid) {
            say("efmi-inject: injecting\n");
            ExitProcess(inject(pid, argv[2], timeout) == 0 ? 0 : 4);
        }
    }
    say("efmi-inject: timed out waiting for the process\n");
    ExitProcess(5);
}

# Wine patches for Arknights: Endfield on macOS

Applied, in order, on top of Wine + the full wine-staging patch set of the
same version (the sources the official WineHQ macOS packages are built from),
by `../build-modules.sh`. Only four modules come out of the build —
`ntdll.so`, `ntdll.dll`, `kernel32.dll`, `ntoskrnl.exe` — and the launcher
drops them into the unmodified WineHQ bundle; nothing else in Wine changes.

| Patch | What it fixes | Origin |
| --- | --- | --- |
| `0001-ntoskrnl-…` | The kernel exports the ACE anti-cheat driver (`ACE-BASE.sys`) calls and Wine leaves as stubs — `PsGetProcessImageFileName`, `SeLocateProcessImageName`, `PsReferencePrimaryToken`, `PsGetProcessCreateTimeQuadPart`, `PsGetProcessSessionId`, `PsGetThreadProcess`, `PsGetContextThread`, `MmGetPhysicalMemoryRanges`, `MmGetVirtualForPhysical`, the `KeRegisterBugCheck*` family, `KeCapturePersistentThreadState` — plus `PsGetProcessExitStatus`, the one call left aborting after the rest. | dw-proton's `em-backports` (Etaash Mathamsetty), rebased onto Wine 11.16 where `KeAcquireGuardedMutex`/`KeReleaseGuardedMutex` already exist; `PsGetProcessExitStatus` is new here |
| `0002-kernel32-…` | The `int3` stubs `GetProcAddress` hands the game's VMProtect/TenProtect layer for `KiUserApcDispatcher` / `KiUserCallbackDispatcher`, limited to `Endfield.exe` | dw-proton (Ziia Shi / mkrsym1, NelloKudo) |
| `0003-ntdll-…` | `NtDelayExecution` relative waits timed by `QueryPerformanceCounter` — the anti-cheat is timing-sensitive | dw-proton (Etaash Mathamsetty) |
| `0004-ntdll-…` | Two Rosetta 2 bugs, macOS only: it raises an invalid-opcode fault on multi-byte NOPs (`0F 1F`, which the protector emits by the hundred thousand) and reports the driver's `mov reg, cr3` anti-VM probe as invalid-opcode instead of the `#GP` a real CPU gives, so ACE got `EXCEPTION_ILLEGAL_INSTRUCTION` where Linux gives `EXCEPTION_PRIV_INSTRUCTION` ("driver error 13") | [Endfield_FineWine](https://github.com/stoicswe/Endfield_FineWine) (stoicswe), rewritten for upstream Wine's signal handler — the original targets CrossOver's |

The dw-proton set is the Endfield subset of [dw-proton](https://dawn.wine/)
(Dawn Winery), the Linux compatibility layer this launcher installs there; the
macOS port of it — and the discovery that these plus the Rosetta fixes are
all it takes — is the work of the Endfield_FineWine project, which proved the
combination on CrossOver 26.2. This directory carries the same idea over to
the open-source WineHQ builds.

## License

Everything here modifies Wine and is therefore **LGPL-2.1-or-later**, Wine's
license. The dw-proton patches keep their upstream authorship (see above);
the Rosetta fixes are derived from Endfield_FineWine's, which are LGPL for the
same reason. The complete corresponding source of a published module set is
the Wine tag, the wine-staging tag and this directory, all named in the
`MODULES.txt` inside each archive.

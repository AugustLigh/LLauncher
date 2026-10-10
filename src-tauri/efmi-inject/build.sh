#!/bin/sh
set -eu
cd "$(dirname "$0")"
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
printf 'LIBRARY kernel32.dll\nEXPORTS\nCreateToolhelp32Snapshot\nProcess32FirstW\nProcess32NextW\nCloseHandle\nSleep\nLoadLibraryW\nGetProcAddress\nGetCommandLineW\nGetStdHandle\nWriteFile\nExitProcess\n' >"$tmp/kernel32.def"
printf 'LIBRARY shell32.dll\nEXPORTS\nCommandLineToArgvW\n' >"$tmp/shell32.def"
llvm-dlltool -m i386:x86-64 -d "$tmp/kernel32.def" -l "$tmp/kernel32.lib"
llvm-dlltool -m i386:x86-64 -d "$tmp/shell32.def" -l "$tmp/shell32.lib"
clang --target=x86_64-pc-windows-msvc -O2 -ffreestanding -fno-stack-protector \
    -mno-stack-arg-probe -c inject.c -o "$tmp/inject.obj"
lld-link /nologo /subsystem:console /entry:start /nodefaultlib /Brepro \
    /out:efmi-inject.exe "$tmp/inject.obj" "$tmp/kernel32.lib" "$tmp/shell32.lib"

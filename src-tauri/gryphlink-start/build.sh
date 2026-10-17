#!/bin/sh
set -eu
cd "$(dirname "$0")"
x86_64-w64-mingw32-gcc -O2 -ffreestanding -fno-stack-protector -fno-asynchronous-unwind-tables \
    -nostdlib -e start -Wl,--subsystem,windows -Wl,--no-insert-timestamp -s \
    -o gryphlink-start.exe start.c -lkernel32 -lshell32

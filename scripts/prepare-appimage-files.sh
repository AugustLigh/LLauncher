#!/usr/bin/env bash
# Copies the GIO TLS module (from glib-networking) into a stable path so the
# Tauri AppImage bundler can include it (see bundle.linux.appimage.files in
# tauri.conf.json). Without this module bundled, the AppImage shows a black
# screen on systems where glib-networking is not installed: WebKit falls back
# to GDummyTlsBackend and every HTTPS request fails silently (issue #11).
set -euo pipefail

dest="$(cd "$(dirname "$0")/.." && pwd)/src-tauri/bundle-extra"
mkdir -p "$dest"

for dir in \
    /usr/lib/x86_64-linux-gnu/gio/modules \
    /usr/lib64/gio/modules \
    /usr/lib/gio/modules; do
    if [ -f "$dir/libgiognutls.so" ]; then
        cp "$dir/libgiognutls.so" "$dest/libgiognutls.so"
        echo "prepare-appimage-files: bundled $dir/libgiognutls.so"
        exit 0
    fi
done

echo "prepare-appimage-files: error: libgiognutls.so not found." >&2
echo "Install glib-networking (e.g. 'apt install glib-networking' or 'pacman -S glib-networking') and retry." >&2
exit 1

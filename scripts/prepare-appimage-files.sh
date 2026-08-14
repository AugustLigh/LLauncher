#!/usr/bin/env bash
# Copies runtime modules into stable paths so the Tauri AppImage bundler can
# include them (see bundle.linux.appimage.files in tauri.conf.json).
#
# Without the GIO TLS module bundled, the AppImage shows a black screen on
# systems where glib-networking is not installed: WebKit falls back to
# GDummyTlsBackend and every HTTPS request fails silently (issue #11).
#
# WebKitGTK also loads GStreamer plugins dynamically. Bundling the local plugin
# set avoids launching against a partial or incompatible host GStreamer stack on
# SteamOS and other immutable-ish systems (issue #28).
set -euo pipefail

dest="$(cd "$(dirname "$0")/.." && pwd)/src-tauri/bundle-extra"
mkdir -p "$dest"

bundle_gio_tls_module() {
for dir in \
    /usr/lib/x86_64-linux-gnu/gio/modules \
    /usr/lib64/gio/modules \
    /usr/lib/gio/modules; do
    if [ -f "$dir/libgiognutls.so" ]; then
        cp "$dir/libgiognutls.so" "$dest/libgiognutls.so"
        echo "prepare-appimage-files: bundled $dir/libgiognutls.so"
        return 0
    fi
done

echo "prepare-appimage-files: error: libgiognutls.so not found." >&2
echo "Install glib-networking (e.g. 'apt install glib-networking' or 'pacman -S glib-networking') and retry." >&2
exit 1
}

bundle_gstreamer_plugins() {
    local plugin_dest="$dest/gstreamer-1.0"

    for dir in \
        /usr/lib/x86_64-linux-gnu/gstreamer-1.0 \
        /usr/lib64/gstreamer-1.0 \
        /usr/lib/gstreamer-1.0; do
        if [ -d "$dir" ]; then
            rm -rf "$plugin_dest"
            mkdir -p "$plugin_dest"
            cp -a "$dir"/. "$plugin_dest"/
            echo "prepare-appimage-files: bundled GStreamer plugins from $dir"
            return 0
        fi
    done

    echo "prepare-appimage-files: error: GStreamer plugins directory not found." >&2
    echo "Install GStreamer plugins (e.g. 'apt install gstreamer1.0-plugins-base gstreamer1.0-plugins-good' or 'pacman -S gst-plugins-base gst-plugins-good') and retry." >&2
    exit 1
}

bundle_gio_tls_module
bundle_gstreamer_plugins

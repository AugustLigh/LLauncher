#!/usr/bin/env bash
# Build the Wine modules that let Arknights: Endfield run on macOS, for one
# Wine version, from the same sources the official WineHQ macOS package of
# that version is built from (Wine + the full wine-staging set), plus the
# patches in ./patches. Only four modules come out of the build:
#
#   lib/wine/x86_64-unix/ntdll.so        Rosetta fixes, NtDelayExecution
#   lib/wine/x86_64-windows/ntdll.dll    (unchanged source; keeps the pair together)
#   lib/wine/x86_64-windows/kernel32.dll KiUser*Dispatcher int3 stubs
#   lib/wine/x86_64-windows/ntoskrnl.exe the kernel exports the anti-cheat calls
#
# The launcher downloads the WineHQ bundle and drops these four over the
# originals, so they must match that bundle bit for bit in everything but the
# patches: same sources, same configure flags, same x86-64 target. The script
# checks that against the real package before packaging (exported symbols,
# rpaths, and a wineboot of a prefix with the modules swapped in).
#
#   ./build-modules.sh 11.16      # -> out/endfield-wine-modules-11.16-macos.tar.xz
#
# Runs on macOS only (Apple silicon or Intel), with Xcode Command Line Tools
# and Homebrew; installs bison, mingw-w64 and autoconf itself. Apple silicon
# needs Rosetta 2: the tree is built as x86-64, the way the WineHQ package is,
# and the build tools it produces run through Rosetta.
#
# Env: WORK (scratch, default ./work), OUT (default ./out), JOBS,
#      SKIP_DEPS=1, SKIP_SMOKE=1 (no download of the WineHQ package, no checks)
set -euo pipefail

VERSION="${1:?usage: build-modules.sh <wine-version>   e.g. 11.16}"
HERE="$(cd "$(dirname "$0")" && pwd)"
WORK="${WORK:-$HERE/work}"
OUT="${OUT:-$HERE/out}"
JOBS="${JOBS:-$(sysctl -n hw.ncpu)}"
NAME="endfield-wine-modules-${VERSION}-macos"
WINEHQ_URL="https://github.com/Gcenx/macOS_Wine_builds/releases/download/${VERSION}/wine-staging-${VERSION}-osx64.tar.xz"
MODULES="lib/wine/x86_64-unix/ntdll.so lib/wine/x86_64-windows/ntdll.dll lib/wine/x86_64-windows/kernel32.dll lib/wine/x86_64-windows/ntoskrnl.exe"

log() { printf '\n\033[1m==> %s\033[0m\n' "$*"; }
die() { echo "error: $*" >&2; exit 1; }

[ "$(uname -s)" = Darwin ] || die "this builds Mach-O modules and only runs on macOS"

# ---------------------------------------------------------------- 1. tools
if [ "${SKIP_DEPS:-0}" != 1 ]; then
  log "Homebrew dependencies (bison >= 3, mingw-w64, autoconf)"
  brew list --versions bison mingw-w64 autoconf >/dev/null 2>&1 || brew install bison mingw-w64 autoconf
fi
export PATH="$(brew --prefix)/opt/bison/bin:$PATH"
command -v x86_64-w64-mingw32-gcc >/dev/null || die "x86_64-w64-mingw32-gcc not found (brew install mingw-w64)"
command -v autoreconf >/dev/null || die "autoreconf not found (brew install autoconf)"
bison --version | head -1
RUN=""
if [ "$(uname -m)" = arm64 ]; then
  arch -x86_64 /usr/bin/true 2>/dev/null || die "Rosetta 2 is required: softwareupdate --install-rosetta --agree-to-license"
  RUN="arch -x86_64"
fi

# ---------------------------------------------------------------- 2. sources
mkdir -p "$WORK" "$OUT"
SRC="$WORK/wine-$VERSION"
STAGING="$WORK/wine-staging-$VERSION"
if [ ! -d "$SRC/.git" ]; then
  log "Fetching Wine $VERSION and wine-staging $VERSION"
  git clone -q --depth 1 --branch "wine-$VERSION" https://github.com/wine-mirror/wine "$SRC"
  git clone -q --depth 1 --branch "v$VERSION" https://github.com/wine-staging/wine-staging "$STAGING"
fi
cd "$SRC"
if ! git rev-parse -q --verify refs/tags/staging-applied >/dev/null 2>&1; then
  log "Applying the wine-staging patch set"
  git checkout -q -- . && git clean -qfdx
  "$STAGING/staging/patchinstall.py" --all >"$WORK/staging.log" 2>&1 \
    || { tail -30 "$WORK/staging.log"; die "wine-staging failed to apply"; }
  git add -A
  git -c user.email=build@llauncher -c user.name=build commit -qm "wine-staging $VERSION"
  git tag staging-applied
fi
log "Applying the Endfield patches"
git checkout -q -- . && git clean -qfd
for p in "$HERE"/patches/*.patch; do
  git apply "$p" || die "$(basename "$p") does not apply to Wine $VERSION + staging"
  echo "  $(basename "$p")"
done

# ---------------------------------------------------------------- 3. build
# The WineHQ package is built for x86-64 with i386 PE modules alongside
# (new WoW64), targeting macOS 10.15; matching that keeps ntdll.so's
# exports and load commands identical. Every optional dependency is off —
# none of the four modules uses one — so no x86-64 libraries are needed.
BUILD="$WORK/build-$VERSION"
rm -rf "$BUILD" && mkdir -p "$BUILD"
export MACOSX_DEPLOYMENT_TARGET=10.15
log "Configuring (x86-64)"
( cd "$BUILD" && $RUN "$SRC/configure" --enable-archs=i386,x86_64 --disable-tests \
    --without-x --without-freetype --without-gnutls --without-sdl --without-vulkan \
    --without-opengl --without-coreaudio --without-krb5 --without-gstreamer \
    --without-gphoto --without-sane --without-pcap --without-usb --without-cups \
    --without-openal --without-inotify --without-netapi --without-unwind \
    --without-capi --without-opencl --without-pcsclite --without-ffmpeg \
    --without-gettext >"$WORK/configure.log" 2>&1 ) \
  || { tail -40 "$WORK/configure.log"; die "configure failed"; }
grep -E '^checking (host system type|for x86_64-w64-mingw32-gcc)' "$WORK/configure.log" || true
log "Building the four modules (-j$JOBS)"
( cd "$BUILD" && $RUN make -j"$JOBS" \
    dlls/ntdll/ntdll.so dlls/ntdll/x86_64-windows/ntdll.dll \
    dlls/kernel32/x86_64-windows/kernel32.dll \
    dlls/ntoskrnl.exe/x86_64-windows/ntoskrnl.exe >"$WORK/build.log" 2>&1 ) \
  || { grep -n -B2 -A10 -E 'error[: ]' "$WORK/build.log" | tail -80; die "build failed"; }

# ---------------------------------------------------------------- 4. stage
STAGE="$WORK/stage/$NAME"
rm -rf "$WORK/stage" && mkdir -p "$STAGE/lib/wine/x86_64-unix" "$STAGE/lib/wine/x86_64-windows" "$STAGE/patches"
cp "$BUILD/dlls/ntdll/ntdll.so" "$STAGE/lib/wine/x86_64-unix/"
cp "$BUILD/dlls/ntdll/x86_64-windows/ntdll.dll" \
   "$BUILD/dlls/kernel32/x86_64-windows/kernel32.dll" \
   "$BUILD/dlls/ntoskrnl.exe/x86_64-windows/ntoskrnl.exe" "$STAGE/lib/wine/x86_64-windows/"
OUR_NTDLL="$STAGE/lib/wine/x86_64-unix/ntdll.so"
rpaths() { otool -l "$1" | awk '/LC_RPATH/{f=1} f&&/path/{print $2; f=0}'; }
exports() { nm -g --defined-only "$1" | awk '{print $3}' | grep -v '^$' | sort -u; }
cp "$HERE"/patches/*.patch "$HERE/patches/README.md" "$STAGE/patches/"
file "$STAGE"/lib/wine/*/* | sed 's|.*/stage/||'

# ---------------------------------------------------------------- 5. verify
if [ "${SKIP_SMOKE:-0}" != 1 ]; then
  log "Checking against the WineHQ package $VERSION"
  PKG="$WORK/winehq-$VERSION"
  if [ ! -d "$PKG/Wine Staging.app" ]; then
    mkdir -p "$PKG"
    curl -fsSL --retry 3 -o "$PKG/wine.tar.xz" "$WINEHQ_URL"
    tar -xJf "$PKG/wine.tar.xz" -C "$PKG"
  fi
  WINE_RES="$PKG/Wine Staging.app/Contents/Resources/wine"
  REF_NTDLL="$WINE_RES/lib/wine/x86_64-unix/ntdll.so"
  [ -x "$WINE_RES/bin/wine" ] || die "no wine loader in the WineHQ package"
  # dyld resolves the @rpath dependencies of a library that ntdll.so dlopens
  # (say, libgnutls for bcrypt.so) through ntdll.so's own run-path list as
  # well, so ours has to carry the same entries as the package's. WineHQ's
  # build adds `@loader_path/../../` (the bundle's lib/) on top of what
  # Wine's configure sets; take over whatever the reference has.
  while read -r rp; do
    rpaths "$OUR_NTDLL" | grep -qxF -- "$rp" || { install_name_tool -add_rpath "$rp" "$OUR_NTDLL"; echo "  added rpath $rp"; }
  done < <(rpaths "$REF_NTDLL")
  # The other unix libraries link against ntdll.so's exports: the set must
  # be the same as in the package, or the sources/flags are out of sync.
  if ! diff <(exports "$REF_NTDLL") <(exports "$OUR_NTDLL") >"$WORK/exports.diff"; then
    cat "$WORK/exports.diff"; die "ntdll.so exports differ from the WineHQ build"
  fi
  if ! diff <(rpaths "$REF_NTDLL" | sort) <(rpaths "$OUR_NTDLL" | sort); then
    die "ntdll.so rpaths differ from the WineHQ build"
  fi
  echo "  exports and rpaths match the WineHQ ntdll.so"
fi
# Ad-hoc signature, as every Mach-O in the WineHQ bundle carries one — after
# any load-command edit, which would have invalidated an earlier one.
codesign --force --sign - "$OUR_NTDLL"
if [ "${SKIP_SMOKE:-0}" != 1 ]; then
  # Swap the modules into a copy of the package and boot a prefix with it.
  TEST="$WORK/smoke"; rm -rf "$TEST"; mkdir -p "$TEST"
  cp -a "$PKG/Wine Staging.app" "$TEST/"
  for m in $MODULES; do cp "$STAGE/$m" "$TEST/Wine Staging.app/Contents/Resources/wine/$m"; done
  export WINEPREFIX="$TEST/prefix" WINEDEBUG=-all
  W="$TEST/Wine Staging.app/Contents/Resources/wine/bin"
  "$W/wine" wineboot -u >"$WORK/wineboot.log" 2>&1 || { tail -20 "$WORK/wineboot.log"; die "wineboot failed with the patched modules"; }
  "$W/wineserver" -w
  [ -f "$WINEPREFIX/drive_c/windows/system32/kernel32.dll" ] || die "wineboot did not populate the prefix"
  "$W/wine" cmd /c ver 2>/dev/null | grep -qi windows || die "cmd /c ver failed under the patched modules"
  "$W/wineserver" -k || true
  echo "  wineboot + cmd ran with the patched modules"
fi

# ---------------------------------------------------------------- 6. package
log "Packaging"
# Written last: the hashes have to describe the signed, rpath-adjusted files.
{
  echo "Endfield Wine modules for macOS — Wine $VERSION"
  echo "Built $(date -u +%Y-%m-%dT%H:%M:%SZ) on $(sw_vers -productVersion) $(uname -m), $(clang --version | head -1)"
  echo "Sources: wine tag wine-$VERSION + wine-staging tag v$VERSION (https://gitlab.winehq.org/wine), patches/ (LGPL-2.1-or-later)"
  echo "Target: x86-64, macOS >= $MACOSX_DEPLOYMENT_TARGET, for the WineHQ package $WINEHQ_URL"
  echo "Modules:"; for m in $MODULES; do echo "  $m  $(shasum -a 256 "$STAGE/$m" | cut -c1-64)"; done
} > "$STAGE/MODULES.txt"
cat "$STAGE/MODULES.txt"
( cd "$WORK/stage" && COPYFILE_DISABLE=1 tar -cJf "$OUT/$NAME.tar.xz" --exclude='.DS_Store' "$NAME" )
( cd "$OUT" && shasum -a 256 "$NAME.tar.xz" > "$NAME.tar.xz.sha256" )
ls -la "$OUT"
echo "done: $OUT/$NAME.tar.xz"

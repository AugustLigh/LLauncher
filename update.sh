#!/usr/bin/env bash
# Release helper: bumps the version everywhere, builds all bundles, commits,
# tags and publishes a GitHub release. The AUR package is updated automatically
# by the aur-publish workflow when the release is published.
#
# Usage:
#   ./update.sh 0.2.2
set -euo pipefail

cd "$(dirname "$0")"

VERSION="${1:-}"
if [[ ! "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo "Usage: ./update.sh <version>   (e.g. ./update.sh 0.2.2)" >&2
    exit 1
fi

if [ -n "$(git status --porcelain)" ]; then
    echo "error: working tree is not clean — commit or stash your changes first." >&2
    git status --short
    exit 1
fi

command -v gh >/dev/null || { echo "error: GitHub CLI (gh) is required." >&2; exit 1; }

CURRENT="$(node -p "require('./package.json').version")"
if [ "$CURRENT" = "$VERSION" ]; then
    echo "==> Version is already $VERSION, skipping bump"
else
    echo "==> Bumping version $CURRENT -> $VERSION"
    npm version "$VERSION" --no-git-tag-version >/dev/null
    sed -i "0,/\"version\": \".*\"/s//\"version\": \"$VERSION\"/" src-tauri/tauri.conf.json
    sed -i "0,/^version = \".*\"/s//version = \"$VERSION\"/" src-tauri/Cargo.toml
fi

echo "==> Building bundles (this takes a while)"
npx tauri build

APPIMAGE="src-tauri/target/release/bundle/appimage/LLauncher_${VERSION}_amd64.AppImage"
DEB="src-tauri/target/release/bundle/deb/LLauncher_${VERSION}_amd64.deb"
RPM="src-tauri/target/release/bundle/rpm/LLauncher-${VERSION}-1.x86_64.rpm"
for f in "$APPIMAGE" "$DEB" "$RPM"; do
    [ -f "$f" ] || { echo "error: expected bundle not found: $f" >&2; exit 1; }
done

echo "==> Committing and tagging $VERSION"
git add package.json src-tauri/tauri.conf.json src-tauri/Cargo.toml src-tauri/Cargo.lock
if ! git diff --cached --quiet; then
    git commit -m "Release v$VERSION"
fi
git tag "$VERSION"
git push origin HEAD "$VERSION"

echo "==> Creating GitHub release"
gh release create "$VERSION" "$APPIMAGE" "$DEB" "$RPM" \
    --title "LLauncher $VERSION" \
    --generate-notes

echo "==> Done. The aur-publish workflow will update the AUR package automatically."

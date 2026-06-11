#!/usr/bin/env bash
# Release helper.
#
# Default flow: bumps the version, commits, tags and pushes. GitHub Actions
# (.github/workflows/release.yml) then builds the bundles on Ubuntu 22.04 LTS
# (better glibc compatibility than a local build), publishes the GitHub
# release and updates the AUR package.
#
#   ./update.sh 0.2.3
#
# Fallback: build and publish everything locally instead of CI:
#
#   ./update.sh 0.2.3 --local
set -euo pipefail

cd "$(dirname "$0")"

VERSION="${1:-}"
MODE="${2:-ci}"
if [[ ! "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo "Usage: ./update.sh <version> [--local]   (e.g. ./update.sh 0.2.3)" >&2
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
    # Refresh Cargo.lock with the new version
    (cd src-tauri && cargo check -q)
fi

echo "==> Committing and tagging $VERSION"
git add package.json src-tauri/tauri.conf.json src-tauri/Cargo.toml src-tauri/Cargo.lock
if ! git diff --cached --quiet; then
    git commit -m "Release v$VERSION"
fi
git tag "$VERSION"

if [ "$MODE" != "--local" ]; then
    git push origin HEAD "$VERSION"
    echo "==> Starting the CI release build"
    gh workflow run release.yml -f tag="$VERSION"
    echo "==> Done. CI is building the release now:"
    echo "    https://github.com/AugustLigh/LLauncher/actions"
    echo "    The GitHub release and the AUR package will be published automatically."
    exit 0
fi

echo "==> Building bundles locally (this takes a while)"
npx tauri build

APPIMAGE="src-tauri/target/release/bundle/appimage/LLauncher_${VERSION}_amd64.AppImage"
DEB="src-tauri/target/release/bundle/deb/LLauncher_${VERSION}_amd64.deb"
RPM="src-tauri/target/release/bundle/rpm/LLauncher-${VERSION}-1.x86_64.rpm"
for f in "$APPIMAGE" "$DEB" "$RPM"; do
    [ -f "$f" ] || { echo "error: expected bundle not found: $f" >&2; exit 1; }
done

git push origin HEAD "$VERSION"

echo "==> Creating GitHub release"
gh release create "$VERSION" "$APPIMAGE" "$DEB" "$RPM" \
    --title "LLauncher $VERSION" \
    --generate-notes

echo "==> Done. The aur-publish workflow will update the AUR package automatically."

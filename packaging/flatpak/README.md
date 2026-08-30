# Flatpak packaging

Files for building LLauncher as a Flatpak and submitting it to Flathub.

## Layout

- `io.github.augustligh.LLauncher.yml` — the flatpak-builder manifest. Builds
  the app from the git tag pinned in its `sources` section, fully offline as
  Flathub requires.
- `cargo-sources.json` / `node-sources.json` — generated offline mirrors of
  every Rust crate and npm package. **Regenerate on every dependency change**
  (see below), or the Flathub build will fail.
- `io.github.augustligh.LLauncher.desktop` — desktop entry (the flatpak does
  not use the handlebars template in `src-tauri/`).
- `io.github.augustligh.LLauncher.metainfo.xml` — AppStream metadata shown on
  Flathub. Add a `<release>` entry for every new version.

## Building locally

```bash
# one-time setup
flatpak remote-add --user --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo
flatpak install --user flathub org.flatpak.Builder

# build + install into the user installation
flatpak run org.flatpak.Builder --user --install --force-clean --install-deps-from=flathub \
    build-dir packaging/flatpak/io.github.augustligh.LLauncher.yml

flatpak run io.github.augustligh.LLauncher
```

The manifest builds the **git tag** it pins, not the working tree. To test
uncommitted changes, temporarily replace the `type: git` source of the
`llauncher` module with `type: dir, path: ../..`.

## Regenerating the offline sources

After changing `Cargo.lock` or `yarn.lock`:

```bash
python3 -m venv /tmp/fpvenv && /tmp/fpvenv/bin/pip install aiohttp tomlkit
git clone https://github.com/flatpak/flatpak-builder-tools /tmp/fbt
/tmp/fpvenv/bin/pip install /tmp/fbt/node

/tmp/fpvenv/bin/python /tmp/fbt/cargo/flatpak-cargo-generator.py \
    src-tauri/Cargo.lock -o packaging/flatpak/cargo-sources.json
/tmp/fpvenv/bin/flatpak-node-generator yarn yarn.lock \
    -o packaging/flatpak/node-sources.json
```

## Releasing a new version

1. Bump the `tag:` in the manifest's `llauncher` module.
2. Regenerate the source manifests if the lockfiles changed.
3. Add a `<release>` entry to the metainfo.
4. After Flathub acceptance, updates are PRs against the app's repo under
   `github.com/flathub/io.github.augustligh.LLauncher` (the Flathub bot can
   open them automatically when a new GitHub release appears).

## Submitting to Flathub (first time)

1. Validate: `flatpak run --command=flatpak-builder-lint org.flatpak.Builder appstream packaging/flatpak/io.github.augustligh.LLauncher.metainfo.xml`
   and `... manifest packaging/flatpak/io.github.augustligh.LLauncher.yml`.
2. Fork `github.com/flathub/flathub`, create a branch **from `new-pr`**, put
   the contents of this directory at the repository root, and open a PR
   against the `new-pr` branch.
3. After review and merge, verify the app at
   `https://flathub.org/apps/io.github.augustligh.LLauncher` via the GitHub
   verification flow to get the "verified" badge.

## Known linter findings

`flatpak-builder-lint` flags `--filesystem=home` as an error. Game launchers
with user-chosen, multi-gigabyte install locations (Lutris, Heroic, …)
routinely get a reviewer-granted exception for this — mention the use case in
the submission PR. The long-term alternative is moving folder selection fully
onto the file-chooser portal and dropping the permission.

## Sandbox notes

- Wine/DWProton runs **inside** the sandbox; 32-bit support comes from the
  `org.freedesktop.Platform.Compat.i386` and `GL32` extensions
  (`--allow=multiarch`), the same way Bottles does it.
- `gamemode` and `gamescope` from the host are not visible in the sandbox;
  the system check will report them as missing. MangoHud can be provided
  later via the `org.freedesktop.Platform.VulkanLayer.MangoHud` extension.
- Game data lives in the sandboxed XDG dirs
  (`~/.var/app/io.github.augustligh.LLauncher/`); the game install folder is
  user-chosen, hence `--filesystem=home`.

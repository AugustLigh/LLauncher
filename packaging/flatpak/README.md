# Flatpak packaging

Files for building LLauncher as a Flatpak and publishing it to the project's
own Flatpak repository.

Flathub is not an option: its
[generative AI policy](https://docs.flathub.org/docs/for-app-authors/requirements#generative-ai-policy)
disallows applications containing AI-assisted code. Updates for Flatpak users
are served from `https://augustligh.github.io/LLauncher/` instead.

## Layout

- `io.github.augustligh.LLauncher.yml` — the flatpak-builder manifest. Builds
  the app from the git tag pinned in its `sources` section, fully offline.
- `cargo-sources.json` / `node-sources.json` — generated offline mirrors of
  every Rust crate and npm package. **Regenerate on every dependency change**
  (see below), or the offline build will fail.
- `io.github.augustligh.LLauncher.desktop` — desktop entry (the flatpak does
  not use the handlebars template in `src-tauri/`).
- `io.github.augustligh.LLauncher.metainfo.xml` — AppStream metadata, served
  from the repository's appstream branch. Add a `<release>` entry for every
  new version.

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

Publishing itself is automatic: `release.yml` chains `flatpak.yml`, which
builds the bundle from the tag, attaches it to the GitHub release, and imports
it into the repository served from the `gh-pages` branch.

## The self-hosted repository

`flatpak.yml`'s `publish` job holds the whole mechanism:

- The `gh-pages` branch **is** the ostree repository — GitHub Pages serves it
  at `https://augustligh.github.io/LLauncher/`. Each run fetches the branch,
  imports the freshly built bundle with `flatpak build-import-bundle`, and
  refreshes the summary and static deltas with `flatpak build-update-repo`.
- The branch is force-pushed as a single root commit every time, so its git
  history never accumulates the binary objects of past releases. The published
  tree stays around 7 MB.
- `--prune` drops the previous version. Bundles imported from a `.flatpak`
  file carry parentless commits, so the old commit becomes unreachable the
  moment the ref moves. Older versions remain downloadable as bundles from
  their GitHub releases.
- git does not track empty directories, so the job recreates the ones ostree
  expects (`refs/remotes`, `tmp`, `state`, …) after fetching the branch.
  Without that, `build-update-repo` fails with `opendir(refs/remotes)`.

### Signing

The repository is signed with a dedicated GPG key whose private half lives in
the `FLATPAK_GPG_PRIVATE_KEY` repository secret; the public half is written
into `llauncher.flatpakrepo` as `GPGKey=` on every publish. Clients pin that
key when they run `flatpak remote-add`, so an unsigned or differently signed
commit is refused with "ref does not exist in remote" rather than installed.

The job fails outright when the secret is missing, precisely so that a
publish never silently degrades into an unsigned one that existing installs
would reject anyway.

Rotating or losing the key means every user has to remove and re-add the
remote. Keep the backup of the private key somewhere safe.

## Installing from the repository

```bash
flatpak remote-add --if-not-exists --user llauncher \
    https://augustligh.github.io/LLauncher/llauncher.flatpakrepo
flatpak install --user llauncher io.github.augustligh.LLauncher
flatpak update --user io.github.augustligh.LLauncher
```

## Known linter findings

`flatpak-builder-lint` flags `--filesystem=home` as an error. It is kept
deliberately: the game install location is user-chosen and routinely lives on
another drive. The long-term alternative is moving folder selection fully onto
the file-chooser portal and dropping the permission.

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

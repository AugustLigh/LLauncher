<div align="center">

# 🚀 LLauncher

**A native Linux launcher for Arknights: Endfield — with Windows and macOS builds too**

Built with Tauri v2, React, and Rust

[![Tauri](https://img.shields.io/badge/Tauri-v2-FFC131?logo=tauri&logoColor=white)](https://v2.tauri.app)
[![React](https://img.shields.io/badge/React-18-61DAFB?logo=react&logoColor=white)](https://react.dev)
[![Rust](https://img.shields.io/badge/Rust-2021-DEA584?logo=rust&logoColor=white)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE)

</div>

---

<img width="1280" height="719" alt="изображение" src="https://github.com/user-attachments/assets/181529d5-23b0-463a-b1a1-c5d13e9a4475" />


LLauncher is a lightweight, native Linux launcher for **Arknights: Endfield**. It handles game installation, updates, and launching through Proton — no Steam or Lutris required.

The same launcher builds for Windows, where the game needs no compatibility
layer: the install, update and verification machinery is identical and the
Proton-specific settings simply disappear.

It also builds for macOS (experimental — see [macOS](#macos) below), where it
assembles its own open-source compatibility layer the way it downloads
DWProton on Linux: the official WineHQ Wine Staging package, the same
anti-cheat patches dw-proton carries plus two Rosetta 2 fixes, and DXMT, a
Direct3D 11 → Metal translator.

[Download the latest release](https://github.com/AugustLigh/LLauncher/releases/latest) (AppImage / .deb / .rpm / .flatpak, an .exe installer and .msi for Windows, a universal .dmg for macOS)

On Arch Linux (and derivatives like CachyOS, Manjaro, EndeavourOS) install from the AUR — dependencies are handled automatically:

```bash
yay -S llauncher-bin   # or: paru -S llauncher-bin
```

For Flatpak, add the launcher's own repository once and get updates through
`flatpak update` like any other Flatpak app:

```bash
flatpak remote-add --if-not-exists --user llauncher \
    https://augustligh.github.io/LLauncher/llauncher.flatpakrepo
flatpak install --user llauncher io.github.augustligh.LLauncher
```

The repository is signed, and its public key is pinned when the remote is
added. The `.flatpak` bundle on each release stays available for a one-off
install without a remote.

## Features

- **One-click install & launch** — download, verify, extract, and play
- **Auto-updates** — detects new game versions and patches seamlessly
- **Proton management** — download and manage DWProton versions directly from the launcher
- **macOS without CrossOver** — downloads Wine Staging, its own patched Wine modules (the dw-proton anti-cheat fixes and the Rosetta 2 fixes, built in CI) and DXMT — all open source — installs Rosetta 2 on the way, and runs the game with no paid or proprietary layer
- **Multi-threaded downloads** — up to 8 concurrent connections with per-worker speed limiting
- **Download-friendly power handling** — keeps the system from suspending while a transfer runs, and a "Turn off screen" button saves battery during long downloads (handy on the Steam Deck)
- **File verification** — MD5 checksum validation for every downloaded file, with smart skip for already verified files
- **System tray** — minimize to tray, launch from tray
- **In-app news** — announcements and updates from the official API
- **Gamescope integration** — run the game in Valve's micro-compositor with FSR/NIS upscaling, FPS cap, HDR and window-mode control
- **Prefix toolbox** — open the Wine prefix, run winecfg, clear shader caches, back up / restore / reset the prefix from Settings
- **Mod support** — install EFMI (Endfield Model Importer) from the launcher and start the game with mods as a separate action, leaving the normal launch on the native Vulkan renderer; vkBasalt and ReShade add-ons for graphics mods
- **Controller support** — gamepads are handed to the game as XInput controllers, so a DualShock/DualSense works without Steam Input
- **Play statistics** — total playtime and the last launch, with a session journal stored locally
- **Quick launch** — `llauncher --play` and a desktop-menu "Launch Arknights: Endfield" action start the game straight from your app menu
- **Configurable launch options** — Gamemode, MangoHUD, DXVK Async, Wayland, custom env vars and arguments
- **System checks** — warns about missing dependencies (Proton, ntsync)
- **Custom UI** — Endfield-inspired artwork and dark layout, a compact launch menu, integrated download progress, and dedicated settings pages

## Prerequisites

Common to both platforms:

- **Node.js** >= 18 and **npm** (or yarn)
- **Rust** toolchain ([rustup](https://rustup.rs))

Linux (x86_64):

- **System libraries** for Tauri v2 — see the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/#linux)
- **glib-networking** — its GIO TLS module is bundled into the AppImage at build time
- **GStreamer plugins** — the build copies the local `gstreamer-1.0` plugin directory into the AppImage for WebKitGTK
- A **Proton** build (DWProton can be downloaded from within the launcher)

Windows (x86_64):

- **Microsoft Visual C++ Build Tools** and the **Windows SDK** — see the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/#windows)
- **WebView2** — preinstalled on Windows 11 and current Windows 10; the installer fetches it otherwise

macOS (Apple silicon or Intel):

- **Xcode Command Line Tools** — see the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/#macos)
- Nothing else at build time; at run time the launcher downloads Wine Staging (and DXMT on Apple silicon) itself, and asks for an administrator password once to install Rosetta 2 if it is missing

## Getting Started

```bash
# Clone the repository
git clone https://github.com/your-username/LLauncher.git
cd LLauncher

# Install frontend dependencies
npm install

# Run in development mode
npx tauri dev
```

## Building

For a local Linux executable without installer packages:

```bash
npm run tauri -- build --no-bundle --config '{"build":{"beforeBuildCommand":"npm run build"}}'
./src-tauri/target/release/llauncher
```

Quit an older running instance from the tray before launching the new build.

```bash
./build.sh
```

Release bundles will be created in `src-tauri/target/release/bundle/`:

| Format   | Path                                         |
| -------- | -------------------------------------------- |
| AppImage | `bundle/appimage/LLauncher_0.1.0_amd64.AppImage` |
| .deb     | `bundle/deb/LLauncher_0.1.0_amd64.deb`           |
| .rpm     | `bundle/rpm/LLauncher-0.1.0-1.x86_64.rpm`        |

AppImage builds embed the build host's GIO TLS module and GStreamer plugin
directory via `scripts/prepare-appimage-files.sh`. To refresh those embedded
libraries after a WebKitGTK/GStreamer update, update the distro packages on the
build host and run `./build.sh` again.

On Windows, run the build from a Windows host (the bundlers need it):

```powershell
npx tauri build
```

| Format | Path                                          |
| ------ | --------------------------------------------- |
| NSIS   | `bundle/nsis/LLauncher_0.3.3_x64-setup.exe`    |
| MSI    | `bundle/msi/LLauncher_0.3.3_x64_en-US.msi`     |

On macOS, likewise from a Mac (`--target universal-apple-darwin` for one DMG
that runs on both Apple silicon and Intel, which is what CI attaches to a
release):

```bash
rustup target add aarch64-apple-darwin x86_64-apple-darwin
npx tauri build --target universal-apple-darwin
```

| Format | Path                                                          |
| ------ | ------------------------------------------------------------- |
| DMG    | `universal-apple-darwin/release/bundle/dmg/LLauncher_0.3.3_universal.dmg` |

The platform-specific bundler settings live in `src-tauri/tauri.linux.conf.json`,
`src-tauri/tauri.windows.conf.json` and `src-tauri/tauri.macos.conf.json`; Tauri
merges the matching one over `tauri.conf.json` automatically.

## Configuration

Settings are stored in `~/.config/llauncher/settings.json` (`%APPDATA%\llauncher\settings.json` on Windows, `~/Library/Application Support/llauncher/settings.json` on macOS) and can be edited through the in-app settings panel.

| Category  | Options                                                            |
| --------- | ------------------------------------------------------------------ |
| General | Language, on-launch action, autostart, Discord presence |
| Game & files | Game and download folders, download limits, verification, repair |
| Launch | Proton versions, controller and FPS settings, advanced launch options |
| Mods | EFMI, mods folder and catalog, visual add-ons |
| Diagnostics | System checks, logs, debug information, prefix tools |

On Windows Proton management and every Proton-only launch option are hidden — the
game runs natively — and the Launch tab offers the Windows-only options instead:
the game's own Vulkan renderer (`-vulkan`, the path every Linux session uses; the
official launcher never enables it on Windows, so treat it as an experiment and
compare), "Prefer the dedicated GPU" and "Optimizations for windowed games"
(the launcher's entry for the game on the Windows graphics settings page, the
counterpart of PRIME offload on Linux), the High performance power plan for the
length of a session, above-normal process priority, and "Run as administrator"
for the rare case where the anti-cheat refuses to load without elevation.

On macOS the runtime picker lists Wine Staging releases instead of DWProton
ones — only those a patched module set has been published for — and the
Launch tab offers the macOS-only options (Vulkan renderer, Rosetta AVX, Metal
HUD) in place of the Linux wrappers. The prefix tools are the same.

<img width="1282" height="715" alt="изображение" src="https://github.com/user-attachments/assets/e9262948-b29a-4b93-bb2c-8e0438db8a6f" />


Default paths (Linux):

```
Game:   ~/Games/ArknightsEndfield
Proton: ~/.local/share/llauncher/proton
Config: ~/.config/llauncher/settings.json
Logs:   ~/.config/llauncher/launch.log
```

Default paths (Windows):

```
Game:   %USERPROFILE%\Games\ArknightsEndfield
Config: %APPDATA%\llauncher\settings.json
Logs:   %APPDATA%\llauncher\launch.log
```

Default paths (macOS):

```
Game:   ~/Games/ArknightsEndfield
Wine:   ~/Library/Application Support/llauncher/wine
Prefix: ~/Library/Application Support/llauncher/prefix
Config: ~/Library/Application Support/llauncher/settings.json
Logs:   ~/Library/Application Support/llauncher/launch.log
```

## Mods

Almost all of them run on [EFMI](https://github.com/SpectrumQT/EFMI-Package), the
Endfield Model Importer. EFMI is the Endfield-specific half of a loader: 3DMigoto
supplies the `d3d11.dll` proxy that intercepts draw calls, EFMI decides which models
to swap. Upstream installs the two through XXMI Launcher, a Windows GUI; this
launcher fetches both release archives itself, so no extra tool has to run inside
the Proton prefix. Settings → Mods installs it, opens the `Mods` folder and switches
on a second launch button. Then:

1. **Install the loader** — downloaded straight into the game directory.
2. **Drop mods into `Mods/`** — one directory per mod. [Catalogue](https://gamebanana.com/games/21842).
3. **Use "play with mods"** — the ordinary Play button is untouched.

Two caveats, both deliberate reasons the modded launch is a separate button rather
than a setting:

- **It costs frames on Linux.** The loader hooks DirectX 11 and has no Vulkan
  equivalent, so a modded session runs on the game's D3D11 path through DXVK instead
  of its native Vulkan renderer. On Windows the game runs on D3D11 by default (a
  modded launch ignores the Vulkan renderer toggle) and only the loader's own
  overhead applies.
- **The game ships the ACE anti-cheat.** No wave of bans over cosmetic mods has been
  documented, but nobody — the mod authors included — guarantees anything.

### Image and colour

The other family of mods — sharpening, colour grading, HDR — does not touch models
and does not need the D3D11 detour:

- **vkBasalt** is a Vulkan layer that runs ReShade-format effects on the game's
  *native* renderer, so it costs no frames at all. Install the package, then switch
  it on in Settings → Mods.
- **ReShade add-ons** such as [RenoDX](https://github.com/clshortfuse/renodx)
  (graphics overhaul, native HDR) rewrite the game's shaders and are DirectX-only,
  so they ride along with the modded launch. Put ReShade in the game folder as
  `dxgi.dll` — `d3d11.dll` belongs to the mod loader — and the launcher picks it up on its
  own; the modded launch already sets the override it needs.

Mods that patch the game itself rather than the renderer work on a normal Vulkan
launch and need none of this.

## macOS

Experimental: it builds and is tested in CI on an Apple silicon runner, and
the patched Wine modules boot a prefix there — but nobody on the project owns
a Mac, so nothing below has been tried against the game itself yet. Reports,
working or not, are welcome.

The idea is the same as on Linux: press **Launch**, and if nothing can run a
Windows executable yet the launcher offers to install it. What it fetches is
entirely open source, nothing from CodeWeavers or Apple:

- **Wine Staging** — the [official WineHQ package for macOS](https://github.com/Gcenx/macOS_Wine_builds)
  (a `Wine Staging.app` bundle, ~190 MB), with winevulkan and MoltenVK inside.
- **The Endfield modules** — four Wine modules (`ntdll.so`, `ntdll.dll`,
  `kernel32.dll`, `ntoskrnl.exe`) this project builds itself from the same
  Wine sources, with the anti-cheat patches from [dw-proton](https://dawn.wine/)
  (the kernel exports the ACE driver calls and Wine leaves as stubs, the
  `KiUser*Dispatcher` stubs for the protector, QPC-timed waits) and the two
  Rosetta 2 fixes from [Endfield_FineWine](https://github.com/stoicswe/Endfield_FineWine)
  (Rosetta faults on the multi-byte NOPs the protector emits, and reports the
  driver's `mov cr3` probe as the wrong exception — "driver error 13"). They
  replace the originals inside the bundle. Sources, patches and the build
  script are in [`macos/wine/`](macos/wine/); CI builds and publishes a set
  per Wine version as the `wine-modules-<version>` release, and the picker
  only offers Wine versions that have one.
- **DXMT** — [3Shain/dxmt](https://github.com/3Shain/dxmt), a Direct3D 11 → Metal
  translator, the open-source counterpart of the D3DMetal CrossOver ships.
  Installed into the Wine bundle as its builtin `d3d11`/`dxgi`, exactly as
  Heroic Games Launcher does. Apple silicon only; an Intel Mac gets bare Wine
  and should try the **Vulkan renderer** launch option instead.
- **Rosetta 2**, if the Mac is Apple silicon and does not have it: every Wine
  for macOS is x86-64. macOS asks for an administrator password once.

Everything lands under `~/Library/Application Support/llauncher/` — the Wine
builds in `wine/`, the prefix in `prefix/endfield/pfx`. Settings > Launch
lists the Wine builds the same way it lists DWProton on Linux; a build of
your own can be pointed at in the custom paths, but without the modules the
anti-cheat driver aborts on start, so that is for debugging.

The game is started with `-force-d3d11`: its default renderer is Vulkan (or
Direct3D 12), which does not draw a frame over MoltenVK, and DXMT only does
Direct3D 11. The **Vulkan renderer** toggle turns that off.

Endfield_FineWine proved this combination — dw-proton's patches, the Rosetta
fixes, Direct3D 11 — on CrossOver 26.2 with Apple's D3DMetal; this port
carries it over to WineHQ's build and DXMT, so the graphics half is the
untested one. The FineWine write-up is the place to start when something
does not work.

**Opening the launcher the first time.** The DMG is not signed (there is no
Apple developer account behind the project), so Gatekeeper refuses it with
"damaged" or "unidentified developer". Once:

```bash
xattr -dr com.apple.quarantine /Applications/LLauncher.app
```

or right-click the app → Open → Open.

**What to expect.** The first launch creates the prefix, which takes a minute
under Rosetta. The launch options for macOS are `ROSETTA_ADVERTISE_AVX`, the
Metal performance HUD and the `-vulkan` renderer switch. Things to look at
first in the launch log (Settings > Diagnostics) when it fails:

- An abort in `ntoskrnl.exe` or `ACE-BASE.sys` means the Endfield modules are
  not in the active Wine — Settings > Launch says whether they are.
- A white screen or a `d3d11` device-creation error is the renderer: check
  that DXMT is listed for the build and that the Vulkan toggle is off.
- DXMT is young. A rendering problem is far more likely to be DXMT than the
  game; the launch options let you switch the game to its Vulkan renderer to
  tell the two apart.

**Building the modules yourself** (on a Mac, with Xcode Command Line Tools and
Homebrew): `macos/wine/build-modules.sh 11.16` fetches Wine and wine-staging
at that version, applies the patches, builds the four modules, checks them
against the WineHQ package of the same version (identical exports and rpaths,
then a prefix booted with the modules swapped in) and packs
`endfield-wine-modules-11.16-macos.tar.xz`. `Wine modules (macOS)` in GitHub
Actions runs the same script and publishes the result.

## Troubleshooting

**Black screen / blank window on launch (AppImage)**

Recent AppImages bundle the GIO TLS module and disable the WebKit DMA-BUF renderer automatically. If you still hit a black screen (or are on an older release):

```bash
# Install glib-networking (Arch/CachyOS: pacman -S glib-networking)
GIO_MODULE_DIR=/usr/lib/gio/modules WEBKIT_DISABLE_COMPOSITING_MODE=1 ./LLauncher_*.AppImage
```

**`libayatana-appindicator is deprecated` warning**

Harmless — it comes from the system tray library and does not affect functionality.

**Game on an NTFS partition**

The Proton prefix is stored under `~/.local/share/llauncher/prefix/` (configurable via `proton_prefix_dir` in settings), so the game itself may live on NTFS. Note that running games from NTFS under Linux is generally discouraged — ext4/btrfs/ZFS are safer choices.

## Project Structure

```
LLauncher/
├── src/                        # React frontend
│   ├── components/
│   │   ├── layout/             #   TitleBar, MainLayout
│   │   ├── home/               #   HomePage, ActionButton, GameStatus, ProgressBar, ...
│   │   ├── settings/           #   SettingsModal, PathSelector, LanguageSelector
│   │   └── common/             #   SystemWarning, GlassCard, IconButton
│   ├── hooks/                  #   useDownload, useGameState, useSettings, ...
│   ├── styles/                 #   CSS variables, animations, global styles
│   └── utils/                  #   Formatting helpers
├── src-tauri/                  # Rust backend
│   └── src/
│       ├── api/                #   API client, types, constants
│       ├── config/             #   Settings persistence, path management
│       ├── download/           #   Download manager, workers, extraction, verification
│       ├── game/               #   Game state detection, launching (launcher/{linux,macos,windows}.rs), mods
│       ├── commands.rs         #   Tauri command handlers
│       └── lib.rs              #   App setup and plugin registration
├── package.json
└── vite.config.js
```

## Tech Stack

| Layer    | Technology                    |
| -------- | ----------------------------- |
| Frontend | React 18, Vite 6, CSS3       |
| Backend  | Rust, Tauri v2                |
| HTTP     | reqwest (async, streaming)    |
| Crypto   | md-5 (file verification)      |
| Runtime  | Tokio (async, multi-threaded) |

## Contributing

Contributions are welcome! Feel free to open issues and pull requests.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/my-feature`)
3. Commit your changes (`git commit -m 'Add my feature'`)
4. Push to the branch (`git push origin feature/my-feature`)
5. Open a Pull Request

## Disclaimer

This project is not affiliated with Gryphline, Hypergryph, or any of their subsidiaries. Arknights: Endfield is a trademark of Gryphline/Hypergryph. This is a community-made tool.

## License

This project is licensed under the [MIT License](LICENSE).

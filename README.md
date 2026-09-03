<div align="center">

# 🚀 LLauncher

**A native Linux launcher for Arknights: Endfield — with Windows builds too**

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

[Download the latest release](https://github.com/AugustLigh/LLauncher/releases/latest) (AppImage / .deb / .rpm / .flatpak, plus an .exe installer and .msi for Windows)

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
- **Multi-threaded downloads** — up to 8 concurrent connections with per-worker speed limiting
- **File verification** — MD5 checksum validation for every downloaded file, with smart skip for already verified files
- **System tray** — minimize to tray, launch from tray
- **In-app news** — announcements and updates from the official API
- **Gamescope integration** — run the game in Valve's micro-compositor with FSR/NIS upscaling, FPS cap, HDR and window-mode control
- **Prefix toolbox** — open the Wine prefix, run winecfg, clear shader caches, back up / restore / reset the prefix from Settings
- **Mod support** — install the 3DMigoto loader from the launcher and start the game with mods as a separate action, leaving the normal launch on the native Vulkan renderer; vkBasalt and ReShade add-ons for graphics mods
- **Play statistics** — session journal with weekly playtime, average session length and a 7-day activity chart
- **Quick launch** — `llauncher --play` and a desktop-menu "Launch Arknights: Endfield" action start the game straight from your app menu
- **Configurable launch options** — Gamemode, MangoHUD, DXVK Async, Wayland, custom env vars and arguments
- **System checks** — warns about missing dependencies (Proton, ntsync)
- **Custom UI** — glassmorphism-styled interface with no system decorations

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
| NSIS   | `bundle/nsis/LLauncher_0.3.2_x64-setup.exe`    |
| MSI    | `bundle/msi/LLauncher_0.3.2_x64_en-US.msi`     |

The platform-specific bundler settings live in `src-tauri/tauri.linux.conf.json`
and `src-tauri/tauri.windows.conf.json`; Tauri merges the matching one over
`tauri.conf.json` automatically.

## Configuration

Settings are stored in `~/.config/llauncher/settings.json` (`%APPDATA%\llauncher\settings.json` on Windows) and can be edited through the in-app settings panel.

| Category  | Options                                                            |
| --------- | ------------------------------------------------------------------ |
| Paths     | Game directory, download directory, Proton directory               |
| Proton    | Manage installed versions, download DWProton, system check results |
| Launch    | Gamemode, MangoHUD, Vulkan, Wayland, DXVK Async, on-launch action |
| Downloads | Speed limit, concurrent connections, custom env vars, launch args  |

On Windows the Proton tab and every Proton-only launch option are hidden — the
game runs natively — and the Launch tab offers "Run as administrator" instead,
for the rare case where the anti-cheat refuses to load without elevation.

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

## Mods

Endfield mods are closer to resource packs than to Minecraft mods: they replace how
characters, weapons and the interface *look* on your screen. They cannot add
characters, items or mechanics — progress and content live on the server, which knows
nothing about them.

Almost all of them run on [3DMigoto](https://github.com/wakka810/3dmigoto-arknights-endfield),
a `d3d11.dll` proxy that intercepts draw calls and swaps in the models a mod supplies.
Settings → Mods installs it, opens the `Mods` folder and switches on a second launch
button. Then:

1. **Install the loader** — downloaded straight into the game directory.
2. **Drop mods into `Mods/`** — one directory per mod. [Catalogue](https://gamebanana.com/games/21842).
3. **Use "play with mods"** — the ordinary Play button is untouched.

Two caveats, both deliberate reasons the modded launch is a separate button rather
than a setting:

- **It costs frames on Linux.** 3DMigoto hooks DirectX 11 and has no Vulkan
  equivalent, so a modded session runs on the game's D3D11 path through DXVK instead
  of its native Vulkan renderer. On Windows the game already runs on D3D11 and only
  the loader's own overhead applies.
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
  `dxgi.dll` — `d3d11.dll` belongs to 3DMigoto — and the launcher picks it up on its
  own; the modded launch already sets the override it needs.

Mods that patch the game itself rather than the renderer work on a normal Vulkan
launch and need none of this.

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
│       ├── game/               #   Game state detection, launching (launcher/{linux,windows}.rs), mods
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

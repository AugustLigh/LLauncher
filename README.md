<div align="center">

# 🚀 LLauncher

**A native Linux launcher for Arknights: Endfield**

Built with Tauri v2, React, and Rust

[![Tauri](https://img.shields.io/badge/Tauri-v2-FFC131?logo=tauri&logoColor=white)](https://v2.tauri.app)
[![React](https://img.shields.io/badge/React-18-61DAFB?logo=react&logoColor=white)](https://react.dev)
[![Rust](https://img.shields.io/badge/Rust-2021-DEA584?logo=rust&logoColor=white)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE)

</div>

---

<img width="1280" height="719" alt="изображение" src="https://github.com/user-attachments/assets/181529d5-23b0-463a-b1a1-c5d13e9a4475" />


LLauncher is a lightweight, native Linux launcher for **Arknights: Endfield**. It handles game installation, updates, and launching through Proton — no Steam or Lutris required.

[Download the latest release](https://github.com/AugustLigh/LLauncher/releases/latest) (AppImage / .deb / .rpm)

On Arch Linux (and derivatives like CachyOS, Manjaro, EndeavourOS) install from the AUR — dependencies are handled automatically:

```bash
yay -S llauncher-bin   # or: paru -S llauncher-bin
```

## Features

- **One-click install & launch** — download, verify, extract, and play
- **Auto-updates** — detects new game versions and patches seamlessly
- **Proton management** — download and manage DWProton versions directly from the launcher
- **Multi-threaded downloads** — up to 8 concurrent connections with per-worker speed limiting
- **File verification** — MD5 checksum validation for every downloaded file, with smart skip for already verified files
- **System tray** — minimize to tray, launch from tray
- **In-app news** — announcements and updates from the official API
- **Configurable launch options** — Gamemode, MangoHUD, DXVK Async, Wayland, custom env vars and arguments
- **System checks** — warns about missing dependencies (Proton, ntsync)
- **Custom UI** — glassmorphism-styled interface with no system decorations

## Prerequisites

- **Linux** (x86_64)
- **Node.js** >= 18 and **npm** (or yarn)
- **Rust** toolchain ([rustup](https://rustup.rs))
- **System libraries** for Tauri v2 — see the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/#linux)
- **glib-networking** — its GIO TLS module is bundled into the AppImage at build time
- A **Proton** build (DWProton can be downloaded from within the launcher)

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
npx tauri build
```

Release bundles will be created in `src-tauri/target/release/bundle/`:

| Format   | Path                                         |
| -------- | -------------------------------------------- |
| AppImage | `bundle/appimage/LLauncher_0.1.0_amd64.AppImage` |
| .deb     | `bundle/deb/LLauncher_0.1.0_amd64.deb`           |
| .rpm     | `bundle/rpm/LLauncher-0.1.0-1.x86_64.rpm`        |

## Configuration

Settings are stored in `~/.config/llauncher/settings.json` and can be edited through the in-app settings panel.

| Category  | Options                                                            |
| --------- | ------------------------------------------------------------------ |
| Paths     | Game directory, download directory, Proton directory               |
| Proton    | Manage installed versions, download DWProton, system check results |
| Launch    | Gamemode, MangoHUD, Vulkan, Wayland, DXVK Async, on-launch action |
| Downloads | Speed limit, concurrent connections, custom env vars, launch args  |

<img width="1282" height="715" alt="изображение" src="https://github.com/user-attachments/assets/e9262948-b29a-4b93-bb2c-8e0438db8a6f" />


Default paths:

```
Game:   ~/Games/ArknightsEndfield
Proton: ~/.local/share/llauncher/proton
Config: ~/.config/llauncher/settings.json
Logs:   ~/.config/llauncher/launch.log
```

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
│       ├── game/               #   Game state detection, Proton launching
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

# Tommy Memory Cleaner

[![Release](https://github.com/tommy4377/Tommy-Memory-Cleaner/actions/workflows/release.yml/badge.svg)](https://github.com/tommy4377/Tommy-Memory-Cleaner/actions/workflows/release.yml)
![Version](https://img.shields.io/badge/version-4.0.0-blue)
![Platform](https://img.shields.io/badge/platform-Windows%2010%20%7C%2011-0078D6?logo=windows)
![Tauri](https://img.shields.io/badge/Tauri-2.0-FFC131?logo=tauri)
![Svelte](https://img.shields.io/badge/Svelte-4-FF3E00?logo=svelte)

**Tommy Memory Cleaner (TMC)** is a lightweight Windows desktop utility that frees up RAM on demand or automatically. It purges specific Windows memory areas — working sets, standby lists, file caches, and more — using native NT system calls, wrapped in a modern, themeable UI built with Tauri 2.0 and Svelte.

## Key Features

- **Eight targetable memory areas** — Working Set, Modified Page List, Standby List, Low-Priority Standby List, System File Cache, Combined Page List, Modified File Cache, and Registry Cache.
- **One-click optimization profiles**:
  | Profile | Memory areas | Best for |
  |---|---|---|
  | **Normal** | Working Set + Registry Cache + Low-Priority Standby | Instant cleanup with no perceptible latency |
  | **Balanced** | Normal + full Standby List + file caches | Deep refresh after heavy usage |
  | **Gaming** | Balanced + Modified/Combined Page Lists | Full RAM reset before gaming sessions |
- **Automatic optimization** on a schedule (interval in hours) or when free memory drops below a configurable threshold.
- **Live system tray monitor** — the tray icon shows current memory usage percentage with customizable colors and warning/danger levels.
- **Global hotkey** (default `Ctrl+Alt+N`) to optimize from anywhere, even in-game.
- **Process exclusions** with built-in protection for critical system processes.
- **Silent admin elevation** — one UAC prompt on first run; subsequent launches elevate silently via a scheduled task.
- **Compact and full view**, light/dark themes with custom accent colors, and 9 UI languages (English, Italian, Spanish, French, Portuguese, German, Arabic, Japanese, Chinese).
- **Console mode** for scripting and task automation (see [CLI usage](#command-line-usage)).

## Getting Started

### Requirements

- Windows 10 or Windows 11 (64-bit)
- Administrator privileges (required by the Windows memory-management APIs)
- WebView2 Runtime (bundled with the installer)

### Install

Download the latest `Tommy Memory Cleaner_x.x.x_x64-setup.exe` installer from the [Releases page](https://github.com/tommy4377/Tommy-Memory-Cleaner/releases) and run it. A first-run setup dialog lets you pick language, theme, autostart, and notification preferences.

### Basic usage

1. Launch the app — grant the UAC prompt so it can optimize system memory.
2. Pick a profile (**Normal**, **Balanced**, or **Gaming**).
3. Click **Optimize**, press the global hotkey, or right-click the tray icon → *Optimize*.
4. Optionally enable **Auto Optimization** in Settings to run on an interval or when free memory gets low.

### Command-line usage

The same executable runs headless when given arguments — handy for scripts and Task Scheduler:

```bat
:: Optimize specific memory areas
TommyMemoryCleaner.exe /WorkingSet /StandbyList

:: Run a predefined profile
TommyMemoryCleaner.exe /Profile:Balanced

:: Show all options
TommyMemoryCleaner.exe /?
```

### Build from source

Prerequisites: [Node.js](https://nodejs.org) 20+, [Rust](https://rustup.rs) (stable), and the Tauri CLI (`cargo install tauri-cli --version "^2"`).

```bash
git clone https://github.com/tommy4377/Tommy-Memory-Cleaner.git
cd Tommy-Memory-Cleaner/TMC/ui
npm install

# Development (hot reload)
cd ../src-tauri
cargo tauri dev

# Production build with NSIS installer
cargo tauri build --bundles nsis
```

The project layout:

- [`TMC/ui`](TMC/ui) — Svelte 4 + TypeScript frontend (Vite)
- [`TMC/src-tauri`](TMC/src-tauri) — Rust backend (memory engine, tray, hotkeys, CLI)
- [`.github/workflows/release.yml`](.github/workflows/release.yml) — CI release pipeline (builds the installer on `v*.*.*` tags)

## Getting Help

- **Bug reports & feature requests** — open an issue on the [GitHub issue tracker](https://github.com/tommy4377/Tommy-Memory-Cleaner/issues).
- **Questions** — use [GitHub Discussions](https://github.com/tommy4377/Tommy-Memory-Cleaner/discussions).
- **Diagnostics** — optimization results are logged to the Windows Event Viewer, and application logs are written to `%LOCALAPPDATA%\TommyMemoryCleaner`.

## Maintainers & Contributing

Tommy Memory Cleaner is created and maintained by [**Tommy437** (@tommy4377)](https://github.com/tommy4377).

Contributions are welcome:

1. Fork the repository and create a feature branch.
2. Make your changes — please keep comments and code in English and run `npm run lint` (frontend) and `cargo check` (backend) before submitting.
3. Open a pull request describing what you changed and why.

For anything larger than a small fix, open an issue first so the approach can be discussed.

## License

© 2025 Tommy437. All rights reserved. This project does not currently ship with an open-source license; contact the maintainer regarding reuse.

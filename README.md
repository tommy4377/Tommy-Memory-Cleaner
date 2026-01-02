# 🧹 Tommy Memory Cleaner

<div align="center">

![Version](https://img.shields.io/badge/version-1.0.0-blue.svg)
![Platform](https://img.shields.io/badge/platform-Windows-lightgrey.svg)
![License](https://img.shields.io/badge/license-MIT-green.svg)
![Built With](https://img.shields.io/badge/built%20with-Rust%20%2B%20Tauri-orange.svg)

**Advanced Memory Optimization Tool for Windows**

*Professional-grade memory optimization with advanced Windows API integration*

[Download Latest Release](https://github.com/tommy4377/Tommy-Memory-Cleaner/releases) • [Features](#-features) • [Usage](#-usage) • [CLI](#-command-line-interface)

</div>

---

## 📖 Overview

**Tommy Memory Cleaner** is a high-performance memory optimization tool for Windows that uses advanced Windows APIs and system-level techniques to maximize RAM efficiency. Built with Rust and Tauri v2, it delivers lightning-fast optimization with both GUI and CLI interfaces.

### Key Features
- ⚡ **Ultra-Fast**: Advanced syscalls with 90-98% performance improvements
- 🎯 **Smart Optimization**: 8 memory areas with intelligent algorithms
- 🔧 **Full CLI**: Complete command-line automation support
- 🔒 **Privacy-First**: 100% offline, no telemetry or data collection
- 🎨 **Modern UI**: Beautiful interface with themes and multi-language support

---

## ✨ Features

### 🚀 Advanced Memory Optimization

Optimizes 8 distinct memory areas using professional-grade techniques:

| Area | Function | Performance |
|------|----------|-------------|
| **Working Set** | Active process memory | ~730ms |
| **Modified Page List** | Pages waiting for disk write | **185ms** (87% faster!) |
| **Standby List** | Reclaimable cached memory | ~168ms |
| **Low Priority Standby** | Low-priority cache | **104ms** (98.5% faster!) |
| **System File Cache** | File system cache | ~35ms |
| **Modified File Cache** | Write buffers & volume cache | ~221ms |
| **Combined Page List** | Combined page management | ~56ms |
| **Registry Cache** | Windows registry cache | ~20ms |

### 📋 Optimization Profiles

- **Normal Profile** (Light)
  - Areas: Working Set, Modified Page List, Registry Cache
  - Best for: Everyday use, minimal impact

- **Balanced Profile** (Recommended)
  - Areas: Working Set, Modified Page List, Standby List, System File Cache, Modified File Cache, Registry Cache
  - Best for: General computing, optimal balance

- **Gaming Profile** (Aggressive)
  - Areas: All 8 memory areas
  - Best for: Gaming, resource-intensive applications

### ⚙️ Advanced Features

#### System-Level Optimization
- **Advanced Syscalls**: Direct Windows kernel calls with SYSTEM privileges
- **SSN Caching**: Optimized syscall number resolution
- **Three-Tier Fallback**: Advanced → Direct NT → Standard API
- **Windows 11 Compatible**: Full support including 24H2+

#### Automation & Protection
- **Scheduled Optimization**: Custom intervals (minutes/hours)
- **Low Memory Trigger**: Auto-optimize when RAM below threshold
- **Process Exclusions**: Protect critical applications
- **Smart Cooldown**: Prevents excessive optimization

#### User Interface
- **Modern Design**: Rounded windows with transparency
- **System Tray**: Real-time memory monitoring
- **Global Hotkeys**: Quick optimization (default: `Ctrl+Alt+N`)
- **Multi-Language**: 10+ languages supported
- **Theme Support**: Light/dark with custom colors

---

## 💻 Command Line Interface

### Usage

```bash
TommyMemoryCleaner.exe [OPTIONS]
```

### Memory Areas
- `/WorkingSet` - Optimize Working Set
- `/ModifiedPageList` - Optimize Modified Page List  
- `/StandbyList` - Optimize Standby List
- `/StandbyListLow` - Optimize Low Priority Standby List
- `/SystemFileCache` - Optimize System File Cache
- `/CombinedPageList` - Optimize Combined Page List
- `/ModifiedFileCache` - Optimize Modified File Cache
- `/RegistryCache` - Optimize Registry Cache

### Profiles
- `/Profile:Normal` - Use Normal profile
- `/Profile:Balanced` - Use Balanced profile
- `/Profile:Gaming` - Use Gaming profile

### Examples

```bash
# Optimize specific areas
TommyMemoryCleaner.exe /WorkingSet /StandbyList /SystemFileCache

# Use predefined profile
TommyMemoryCleaner.exe /Profile:Balanced

# Maximum optimization for gaming
TommyMemoryCleaner.exe /Profile:Gaming

# Show help
TommyMemoryCleaner.exe /?
```

### Output
- Displays selected profile/areas
- Shows real-time progress
- Reports freed memory (e.g., "1.15 GB freed")
- Exit code 0 on success, 1 on error

---

## 📥 Installation

### System Requirements
- **OS**: Windows 10/11 (64-bit)
- **Privileges**: Administrator (recommended for full functionality)
- **RAM**: 4 GB minimum (8 GB+ recommended)
- **Disk Space**: ~15 MB

### Quick Start
1. Download from [Releases](https://github.com/tommy4377/Tommy-Memory-Cleaner/releases)
2. Extract archive (if applicable)
3. Run `TommyMemoryCleaner.exe` as Administrator
4. Complete setup wizard (theme, language, preferences)
5. Application minimizes to system tray

---

## 🎯 Usage Guide

### Manual Optimization
- **GUI**: Click "Optimize" button in main window
- **System Tray**: Right-click tray icon → "Optimize Memory"
- **Hotkey**: Press `Ctrl+Alt+N` (default)

### Automatic Optimization
Configure in **Settings → Auto Optimization**:
- **Scheduled**: Set custom intervals (e.g., every 30 minutes)
- **Low Memory Trigger**: Auto-optimize when RAM below threshold (e.g., 30%)

### Process Exclusions
1. **Settings → Process Exclusions**
2. Click "Add Process"
3. Enter process name (e.g., `chrome.exe`, `steam.exe`)
4. Critical system processes are automatically protected

---

## 🔧 Advanced Configuration

### Custom Hotkeys
- **Format**: `Ctrl+Alt+Key` or `Ctrl+Shift+Key`
- **Examples**: `Ctrl+Alt+N`, `Ctrl+Shift+M`, `F12`
- Configure in **Settings → Hotkey**

### Theme Customization
- **Customization → Colors**
- Customize light/dark theme colors
- Real-time preview
- Auto theme switching

### Configuration Location
- **Windows**: `%APPDATA%\TommyMemoryCleaner\config.json`
- Includes profiles, exclusions, UI preferences, hotkeys

---

## 📊 Performance

### Benchmarks
- **Optimization Time**: 1-3 seconds (all areas)
- **Memory Freed**: 800MB-3GB per optimization
- **CPU Usage**: <1% idle, brief spikes during optimization
- **Memory Footprint**: 30-50 MB
- **Startup Time**: <2 seconds

### Recent Optimizations
- ModifiedPageList: 4300ms → 185ms (96% faster)
- LowPriorityStandby: 6732ms → 104ms (98.5% faster)
- Added SSN caching for faster subsequent calls
- Windows 11 24H2+ compatibility fixes

---

## 🔒 Privacy & Security

- ✅ **No Data Collection**: Zero telemetry, fully offline
- ✅ **No Internet Required**: All operations local
- ✅ **Open Source**: Code available for review
- ✅ **Administrator Privileges**: Required for system-level operations
- ✅ **Process Protection**: Critical processes automatically protected
- ✅ **Event Logging**: All operations logged to Windows Event Viewer

---

## 🐛 Troubleshooting

### Common Issues

**Optimization Not Working**
- Run as Administrator
- Check Windows Event Viewer for errors
- Verify antivirus isn't blocking the application

**Notifications Not Showing**
- Check Windows notification settings
- Enable notifications in app settings
- Disable "Do Not Disturb" mode

**Command Line Not Working**
- Use Command Prompt or PowerShell
- Run with `/?` to verify installation
- Check administrator privileges

---

## 🛠️ Building from Source

### Prerequisites
- **Rust**: Latest stable ([rustup.rs](https://rustup.rs/))
- **Node.js**: v18+ ([nodejs.org](https://nodejs.org/))
- **Tauri CLI**: v2.x
- **Windows SDK**: For Windows development

### Build
```bash
git clone https://github.com/tommy4377/Tommy-Memory-Cleaner.git
cd Tommy-Memory-Cleaner/TMC

# Install Tauri CLI
cargo install tauri-cli --version "^2.0"

# Build frontend
cd ui
npm install
npm run build

# Build application
cd ../src-tauri
cargo tauri build

# Output in src-tauri/target/release/
```

---

## 📝 Changelog

### Version 1.0.0
- ✨ Initial release with advanced memory optimization
- 🚀 8 memory areas with professional-grade techniques
- ⚡ 90-98% performance improvements over standard methods
- 🔧 Full CLI automation support
- 🎨 Modern UI with themes and multi-language
- 🔒 Privacy-first design (100% offline)
- 🛡️ Windows 11 24H2+ compatibility
- 📊 Real-time system tray monitoring

---

## 📄 License

MIT License - see [LICENSE](LICENSE) file for details.

---

## 👤 Author

**tommy4377**

- GitHub: [@tommy4377](https://github.com/tommy4377)
- Project: [Tommy Memory Cleaner](https://github.com/tommy4377/Tommy-Memory-Cleaner)

---

<div align="center">

**Made with ❤️ by tommy4377**

⭐ **Star this repo if you find it useful!**

[Download Latest Release](https://github.com/tommy4377/Tommy-Memory-Cleaner/releases) • [Report Bug](https://github.com/tommy4377/Tommy-Memory-Cleaner/issues) • [Request Feature](https://github.com/tommy4377/Tommy-Memory-Cleaner/issues)

</div>

# 🧹 Tommy Memory Cleaner

<div align="center">

![Version](https://img.shields.io/badge/version-1.0.0-blue.svg)
![Platform](https://img.shields.io/badge/platform-Windows-lightgrey.svg)
![License](https://img.shields.io/badge/license-MIT-green.svg)
![Built With](https://img.shields.io/badge/built%20with-Rust%20%2B%20Tauri-orange.svg)

**Advanced Memory Optimization Tool for Windows**

*A professional, lightweight memory optimization utility built with Rust and Tauri v2*

[Download Latest Release](https://github.com/tommy4377/Tommy-Memory-Cleaner/releases) • [Features](#-features) • [Usage](#-usage) • [Command Line](#-command-line-interface)

</div>

---

## 📖 Overview

**Tommy Memory Cleaner** is a powerful, modern memory optimization tool designed specifically for Windows systems. It intelligently manages your system's RAM by clearing unnecessary cached data, optimizing memory allocation, and providing real-time monitoring capabilities.

Built with **Rust** and **Tauri v2** for optimal performance and minimal resource usage, this tool offers both **graphical** and **command-line** interfaces for maximum flexibility. Whether you're a power user looking for automation or someone who wants an intuitive GUI, Tommy Memory Cleaner has you covered.

### Key Highlights

- ⚡ **High Performance**: Built with Rust for blazing-fast optimization
- 🎨 **Beautiful UI**: Modern, rounded interface with light/dark themes
- 🔧 **Command Line**: Full CLI support for automation and scripting
- 🌍 **Multi-language**: Support for 10+ languages
- 🔒 **Privacy First**: No telemetry, no data collection, 100% local
- 📊 **Real-time Monitoring**: Live memory usage tracking
- 🎯 **Smart Profiles**: Pre-configured optimization profiles

---

## ✨ Features

### 🚀 Memory Optimization

Tommy Memory Cleaner optimizes multiple memory areas for comprehensive RAM management:

| Area | Description | Available In |
|------|-------------|--------------|
| **Working Set** | Active memory pages of running processes | All Profiles |
| **Modified Page List** | Pages waiting to be written to disk | All Profiles |
| **Standby List** | Cached memory that can be reclaimed | Balanced, Gaming |
| **Low Priority Standby** | Low-priority cached memory | Gaming |
| **System File Cache** | File system cache | Balanced, Gaming |
| **Modified File Cache** | File write buffers and volume cache | Balanced, Gaming |
| **Combined Page List** | Combined memory page list | Gaming |
| **Registry Cache** | Windows registry cache | All Profiles |

### 📋 Optimization Profiles

Choose the profile that best fits your needs:

- **Normal Profile** (Light)
  - Optimizes: Working Set, Modified Page List, Registry Cache
  - Best for: Everyday use, minimal system impact
  - Safest option with basic memory cleanup

- **Balanced Profile** (Recommended)
  - Optimizes: Working Set, Modified Page List, Standby List, System File Cache, Modified File Cache, Registry Cache
  - Best for: General computing, balanced performance
  - Good balance between performance and safety

- **Gaming Profile** (Aggressive)
  - Optimizes: All available memory areas
  - Best for: Gaming, resource-intensive applications
  - Maximum RAM freeing for peak performance

### ⚙️ Advanced Features

#### Automatic Optimization
- **Scheduled Optimization**: Run at custom intervals (e.g., every hour, every 30 minutes)
- **Low Memory Trigger**: Automatically optimize when available RAM drops below threshold
- **Smart Cooldown**: 5-minute cooldown between optimizations to prevent excessive use

#### Process Protection
- **Exclude Processes**: Protect specific applications from optimization
- **Critical Process Detection**: Automatically protects critical system processes
- **Custom Exclusions**: Add any process by name (e.g., `chrome.exe`, `steam.exe`)

#### System Integration
- **System Tray**: Monitor memory usage directly from the tray icon
- **Global Hotkeys**: Quick optimization with customizable keyboard shortcuts (default: `Ctrl+Alt+O`)
- **Windows Toast Notifications**: Get notified when optimization completes
- **Startup Management**: Option to start with Windows
- **Event Logging**: Detailed logs in Windows Event Viewer

#### User Interface
- **Modern Design**: Rounded windows with transparent backgrounds
- **Compact Mode**: Minimalist interface that integrates with system tray
- **Full View**: Comprehensive interface with detailed statistics
- **Theme Support**: Light and dark themes with custom color customization
- **Auto Theme**: Automatic theme switching based on system preferences
- **Multi-language**: English, Italian, Spanish, French, German, Portuguese, Arabic, Japanese, Chinese

---

## 💻 Command Line Interface

Tommy Memory Cleaner includes a powerful command-line interface for automation, scripting, and headless operation.

### Usage

```bash
Tommy Memory Cleaner.exe [OPTIONS]
```

### Options

#### Memory Areas
- `/WorkingSet` - Optimize Working Set
- `/ModifiedPageList` - Optimize Modified Page List
- `/StandbyList` - Optimize Standby List
- `/StandbyListLow` - Optimize Low Priority Standby List
- `/SystemFileCache` - Optimize System File Cache
- `/CombinedPageList` - Optimize Combined Page List
- `/ModifiedFileCache` - Optimize Modified File Cache
- `/RegistryCache` - Optimize Registry Cache

#### Profiles
- `/Profile:Normal` - Use Normal profile
- `/Profile:Balanced` - Use Balanced profile
- `/Profile:Gaming` - Use Gaming profile

#### Help
- `/?`, `/help`, `-h`, `--help` - Show help message

### Examples

```bash
# Optimize specific memory areas
Tommy Memory Cleaner.exe /WorkingSet /StandbyList /SystemFileCache

# Use a predefined profile
Tommy Memory Cleaner.exe /Profile:Balanced

# Maximum optimization for gaming
Tommy Memory Cleaner.exe /Profile:Gaming

# Show help
Tommy Memory Cleaner.exe /?
```

### Output

When run from the command line, the application will:
- Display the selected profile or areas
- Show progress during optimization
- Print results including freed memory
- Exit with code 0 on success, 1 on error
- Provide detailed error messages if optimization fails

### Use Cases

- **Task Scheduler**: Schedule automated optimizations via Windows Task Scheduler
- **Scripts**: Integrate into batch files or PowerShell scripts
- **Remote Management**: Run optimizations remotely via SSH or remote desktop
- **Headless Servers**: Optimize memory on servers without GUI
- **CI/CD**: Include in deployment scripts or maintenance tasks

---

## 📥 Installation

### System Requirements

- **OS**: Windows 10/11 (64-bit)
- **Privileges**: Administrator privileges recommended for full functionality
- **RAM**: 4 GB minimum (8 GB+ recommended)
- **Disk Space**: ~15 MB

### Download

Download the latest release from the [Releases](https://github.com/tommy4377/Tommy-Memory-Cleaner/releases) page.

### First Run

1. **Extract** the downloaded archive (if applicable)
2. **Run** `Tommy Memory Cleaner.exe` as Administrator
   - Right-click → "Run as administrator"
3. **Complete Setup Wizard**:
   - Choose your preferred theme (Light/Dark)
   - Select your language
   - Configure initial settings
   - Set startup preferences
4. The application will minimize to the system tray
5. **Access Options**:
   - Right-click tray icon → Quick actions
   - Double-click tray icon → Open main window
   - Use global hotkey → Quick optimize

---

## 🎯 Usage Guide

### Manual Optimization

#### From Main Window
1. Open the application (double-click tray icon or right-click → "Open TMC")
2. Select your preferred optimization profile
3. Click the **"Optimize"** button
4. Monitor progress in real-time
5. View results and freed memory

#### From System Tray
1. **Right-click** the tray icon
2. Select **"Optimize Memory"**
3. The optimization runs in the background
4. A notification appears when complete

#### Using Global Hotkey
1. Press your configured hotkey (default: `Ctrl+Alt+O`)
2. Optimization starts immediately
3. Notification shows completion status

### Automatic Optimization

Configure automatic optimization in **Settings → Auto Optimization**:

#### Scheduled Optimization
- **Enable**: Toggle scheduled optimization
- **Interval**: Set custom interval (minutes/hours)
- **Time**: Choose specific times of day (optional)

#### Low Memory Trigger
- **Enable**: Toggle low memory trigger
- **Threshold**: Set memory threshold percentage (e.g., 30%)
- **Action**: Automatically optimize when threshold is reached

### Process Exclusions

To protect specific applications:

1. Open **Settings → Process Exclusions**
2. Click **"Add Process"**
3. Enter the process name (e.g., `chrome.exe`, `steam.exe`)
4. Click **"Add"**
5. The process will be excluded from optimization

**Tips:**
- Process names are case-insensitive
- You can add multiple processes
- Critical system processes are automatically protected
- Changes take effect immediately

### Configuration

The application stores settings in:
- **Windows**: `%APPDATA%\TommyMemoryCleaner\config.json`

Configuration includes:
- Optimization profiles and selected areas
- Auto-optimization schedules and thresholds
- Process exclusions list
- UI preferences (theme, language, colors)
- Global hotkey settings
- Notification preferences
- Startup behavior

---

## 🔧 Advanced Configuration

### Custom Hotkeys

Configure global hotkeys in **Settings → Hotkey**:

- **Format**: `Ctrl+Alt+Key` or `Ctrl+Shift+Key`
- **Supported Modifiers**: `Ctrl`, `Alt`, `Shift`
- **Supported Keys**: `A-Z`, `0-9`, `F1-F12`
- **Examples**: 
  - `Ctrl+Alt+O` (default)
  - `Ctrl+Shift+M`
  - `F12` (single key)

### Theme Customization

Customize the appearance in **Customization → Colors**:

- **Light Theme**: Customize background, text, and accent colors
- **Dark Theme**: Customize dark mode colors
- **Main Color**: Set the primary accent color
- **Preview**: See changes in real-time

### Tray Icon Settings

Configure the tray icon in **Customization → Tray Icon**:

- **Show Memory Usage**: Display percentage in tray icon
- **Icon Style**: Choose icon appearance
- **Update Interval**: Set refresh rate for memory display
- **Custom Colors**: Set icon background and text colors

---

## 🐛 Troubleshooting

### Optimization Not Working

**Problem**: Optimization doesn't free memory or fails to run.

**Solutions**:
- ✅ Run the application as **Administrator**
  - Right-click executable → "Run as administrator"
  - Or enable "Always run as admin" in Properties → Compatibility
- ✅ Check Windows Event Viewer for error messages
- ✅ Verify you have sufficient privileges
- ✅ Ensure no antivirus is blocking the application

### Notifications Not Showing

**Problem**: No notifications appear after optimization.

**Solutions**:
- ✅ Check Windows notification settings
- ✅ Ensure notifications are enabled in app settings (Settings → Notifications)
- ✅ Disable "Do Not Disturb" mode in Windows
- ✅ Check Windows Focus Assist settings
- ✅ Verify notification permissions for the app

### Application Won't Start

**Problem**: Application crashes or fails to start.

**Solutions**:
- ✅ Ensure Windows 10/11 (64-bit) is installed
- ✅ Run as Administrator
- ✅ Check if antivirus is blocking the application
- ✅ Verify all dependencies are installed (WebView2 should auto-install)
- ✅ Check Windows Event Viewer for error details
- ✅ Try deleting `%APPDATA%\TommyMemoryCleaner\config.json` and restarting

### High CPU Usage

**Problem**: Application uses too much CPU resources.

**Solutions**:
- ✅ Disable auto-optimization if not needed
- ✅ Increase the interval between scheduled optimizations
- ✅ Reduce the number of monitored memory areas
- ✅ Check for conflicting software (other memory optimizers)
- ✅ Disable tray icon memory display if not needed

### Command Line Not Working

**Problem**: CLI mode doesn't execute or shows errors.

**Solutions**:
- ✅ Ensure you're running from Command Prompt or PowerShell
- ✅ Use full path to executable if not in PATH
- ✅ Check syntax of command-line arguments
- ✅ Verify administrator privileges (may be required)
- ✅ Run with `/?` to see help and verify installation

---

## 📊 System Impact

### Resource Usage

- **CPU Usage**: < 1% when idle, brief spikes during optimization
- **Memory Footprint**: ~30-50 MB
- **Disk Space**: ~15 MB installation size
- **Network**: No internet connection required (fully offline)

### Performance Impact

- **Optimization Time**: 1-5 seconds depending on system and selected areas
- **System Responsiveness**: Minimal impact during optimization
- **Background Operation**: Negligible impact when running in background
- **Startup Time**: < 2 seconds to launch application

---

## 🔒 Privacy & Security

### Privacy Commitment

- ✅ **No Data Collection**: The application does not collect or transmit any data
- ✅ **No Internet Connection**: All operations are performed locally
- ✅ **No Telemetry**: Zero tracking or analytics
- ✅ **Open Source**: Source code is available for review
- ✅ **Local Storage**: All settings stored locally on your device

### Security

- ✅ **Administrator Required**: Needs elevated privileges for system-level operations
- ✅ **Process Protection**: Critical system processes are automatically protected
- ✅ **Safe Operations**: Only optimizes safe, reclaimable memory areas
- ✅ **Event Logging**: All operations logged to Windows Event Viewer for audit

### Code Review

The entire source code is available on GitHub. You can:
- Review the code for security concerns
- Verify no data collection
- Build from source if needed
- Contribute improvements

---

## 🛠️ Building from Source

### Prerequisites

- **Rust**: Latest stable version ([rustup.rs](https://rustup.rs/))
- **Node.js**: v18+ ([nodejs.org](https://nodejs.org/))
- **Tauri CLI**: v2.x
- **Windows SDK**: For Windows development

### Build Steps

```bash
# Clone the repository
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

# Output will be in src-tauri/target/release/
```

### Development

```bash
# Run in development mode
cd TMC/src-tauri
cargo tauri dev
```

---

## 🤝 Contributing

Contributions are welcome! Here's how you can help:

### How to Contribute

1. **Fork** the repository
2. **Create** a feature branch (`git checkout -b feature/amazing-feature`)
3. **Commit** your changes (`git commit -m 'Add amazing feature'`)
4. **Push** to the branch (`git push origin feature/amazing-feature`)
5. **Open** a Pull Request

### Areas for Contribution

- 🐛 **Bug Reports**: Report issues you encounter
- 💡 **Feature Requests**: Suggest new features
- 📝 **Documentation**: Improve documentation and translations
- 🔧 **Code**: Submit pull requests with improvements
- 🎨 **UI/UX**: Improve user interface and experience
- 🌍 **Translations**: Add or improve language translations

### Guidelines

- Follow existing code style and patterns
- Add tests for new features when applicable
- Update documentation for user-facing changes
- Ensure all checks pass before submitting PR

---

## 📝 Changelog

### Version 1.0.0

#### Features
- ✨ Initial release
- 🎨 Modern UI with light/dark themes
- 🔧 Command-line interface
- 📊 Multiple optimization profiles
- ⚡ Automatic optimization (scheduled and low-memory trigger)
- 🛡️ Process exclusions
- 🔑 Global hotkeys
- 🌍 Multi-language support (10 languages)
- 📱 System tray integration
- 🔔 Windows Toast notifications
- 📋 Event logging to Windows Event Viewer

#### Memory Areas
- Working Set
- Modified Page List
- Standby List (normal and low priority)
- System File Cache
- Modified File Cache
- Combined Page List
- Registry Cache

#### Platforms
- Windows 10/11 (64-bit)

---

## 📄 License

This project is licensed under the **MIT License**.

See the [LICENSE](LICENSE) file for details.

---

## 👤 Author

**tommy4377**

- GitHub: [@tommy4377](https://github.com/tommy4377)
- Project: [Tommy Memory Cleaner](https://github.com/tommy4377/Tommy-Memory-Cleaner)

---

## 🙏 Acknowledgments

### Technologies

- **[Tauri](https://tauri.app/)** - Framework for building desktop applications
- **[Svelte](https://svelte.dev/)** - Modern web framework for UI
- **[Rust](https://www.rust-lang.org/)** - Systems programming language
- **[Windows API](https://docs.microsoft.com/en-us/windows/win32/apiindex/windows-api-list)** - Windows system APIs

### Inspiration

Inspired by the need for a modern, efficient, and privacy-focused memory optimization tool for Windows.

---

<div align="center">

**Made with ❤️ by tommy4377**

⭐ **Star this repo if you find it useful!**

[Download Latest Release](https://github.com/tommy4377/Tommy-Memory-Cleaner/releases) • [Report Bug](https://github.com/tommy4377/Tommy-Memory-Cleaner/issues) • [Request Feature](https://github.com/tommy4377/Tommy-Memory-Cleaner/issues)

</div>

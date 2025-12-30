# 🧹 Tommy Memory Cleaner

<div align="center">

![Version](https://img.shields.io/badge/version-1.0.0-blue.svg)
![Platform](https://img.shields.io/badge/platform-Windows-lightgrey.svg)

**Advanced Memory Optimization Tool for Windows**

A professional, lightweight memory optimization utility that helps you free up RAM and improve system performance.

[Download Latest Release](https://github.com/tommy4377/Tommy-Memory-Cleaner/releases) • [Features](#-features) • [Usage](#-usage)

</div>

---

## 📖 What is Tommy Memory Cleaner?

**Tommy Memory Cleaner** is an advanced memory optimization tool designed for Windows systems. It intelligently manages your system's RAM by clearing unnecessary cached data, optimizing memory allocation, and providing real-time monitoring of your system's memory usage.

Built with modern technologies (Rust + Tauri v2) for optimal performance and minimal resource usage, this tool offers both manual and automatic memory optimization to keep your system running smoothly.

## ✨ Features

### 🚀 Core Functionality

- **Smart Memory Optimization**: Automatically clears unused memory areas including:
  - Working Set memory
  - Modified Page List
  - Standby List (normal and low priority)
  - System File Cache
  - Registry Cache
  - Combined Page List
  
- **Multiple Optimization Profiles**:
  - **Normal**: Balanced optimization for everyday use
  - **Balanced**: Moderate optimization with enhanced process protection
  - **Gaming**: Aggressive optimization for maximum performance during gaming

- **Automatic Optimization**:
  - Scheduled optimization (customizable intervals)
  - Low memory trigger (optimizes when available RAM drops below threshold)
  - Smart scheduling to avoid interfering with active applications

- **Process Protection**: Exclude specific applications from optimization to ensure they always have sufficient memory and remain stable

- **Real-time Memory Monitoring**: Visual indicators showing:
  - Total system memory
  - Currently used memory
  - Available memory
  - Memory usage percentage

### 🎨 User Interface

- **Modern, Beautiful Design**: Rounded window with transparent background and smooth animations
- **Compact Mode**: Minimalist interface that integrates with the system tray
- **Full View**: Comprehensive interface with detailed statistics, graphs, and settings
- **Theme Support**: 
  - Light and dark themes
  - Custom color customization
  - Automatic theme switching based on system preferences
  
- **Multi-language Support**: Available in multiple languages including English, Italian, Arabic, Spanish, French, German, Portuguese, Japanese, and Chinese

### ⚙️ Advanced Features

- **System Tray Integration**: 
  - Monitor memory usage directly from the tray icon
  - Quick access to optimization functions
  - Visual memory indicators in the tray
  
- **Global Hotkeys**: 
  - Quickly trigger optimization with customizable keyboard shortcuts
  - Default: `Ctrl+Alt+O` (customizable)
  
- **Windows Toast Notifications**: 
  - Receive notifications when optimization completes
  - Get alerts for low memory situations
  - Customizable notification preferences
  
- **Event Logging**: 
  - Detailed logs saved to Windows Event Viewer
  - Track optimization history and results
  - Troubleshooting information
  
- **Startup Management**: 
  - Option to start with Windows
  - Configure auto-start behavior
  - Minimize to tray on startup

## 📥 Installation

### Download

Download the latest release from the [Releases](https://github.com/tommy4377/Tommy-Memory-Cleaner/releases) page.

**System Requirements:**
- Windows 10/11 (64-bit)
- Administrator privileges recommended for full functionality

### First Run

1. Run `Tommy Memory Cleaner.exe` (may require administrator privileges)
2. Complete the initial setup wizard:
   - Choose your preferred theme (Light/Dark)
   - Select your language
   - Configure initial settings
3. The application will minimize to the system tray
4. Right-click the tray icon to access the main window or quick actions

## 🎯 Usage

### Manual Optimization

1. **From Main Window**: Click the "Optimize" button
2. **From System Tray**: Right-click the tray icon → "Optimize Now"
3. **Using Hotkey**: Press your configured global hotkey (default: `Ctrl+Alt+O`)

### Automatic Optimization

Configure automatic optimization in Settings → Auto Optimization:

- **Scheduled Optimization**: 
  - Set custom intervals (e.g., every hour, every 30 minutes)
  - Choose specific times of day
  - Enable/disable as needed

- **Low Memory Trigger**:
  - Set memory threshold (e.g., optimize when available RAM < 2 GB)
  - Automatic trigger when threshold is reached
  - Prevents system slowdown due to low memory

### Optimization Profiles

Choose the profile that best fits your needs:

- **Normal Profile**: 
  - Safest option for general use
  - Optimizes basic memory areas
  - Suitable for everyday computing

- **Balanced Profile**: 
  - Good balance between performance and safety
  - More thorough memory cleanup
  - Enhanced process protection

- **Gaming Profile**: 
  - Maximum memory optimization
  - Best for gaming and resource-intensive applications
  - Frees up maximum RAM for your games

### Process Exclusions

To protect specific applications from optimization:

1. Go to Settings → Process Exclusions
2. Click "Add Process"
3. Enter the process name (e.g., `chrome.exe`, `steam.exe`)
4. The application will be protected from memory optimization

### System Tray Features

- **Memory Indicator**: Visual bar showing current memory usage
- **Quick Optimize**: One-click optimization
- **Open Main Window**: Access full interface
- **Settings**: Quick access to configuration
- **Exit**: Close the application

## 🔧 Configuration

The application stores settings in:
- **Windows**: `%APPDATA%\TommyMemoryCleaner\config.json`

Key configuration options:
- Optimization profiles and memory areas
- Auto-optimization schedules and thresholds
- Process exclusions list
- UI preferences (theme, language, colors)
- Global hotkey configuration
- Notification settings
- Startup behavior

## 🐛 Troubleshooting

### Optimization Not Working

- **Solution**: Run the application as Administrator
  - Right-click the executable → "Run as administrator"
  - Or enable "Always run as admin" in Properties → Compatibility

### Notifications Not Showing

- Check Windows notification settings
- Ensure notifications are enabled in the app settings
- Check if "Do Not Disturb" mode is active in Windows

### Application Won't Start

- Ensure Windows 10/11 is installed (64-bit)
- Check if antivirus is blocking the application
- Try running as Administrator

### High CPU Usage

- Disable auto-optimization if not needed
- Increase the interval between scheduled optimizations
- Check for conflicting software
- Reduce the number of monitored memory areas

## 📊 System Impact

- **CPU Usage**: Minimal (< 1% when idle)
- **Memory Footprint**: ~30-50 MB
- **Disk Space**: ~15 MB
- **Network**: No internet connection required

## 🔒 Privacy & Security

- **No Data Collection**: The application does not collect or transmit any data
- **No Internet Connection**: All operations are performed locally
- **Open Source**: Source code is available for review
- **No Telemetry**: Zero tracking or analytics

## 🤝 Contributing

Contributions are welcome! Feel free to:
- Report bugs
- Suggest features
- Submit pull requests
- Improve documentation

## 📝 License

This project is licensed under the MIT License.

## 👤 Author

**tommy4377**

- GitHub: [@tommy4377](https://github.com/tommy4377)

## 🙏 Acknowledgments

- Built with [Tauri](https://tauri.app/) - Framework for building desktop applications
- UI powered by [Svelte](https://svelte.dev/) - Modern web framework
- Icons and assets created specifically for this project

---

<div align="center">

**Made with ❤️ by tommy4377**

⭐ Star this repo if you find it useful!

[Download Latest Release](https://github.com/tommy4377/Tommy-Memory-Cleaner/releases)

</div>

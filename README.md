# 🧹 Tommy Memory Cleaner

<div align="center">

![Version](https://img.shields.io/badge/version-1.0.0-blue.svg)
![License](https://img.shields.io/badge/license-MIT-green.svg)
![Platform](https://img.shields.io/badge/platform-Windows-lightgrey.svg)
![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)
![Tauri](https://img.shields.io/badge/tauri-2.0-purple.svg)

**Advanced Memory Optimization Tool for Windows**

A professional, lightweight memory optimization utility built with Rust and Tauri v2, featuring a modern Svelte-based UI.

[Features](#-features) • [Installation](#-installation) • [Building](#-building) • [Usage](#-usage) • [Contributing](#-contributing)

</div>

---

## ✨ Features

### 🚀 Core Functionality
- **Memory Optimization**: Automatically clean and optimize system memory for better performance
- **Multiple Profiles**: Choose from Normal, Balanced, or Gaming profiles based on your needs
- **Auto Optimization**: Schedule automatic memory cleaning or trigger when memory is low
- **Process Protection**: Exclude specific processes from optimization to ensure stability
- **Real-time Monitoring**: Track memory usage with live updates and visual indicators

### 🎨 User Interface
- **Modern Design**: Beautiful, rounded window with transparent background
- **Compact Mode**: Minimalist view for system tray integration
- **Full View**: Comprehensive interface with detailed statistics and settings
- **Theme Support**: Light and dark themes with custom color customization
- **Multi-language**: Support for multiple languages (English, Italian, Arabic, and more)
- **Responsive**: Adapts to different screen sizes and preferences

### ⚙️ Advanced Features
- **System Tray Integration**: Monitor and control from the system tray
- **Global Hotkeys**: Quick optimization with customizable keyboard shortcuts
- **Windows Notifications**: Toast notifications for optimization events
- **Event Logging**: Detailed logging to Windows Event Viewer
- **Startup Management**: Configure automatic startup with Windows
- **Memory Areas**: Fine-grained control over which memory areas to optimize:
  - Working Set
  - Modified Page List
  - Standby List
  - System File Cache
  - Registry Cache
  - Combined Page List
  - And more...

## 📦 Installation

### Pre-built Binary
Download the latest release from the [Releases](https://github.com/tommy437/Tommy-Memory-Cleaner/releases) page and run the installer.

### Requirements
- Windows 10/11 (64-bit)
- Administrator privileges (for full optimization features)

## 🔨 Building

### Prerequisites
- [Rust](https://www.rust-lang.org/tools/install) (latest stable)
- [Node.js](https://nodejs.org/) (v18 or higher)
- [npm](https://www.npmjs.com/) or [yarn](https://yarnpkg.com/)
- [Microsoft Visual C++ Build Tools](https://visualstudio.microsoft.com/downloads/) (for Windows)

### Build Steps

1. **Clone the repository**
   ```bash
   git clone https://github.com/tommy437/Tommy-Memory-Cleaner.git
   cd Tommy-Memory-Cleaner
   ```

2. **Install UI dependencies**
   ```bash
   cd TMC/ui
   npm install
   ```

3. **Build the application**
   ```bash
   cd ../src-tauri
   cargo tauri build
   ```

   The executable will be in `TMC/src-tauri/target/release/`

### Development

To run in development mode:

```bash
# Terminal 1: Start the UI dev server
cd TMC/ui
npm run dev

# Terminal 2: Run Tauri in dev mode
cd TMC/src-tauri
cargo tauri dev
```

## 🎯 Usage

### First Launch
1. Run the application (requires administrator privileges)
2. Complete the initial setup wizard
3. Choose your preferred theme, language, and optimization profile
4. Configure auto-optimization settings if desired

### Manual Optimization
- Click the **Optimize** button in the main window
- Use the global hotkey (default: `Ctrl+Alt+O`)
- Right-click the system tray icon and select "Optimize Now"

### Auto-Optimization
Configure automatic optimization in Settings:
- **Scheduled**: Run at specific times (e.g., every hour)
- **Low Memory**: Trigger when available memory drops below a threshold
- **Both**: Combine scheduled and low-memory triggers

### Profiles
- **Normal**: Balanced optimization for everyday use
- **Balanced**: Moderate optimization with process protection
- **Gaming**: Aggressive optimization for maximum performance

### Process Exclusions
Add processes to the exclusion list to prevent them from being optimized:
1. Go to Settings → Process Exclusions
2. Click "Add Process"
3. Enter the process name (e.g., `chrome.exe`)
4. Save changes

## 🏗️ Architecture

### Tech Stack
- **Backend**: Rust with Tauri v2
- **Frontend**: Svelte 4 + TypeScript
- **Build Tool**: Vite
- **Styling**: CSS with custom themes
- **Icons**: Custom icon set with multiple resolutions

### Project Structure
```
Tommy-Memory-Cleaner/
├── TMC/
│   ├── src-tauri/          # Rust backend
│   │   ├── src/
│   │   │   ├── main.rs     # Main application logic
│   │   │   ├── engine.rs   # Optimization engine
│   │   │   ├── memory/     # Memory operations
│   │   │   ├── system/     # System integration
│   │   │   ├── ui/         # UI bridge and tray
│   │   │   └── ...
│   │   ├── Cargo.toml
│   │   └── tauri.conf.json
│   └── ui/                 # Svelte frontend
│       ├── src/
│       │   ├── components/ # UI components
│       │   ├── lib/        # Utilities and stores
│       │   └── i18n/       # Translations
│       └── package.json
└── README.md
```

## 🔧 Configuration

Configuration is stored in the user's AppData directory:
- **Windows**: `%APPDATA%\TommyMemoryCleaner\config.json`

Key settings:
- Optimization profiles and areas
- Auto-optimization schedules and thresholds
- Process exclusions
- UI preferences (theme, language, colors)
- Global hotkeys
- Notification settings

## 🐛 Troubleshooting

### Optimization Not Working
- Ensure the application is running with administrator privileges
- Check Windows Event Viewer for error messages
- Verify that processes aren't excluded from optimization

### Notifications Not Showing
- Check Windows notification settings
- Run `clean-app-data.bat` to clear notification cache (if available)
- Restart the application

### High CPU Usage
- Disable auto-optimization if not needed
- Increase the interval between scheduled optimizations
- Check for conflicting antivirus software

## 🤝 Contributing

Contributions are welcome! Please follow these steps:

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

### Code Style
- Rust: Follow the official [Rust Style Guide](https://doc.rust-lang.org/nightly/style-guide/)
- TypeScript/Svelte: Use ESLint and Prettier configurations
- Commits: Use conventional commit messages

## 📝 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 👤 Author

**tommy437**

- GitHub: [@tommy437](https://github.com/tommy437)

## 🙏 Acknowledgments

- Built with [Tauri](https://tauri.app/) - A framework for building desktop applications
- UI powered by [Svelte](https://svelte.dev/) - A radical new approach to building user interfaces
- Icons and assets created specifically for this project

## 📊 Statistics

- **Lines of Code**: ~15,000+
- **Languages**: Rust, TypeScript, Svelte, CSS
- **Dependencies**: 50+ (Rust), 20+ (Node.js)

---

<div align="center">

**Made with ❤️ by tommy437**

⭐ Star this repo if you find it useful!

</div>


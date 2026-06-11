# Tommy Memory Cleaner

A powerful, lightweight Windows system optimization tool that reclaims wasted memory through advanced optimization techniques. Built with Rust and Tauri for maximum performance and reliability.

## Features

### 🧠 Advanced Memory Optimization
- **Multi-area optimization**: Targets working set, standby list, modified page list, system file cache, registry cache, and memory compression store
- **Intelligent trimming**: Safely removes unused memory without affecting system stability
- **Stealth mode**: Minimizes process interruption with safe trimming strategies
- **Real-time statistics**: Monitor memory usage across physical RAM, committed memory, and cache

### ⚡ Smart Automation
- **Auto-optimization scheduler**: Run optimizations on custom schedules
- **Global hotkey support**: Instant memory cleanup with configurable keyboard shortcuts
- **Multiple optimization profiles**: Create and switch between different optimization strategies
- **Exclusion lists**: Protect critical processes from optimization

### 🎨 User-Friendly Interface
- **System tray integration**: Minimize to tray, quick access from taskbar
- **Compact and full views**: Choose between minimal or detailed UI
- **Dark/light theme**: Automatic or manual theme switching
- **Internationalization**: Multi-language support
- **Custom notifications**: Toast notifications for optimization events

### 🔒 Security & Stability
- **Privilege escalation**: Safe elevation for deep system optimization
- **Anti-virus whitelist support**: Integrate with Windows Defender and security software
- **Event logging**: Track all optimization operations in Windows Event Log
- **Elevated task runner**: Secure process management for privileged operations

## System Requirements

- **OS**: Windows 10 or later (64-bit)
- **Memory**: 100 MB minimum
- **Disk Space**: ~200 MB for installation

## Installation

### Option 1: Portable Build
1. Download `TommyMemoryCleaner.exe` from [Releases](../../releases)
2. Run the executable directly (no installation required)
3. Settings are stored in `%APPDATA%\Tommy Memory Cleaner\`

### Option 2: Build from Source

#### Prerequisites
- [Rust](https://www.rust-lang.org/tools/install) 1.70+ (Windows GNU or MSVC toolchain)
- [Node.js](https://nodejs.org/) 18+ and npm
- Windows SDK (for Tauri)

#### Build Steps
```bash
# Clone the repository
git clone https://github.com/tommy437/TommyMemoryCleaner.git
cd TommyMemoryCleaner/TMC

# Install frontend dependencies
cd ui
npm install
cd ..

# Build the project
cargo tauri build --target x86_64-pc-windows-msvc

# Release binary location:
# src-tauri/target/release/TommyMemoryCleaner.exe
```

## Usage

### Basic Operation
1. **Launch the application** - Tommy Memory Cleaner appears in your system tray
2. **Click tray icon** - Open the main window
3. **Click "Optimize"** - Run immediate memory optimization
4. **Monitor statistics** - View freed memory and current usage

### Configuration

#### Memory Optimization
Open **Settings** → **Memory Options**:
- **Working Set**: Trim process memory
- **Standby List**: Clear unused memory pages
- **Modified Page List**: Clean dirty memory pages
- **System File Cache**: Optimize file system cache
- **Registry Cache**: Trim registry memory

#### Auto-Optimization
Open **Settings** → **Auto Optimization**:
- Enable/disable scheduled optimizations
- Set custom schedule (hourly, daily, weekly)
- Choose optimization profile
- Configure notification preferences

#### Process Exclusions
Open **Settings** → **Process Exclusions**:
- Add processes to protection list
- Prevents specific applications from being optimized
- Useful for gaming or critical applications

#### Global Hotkeys
Open **Settings** → **Hotkeys**:
- Customize keyboard shortcut for quick optimization
- Default: `Ctrl+Shift+M`

#### Theme & Language
Open **Settings** → **Appearance**:
- Switch between light and dark themes
- Select interface language
- Adjust UI scaling

### Advanced Features

#### Memory Profiles
- **Balanced** (default): Safe optimization for most users
- **Aggressive**: Maximum memory reclamation
- **Custom**: Create your own profile with specific settings

#### Event Logging
- Optimization events logged to Windows Event Viewer
- Location: `Windows Logs → Application`
- Source: "Tommy Memory Cleaner"

## Architecture

### Rust Backend (`src-tauri/`)
- **Memory Engine**: Core optimization algorithms
- **Windows APIs**: Direct system-level memory operations
- **Security Module**: Privilege escalation and anti-virus integration
- **Auto Optimizer**: Scheduled optimization tasks
- **Global Hotkeys**: Keyboard shortcut handling
- **Notifications**: Windows Toast notifications

### TypeScript/Svelte Frontend (`ui/`)
- **Components**: Modular UI components for different views
- **Stores**: Reactive state management
- **i18n**: Multi-language interface
- **Theme System**: Dark/light mode support
- **IPC Bridge**: Communication with Rust backend

## Troubleshooting

### Application won't start
- Ensure Windows Defender or antivirus hasn't quarantined the app
- Add `TommyMemoryCleaner.exe` to antivirus whitelist
- Try running as Administrator
- Check `%APPDATA%\Tommy Memory Cleaner\` for error logs

### Limited optimization results
- Enable aggressive optimization profile
- Ensure enough RAM is available to free
- Close resource-heavy applications
- Check Process Exclusions list

### High CPU usage
- Disable Auto-Optimization temporarily
- Reduce optimization frequency
- Update to the latest version

### Settings not saving
- Run application with Administrator privileges
- Check folder permissions on `%APPDATA%\Tommy Memory Cleaner\`
- Ensure sufficient disk space available

## Performance Impact

- **Memory footprint**: ~30-50 MB typical usage
- **CPU usage**: < 1% idle, spikes only during optimization
- **Startup time**: < 2 seconds
- **Optimization speed**: ~100-500 MB/s depending on system

## Contributing

We welcome contributions! Please refer to [CONTRIBUTING.md](CONTRIBUTING.md) for:
- Code style guidelines
- Commit conventions
- Pull request process
- Development setup

### Development Quick Start
```bash
# Install dependencies
cd TMC
cd ui && npm install && cd ..

# Run in development mode
cargo tauri dev

# Format code
cd ui && npm run format && npm run lint
```

## Support

### Getting Help
- **Issues**: Report bugs via [GitHub Issues](../../issues)
- **Discussions**: Ask questions in [GitHub Discussions](../../discussions)
- **Documentation**: See [docs/](docs/) folder for detailed guides

### Reporting Security Issues
Please report security vulnerabilities responsibly to [security@example.com](mailto:security@example.com) rather than using the public issue tracker.

## Changelog

See [CHANGELOG.md](CHANGELOG.md) for release notes and version history.

## Maintainer

**Tommy** (@tommy4377)
- [GitHub Profile](https://github.com/tommy4377)

## Acknowledgments

Built with:
- [Tauri](https://tauri.app/) - Desktop framework
- [Rust](https://www.rust-lang.org/) - Systems programming
- [Svelte](https://svelte.dev/) - Reactive UI framework
- [Windows API](https://docs.microsoft.com/en-us/windows/win32/api/) - System integration

---

**Made with ❤️ for Windows users who care about system performance**

# Installation Guide

## Option 1: Install from crates.io (Recommended)

If you have Rust installed:

```bash
cargo install roxid
```

Then run anywhere:
```bash
roxid
```

## Option 2: Download Pre-built Binary

1. Go to [Releases](https://github.com/yourusername/roxid/releases/latest)
2. Download the binary for your platform:
   - **Linux x86_64**: `roxid-linux-x86_64.tar.gz`
   - **macOS Intel**: `roxid-macos-x86_64.tar.gz`
   - **macOS Apple Silicon**: `roxid-macos-aarch64.tar.gz`
   - **Windows**: `roxid-windows-x86_64.exe.zip`

3. Extract and add to your PATH:

### Linux/macOS:
```bash
tar xzf roxid-*.tar.gz
sudo mv roxid-* /usr/local/bin/roxid
chmod +x /usr/local/bin/roxid
```

### Windows:
Extract the ZIP and add the directory to your PATH environment variable.

## Option 3: Install from Git

```bash
cargo install --git https://github.com/yourusername/roxid
```

## Option 4: Build from Source

```bash
git clone https://github.com/yourusername/roxid
cd roxid
cargo build --release
sudo cp target/release/roxid /usr/local/bin/
```

## Verify Installation

```bash
roxid --help
```

## Uninstall

### If installed via cargo:
```bash
cargo uninstall roxid
```

### If installed manually:
```bash
sudo rm /usr/local/bin/roxid
```

## Platform-Specific Package Managers

### Homebrew (macOS/Linux) - Coming Soon
```bash
brew install roxid
```

### Scoop (Windows) - Coming Soon
```bash
scoop install roxid
```

### AUR (Arch Linux) - Coming Soon
```bash
yay -S roxid
```

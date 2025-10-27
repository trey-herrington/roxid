# Publishing Roxid

This guide explains how to publish and distribute Roxid.

## Prerequisites

1. **GitHub Repository**: Push your code to GitHub
2. **crates.io Account**: Sign up at https://crates.io
3. **API Token**: Get token from https://crates.io/me

## Publishing to crates.io

### First Time Setup

1. **Update Package Metadata** in `roxid-cli/Cargo.toml`:
   ```toml
   [package]
   name = "roxid"
   version = "0.1.0"
   authors = ["Your Name <your.email@example.com>"]
   description = "A Terminal User Interface (TUI) for managing and executing YAML-based pipelines"
   license = "MIT OR Apache-2.0"
   repository = "https://github.com/yourusername/roxid"
   ```

2. **Login to crates.io**:
   ```bash
   cargo login YOUR_API_TOKEN
   ```

3. **Dry Run** (test without publishing):
   ```bash
   cargo publish --dry-run -p roxid
   ```

4. **Publish**:
   ```bash
   # Publish dependencies first
   cargo publish -p pipeline-service
   cargo publish -p pipeline-rpc
   cargo publish -p roxid-tui
   
   # Then publish main package
   cargo publish -p roxid
   ```

### Updating Versions

1. Update version in `Cargo.toml`
2. Commit changes
3. Create git tag: `git tag v0.1.1`
4. Push tag: `git push --tags`
5. Publish: `cargo publish -p roxid`

## Creating GitHub Releases with Binaries

The included GitHub Actions workflow automatically builds binaries for:
- Linux (x86_64, musl)
- macOS (Intel, Apple Silicon)
- Windows (x86_64)

### Trigger a Release

1. **Create and push a tag**:
   ```bash
   git tag v0.1.0
   git push origin v0.1.0
   ```

2. **GitHub Actions will**:
   - Build binaries for all platforms
   - Create a GitHub Release
   - Attach all binaries to the release

3. **Users can download** from:
   `https://github.com/yourusername/roxid/releases`

### Manual Release (if needed)

```bash
# Build for your current platform
cargo build --release

# The binary is at:
# target/release/roxid
```

## Distribution Checklist

Before first release:

- [ ] Update all `Cargo.toml` files with proper metadata
- [ ] Add LICENSE files (MIT and Apache-2.0)
- [ ] Update README.md with installation instructions
- [ ] Test build on all platforms (or wait for CI)
- [ ] Create GitHub release notes
- [ ] Publish to crates.io
- [ ] Update INSTALL.md with correct URLs

## Package Managers (Future)

### Homebrew

Create a formula in a tap repository:
```ruby
class Roxid < Formula
  desc "Terminal UI for YAML pipeline management"
  homepage "https://github.com/yourusername/roxid"
  url "https://github.com/yourusername/roxid/archive/v0.1.0.tar.gz"
  sha256 "CALCULATED_SHA"

  def install
    system "cargo", "install", *std_cargo_args(path: "roxid-cli")
  end

  test do
    system "#{bin}/roxid", "--version"
  end
end
```

### Scoop (Windows)

Create a manifest:
```json
{
  "version": "0.1.0",
  "description": "Terminal UI for YAML pipeline management",
  "homepage": "https://github.com/yourusername/roxid",
  "license": "MIT OR Apache-2.0",
  "url": "https://github.com/yourusername/roxid/releases/download/v0.1.0/roxid-windows-x86_64.exe.zip",
  "bin": "roxid.exe"
}
```

### AUR (Arch Linux)

Create a PKGBUILD:
```bash
pkgname=roxid
pkgver=0.1.0
pkgrel=1
pkgdesc="Terminal UI for YAML pipeline management"
arch=('x86_64')
url="https://github.com/yourusername/roxid"
license=('MIT' 'Apache')
makedepends=('rust' 'cargo')
source=("$pkgname-$pkgver.tar.gz::$url/archive/v$pkgver.tar.gz")

build() {
  cd "$pkgname-$pkgver"
  cargo build --release --locked -p roxid
}

package() {
  cd "$pkgname-$pkgver"
  install -Dm755 "target/release/roxid" "$pkgdir/usr/bin/roxid"
}
```

## Troubleshooting

### crates.io publish fails

- Ensure package name is unique
- Check all dependencies are published first
- Verify Cargo.toml has required fields

### GitHub Actions fails

- Check workflow syntax
- Verify targets are correct
- Check build logs for errors

### Binary doesn't work on target platform

- Ensure correct target triple
- Check for missing system dependencies
- Verify static linking (use musl for Linux)

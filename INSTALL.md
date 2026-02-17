# Sekuire CLI Installation Guide

## Quick Install

### macOS (Apple Silicon)
```bash
curl -LO https://github.com/YOUR_ORG/agent/releases/latest/download/sekuire-darwin-arm64.tar.gz
tar -xzf sekuire-darwin-arm64.tar.gz
sudo mv sekuire-darwin-arm64 /usr/local/bin/sekuire
sekuire --version
```

### macOS (Intel)
```bash
curl -LO https://github.com/YOUR_ORG/agent/releases/latest/download/sekuire-darwin-amd64.tar.gz
tar -xzf sekuire-darwin-amd64.tar.gz
sudo mv sekuire-darwin-amd64 /usr/local/bin/sekuire
sekuire --version
```

### Linux (x86_64)
```bash
curl -LO https://github.com/YOUR_ORG/agent/releases/latest/download/sekuire-linux-amd64.tar.gz
tar -xzf sekuire-linux-amd64.tar.gz
sudo mv sekuire-linux-amd64 /usr/local/bin/sekuire
sekuire --version
```

---

## Installation Methods

### 1. Pre-built Binaries (Recommended)

Download the latest release from [GitHub Releases](https://github.com/YOUR_ORG/agent/releases).

**Verify the download** (recommended):
```bash
# Download SHA256 checksum
curl -LO https://github.com/YOUR_ORG/agent/releases/latest/download/sekuire-darwin-arm64.tar.gz.sha256

# Verify
sha256sum -c sekuire-darwin-arm64.tar.gz.sha256
```

### 2. Homebrew (macOS - Coming Soon)

```bash
brew tap sekuire/tap
brew install sekuire
```

### 3. Cargo (Build from Source)

If you have Rust installed:

```bash
cargo install sekuire-agent-cli
```

### 4. From Source (Developers)

```bash
git clone https://github.com/YOUR_ORG/agent.git
cd agent/packages/cli
cargo build --release
sudo cp target/release/sekuire /usr/local/bin/
```

---

## Verify Installation

```bash
sekuire --version
# Output: sekuire 0.2.0
```

---

## First Steps

### 1. Login to Sekuire Registry

```bash
sekuire login --registry https://registry.sekuire.ai
```

This will open your browser to authenticate. Once complete, you'll be ready to publish agents.

### 2. Initialize Your First Agent

```bash
mkdir my-agent
cd my-agent
sekuire init --name "MyAgent"
```

### 3. Generate Signing Keys

```bash
sekuire keygen
```

**Important**: Keep your `.sekuire/secret.key` file safe! This is your agent's private key.

---

## Configuration

### Global Config Location
- **macOS/Linux**: `~/.sekuire/config.json`
- **Windows**: `%USERPROFILE%\.sekuire\config.json`

### Example Config
```json
{
  "default_registry": "https://registry.sekuire.ai",
  "editor": "code",
  "auto_update_check": true
}
```

---

## Updating

### Pre-built Binaries
Download and replace the binary with the new version.

### Homebrew
```bash
brew upgrade sekuire
```

### Cargo
```bash
cargo install sekuire-agent-cli --force
```

---

## Uninstallation

### Remove Binary
```bash
sudo rm /usr/local/bin/sekuire
```

### Remove Config and Keys
```bash
rm -rf ~/.sekuire
```

### Homebrew
```bash
brew uninstall sekuire
```

---

## Troubleshooting

### "command not found: sekuire"
Make sure `/usr/local/bin` is in your PATH:
```bash
echo $PATH | grep /usr/local/bin
```

If not, add to your shell config:
```bash
# For bash
echo 'export PATH="/usr/local/bin:$PATH"' >> ~/.bashrc

# For zsh
echo 'export PATH="/usr/local/bin:$PATH"' >> ~/.zshrc
```

### Permission Denied
On macOS Catalina+, you may need to allow the binary:
```bash
xattr -d com.apple.quarantine /usr/local/bin/sekuire
```

Or go to **System Preferences → Security & Privacy** and click "Allow Anyway".

### OpenSSL Errors (Linux)
Install OpenSSL development libraries:
```bash
# Ubuntu/Debian
sudo apt-get install libssl-dev pkg-config

# Fedora/RHEL
sudo dnf install openssl-devel
```

---

## Support

- **Documentation**: https://docs.sekuire.ai
- **Issues**: https://github.com/YOUR_ORG/agent/issues
- **Discord**: https://discord.gg/sekuire

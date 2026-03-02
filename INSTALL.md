# Sekuire CLI Installation Guide

## Quick Install (Recommended)

```bash
curl -fsSL https://install.sekuire.ai | sh
```

Verify installation:

```bash
sekuire --version
```

---

## Installation Methods

### 1. Install Script (Recommended)

```bash
curl -fsSL https://install.sekuire.ai | sh
```

This installer:
- Detects your platform and architecture
- Downloads the latest Sekuire CLI binary
- Verifies the download with SHA256 checksum
- Installs to `~/.sekuire/bin/`
- Adds to your PATH automatically

### 2. Pre-built Binaries

Download the latest release from [GitHub Releases](https://github.com/YOUR_ORG/agent/releases).

**Verify the download** (recommended):
```bash
# Download SHA256 checksum
curl -LO https://github.com/YOUR_ORG/agent/releases/latest/download/sekuire-darwin-arm64.tar.gz.sha256

# Verify
sha256sum -c sekuire-darwin-arm64.tar.gz.sha256
```

### 3. Cargo (Build from Source)

If you have Rust installed:

```bash
cargo install sekuire-cli
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

### Cargo
```bash
cargo install sekuire-cli --force
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

---

## Troubleshooting

### "command not found: sekuire"
Make sure `~/.sekuire/bin` is in your PATH:
```bash
echo $PATH | grep "$HOME/.sekuire/bin"
```

If not, add to your shell config:
```bash
# For bash
echo 'export PATH="$HOME/.sekuire/bin:$PATH"' >> ~/.bashrc

# For zsh
echo 'export PATH="$HOME/.sekuire/bin:$PATH"' >> ~/.zshrc
```

### Permission Denied
On macOS Catalina+, you may need to allow the binary:
```bash
xattr -d com.apple.quarantine ~/.sekuire/bin/sekuire
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

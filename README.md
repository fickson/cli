# Sekuire CLI

The command-line interface for the Sekuire Trust Protocol.

## Installation

### Quick Install (Recommended)

```bash
curl -fsSL https://install.sekuire.ai | sh
```

Verify installation:
```bash
sekuire --version
```

### Cargo

```bash
cargo install sekuire-agent-cli
```

### Manual Binary Installation

Download the latest release from [GitHub Releases](https://github.com/sekuire/releases/releases/latest).

**macOS (Apple Silicon):**
```bash
curl -LO https://github.com/sekuire/releases/releases/latest/download/sekuire-darwin-arm64.tar.gz
tar -xzf sekuire-darwin-arm64.tar.gz
sudo mv sekuire /usr/local/bin/sekuire
```

**macOS (Intel):**
```bash
curl -LO https://github.com/sekuire/releases/releases/latest/download/sekuire-darwin-amd64.tar.gz
tar -xzf sekuire-darwin-amd64.tar.gz
sudo mv sekuire /usr/local/bin/sekuire
```

**Linux (x86_64):**
```bash
curl -LO https://github.com/sekuire/releases/releases/latest/download/sekuire-linux-amd64.tar.gz
tar -xzf sekuire-linux-amd64.tar.gz
sudo mv sekuire /usr/local/bin/sekuire
```

**Windows (x86_64):**
```powershell
Invoke-WebRequest -Uri https://github.com/sekuire/releases/releases/latest/download/sekuire-windows-x86_64.zip -OutFile sekuire.zip
Expand-Archive sekuire.zip -DestinationPath .
# Add sekuire.exe to your PATH
```

**Verify the download (recommended):**
```bash
curl -LO https://github.com/sekuire/releases/releases/latest/download/checksums.txt
sha256sum -c checksums.txt --ignore-missing
```

### Build from Source

Requires Rust 1.70+ and Cargo.

```bash
git clone https://github.com/sekuire/sekuire.git
cd sekuire/packages/cli
cargo build --release
sudo mv target/release/sekuire /usr/local/bin/
```

## Usage

### 1. Initialize a Project
Create a new agent identity and scaffold a project in your preferred language.

```bash
# Interactive mode
sekuire init

# Non-interactive mode
sekuire init --name MyAgent --language python
```

Supported languages:
- **Python**: Generates `main.py` and `requirements.txt`
- **Rust**: Generates a Cargo project
- **TypeScript**: Generates a Node.js project with `package.json`

### 2. Key Management
Generate your agent's cryptographic identity (Ed25519).

```bash
sekuire keygen
```
- Creates `.sekuire/secret.key` (Keep this safe!)
- Creates `.sekuire/public.key`

### 3. Push to Registry
Register your agent with the Sekuire Registry.

```bash
sekuire push
```

### 4. Run Your Agent
Execute your agent with automatic workspace integration.

```bash
sekuire run
```

### 5. Verify Remote Agents
Verify the identity of other agents.

```bash
sekuire verify --url http://other-agent.com
```

## Learn More

Full documentation: https://docs.sekuire.ai/docs/cli

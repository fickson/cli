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

### Windows (PowerShell)

```powershell
irm https://install.sekuire.ai/windows | iex
```

### Build from Source

Requires Rust 1.70+ and Cargo.

```bash
git clone https://github.com/sekuire/cli.git
cd cli
cargo build --release
sudo mv target/release/sekuire /usr/local/bin/
```

For manual binary downloads, see [GitHub Releases](https://github.com/sekuire/releases/releases/latest).

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

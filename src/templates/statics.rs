pub const PYTHON_REQUIREMENTS: &str = r#"sekuire-sdk>=0.1.0
"#;

pub const PYTHON_MAIN: &str = r#"#!/usr/bin/env python3
"""
Sekuire Python Agent - Config-First

This agent uses the new config-first SDK approach.
Configuration is loaded from sekuire.yml.
"""

import asyncio
import sys
from pathlib import Path

try:
    from sekuire_sdk import get_agent
except ImportError:
    print("❌ sekuire-sdk not installed")
    print("   Run: pip install sekuire-sdk")
    sys.exit(1)


async def main():
    """Main agent execution."""
    print("🤖 Sekuire Python Agent\n")

    # Load agent from sekuire.yml
    try:
        agent = await get_agent()
        print(f"✅ Agent loaded: {agent.name}")
        print(f"   Provider: {agent.get_llm_provider()}")
        print(f"   Model: {agent.get_model_name()}")
        if agent.get_tools():
            print(f"   Tools: {', '.join(agent.get_tools())}")
        print()
    except FileNotFoundError:
        print("❌ sekuire.yml not found")
        print("   Make sure you're in the project directory")
        return 1
    except Exception as e:
        print(f"❌ Failed to load agent: {e}")
        print("\nTroubleshooting:")
        print("  1. Check that sekuire.yml exists")
        print("  2. Set API key in .env (copy from .env.example)")
        print("  3. Verify Python SDK is installed")
        return 1

    # Interactive chat loop
    print("💬 Chat with your agent (type 'quit' to exit)\n")

    while True:
        try:
            user_input = input("You: ").strip()

            if not user_input:
                continue

            if user_input.lower() in ['quit', 'exit', 'q']:
                print("\n👋 Goodbye!")
                break

            # Get response
            print("Agent: ", end="", flush=True)
            response = await agent.chat(user_input)
            print(response)
            print()

        except KeyboardInterrupt:
            print("\n\n👋 Goodbye!")
            break
        except Exception as e:
            print(f"\n❌ Error: {e}\n")

    return 0


if __name__ == "__main__":
    sys.exit(asyncio.run(main()))
"#;

pub fn get_rust_cargo(project_name: &str) -> String {
    let package_name = project_name.to_lowercase().replace(" ", "-");
    format!(
        r#"[workspace]
# Empty workspace to prevent parent workspace from affecting this package

[package]
name = "{}"
version = "0.1.0"
edition = "2021"

[dependencies]
sekuire-sdk = {{ path = "../../packages/sdk" }}
tokio = {{ version = "1.0", features = ["full"] }}
serde = {{ version = "1.0", features = ["derive"] }}
serde_json = "1.0"
anyhow = "1.0"
"#,
        package_name
    )
}

pub const RUST_MAIN: &str = r#"use sekuire_sdk::get_agent;
use std::io::{self, Write};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("🤖 Sekuire Rust Agent\n");

    // Load agent from sekuire.yml
    let mut agent = match get_agent(None, None).await {
        Ok(agent) => {
            println!("✅ Agent loaded: {}", agent.name);
            println!("   Provider: {}", agent.get_llm_provider());
            println!("   Model: {}", agent.get_model_name());
            if !agent.get_tools().is_empty() {
                println!("   Tools: {}", agent.get_tools().join(", "));
            }
            println!();
            agent
        }
        Err(e) => {
            eprintln!("❌ Failed to load agent: {}", e);
            eprintln!("\nTroubleshooting:");
            eprintln!("  1. Check that sekuire.yml exists");
            eprintln!("  2. Set API key in .env (copy from .env.example)");
            eprintln!("  3. Run: cargo build");
            return Err(e.into());
        }
    };

    // Interactive chat loop
    println!("💬 Chat with your agent (type 'quit' to exit)\n");

    loop {
        print!("You: ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        if input.is_empty() {
            continue;
        }

        if input.eq_ignore_ascii_case("quit") || input.eq_ignore_ascii_case("exit") {
            println!("\n👋 Goodbye!");
            break;
        }

        // Get response
        match agent.chat(input, None).await {
            Ok(response) => {
                println!("\nAgent: {}\n", response);
            }
            Err(e) => {
                eprintln!("\n❌ Error: {}\n", e);
            }
        }
    }

    Ok(())
}
"#;

pub fn get_ts_package(project_name: &str) -> String {
    let package_name = project_name.to_lowercase().replace(" ", "-");
    format!(
        r#"{{
  "name": "{}",
  "version": "0.1.0",
  "description": "Created via sekuire init",
  "type": "module",
  "scripts": {{
    "build": "tsc",
    "start": "node dist/index.js"
  }},
  "dependencies": {{
    "@sekuire/sdk": "latest"
  }},
  "devDependencies": {{
    "@types/node": "^22.0.0",
    "typescript": "^5.0.0"
  }}
}}"#,
        package_name
    )
}

pub const TS_CONFIG: &str = r#"{
  "compilerOptions": {
    "target": "ES2022",
    "module": "NodeNext",
    "moduleResolution": "NodeNext",
    "outDir": "./dist",
    "rootDir": "./src",
    "strict": true,
    "esModuleInterop": true,
    "skipLibCheck": true,
    "forceConsistentCasingInFileNames": true
  },
  "include": ["src/**/*"],
  "exclude": ["node_modules"]
}"#;

pub const TS_INDEX: &str = r#"import { getAgent } from "@sekuire/sdk";
import * as readline from "readline";

async function main() {
    console.log("🤖 Sekuire TypeScript Agent\n");

    // Load agent from sekuire.yml
    let agent;
    try {
        agent = await getAgent();
        console.log(`✅ Agent loaded: ${agent.name}`);
        console.log(`   Provider: ${agent.getLLMProvider()}`);
        console.log(`   Model: ${agent.getModelName()}`);
        if (agent.getTools().length > 0) {
            console.log(`   Tools: ${agent.getTools().join(", ")}`);
        }
        console.log();
    } catch (error) {
        console.error("❌ Failed to load agent:", error);
        console.error("\nTroubleshooting:");
        console.error("  1. Check that sekuire.yml exists");
        console.error("  2. Set API key in .env (copy from .env.example)");
        console.error("  3. Run: npm install");
        process.exit(1);
    }

    // Interactive chat loop
    console.log("💬 Chat with your agent (type 'quit' to exit)\n");

    const rl = readline.createInterface({
        input: process.stdin,
        output: process.stdout
    });

    const askQuestion = (query: string): Promise<string> => {
        return new Promise(resolve => rl.question(query, resolve));
    };

    while (true) {
        try {
            const input = await askQuestion("You: ");

            if (!input.trim()) {
                continue;
            }

            if (input.trim().toLowerCase() === "quit" || input.trim().toLowerCase() === "exit") {
                console.log("\n👋 Goodbye!");
                rl.close();
                break;
            }

            // Get response
            const response = await agent.chat(input);
            console.log(`\nAgent: ${response}\n`);

        } catch (error) {
            console.error(`\n❌ Error: ${error}\n`);
        }
    }
}

main();
"#;

#[allow(dead_code)]
pub fn get_sekuire_json(name: &str) -> String {
    format!(
        r#"{{
  "project": {{
    "name": "{}",
    "version": "0.1.0",
    "description": "Created via sekuire init",
    "authors": []
  }},
  "identity": {{
    "model": "gpt-4-turbo",
    "system_prompt_path": "./system_prompt.md",
    "tools_path": "./tools.json"
  }}
}}"#,
        name
    )
}

// NEW: YAML-based configuration
// Multi-LLM template with 4 agent variants
pub fn get_sekuire_yml_multi_llm(name: &str) -> String {
    format!(
        r#"# Sekuire Configuration (Multi-LLM Template)
# Generated by: sekuire init --template multi-llm

# Project metadata
project:
  name: "{}"
  version: "0.1.0"
  description: "Multi-LLM compliant AI agent"
  author: ""
  license: "MIT"

# Identity Configuration (Required for Verification)
# Default identity for this project (maps to the OpenAI variant)
identity:
  model: "gpt-4"
  system_prompt_path: "./prompts/openai.md"
  tools_path: "./tools/openai.json"

# Installation / deployment metadata (displayed in the dashboard)
deployment:
  docker_image: "sekuire/agent:{}"
  notes: "Populate required provider keys below; include tool-specific vars when you add external integrations."
  env:
    - name: "OPENAI_API_KEY"
      required: true
      secret: true
      description: "OpenAI API key."
      example: "sk-..."
    - name: "ANTHROPIC_API_KEY"
      required: false
      secret: true
      description: "Anthropic API key."
      example: "sk-ant-..."
    - name: "GOOGLE_API_KEY"
      required: false
      secret: true
      description: "Google API key (Gemini)."
      example: "..."
    - name: "SEKUIRE_AGENT_ID"
      required: false
      secret: false
      description: "Agent id used for handshake/server."
      default_value: "{}"
    - name: "PORT"
      required: false
      secret: false
      description: "HTTP port (if you enable server mode)."
      default_value: "8000"
    - name: "HOST"
      required: false
      secret: false
      description: "Bind address."
      default_value: "0.0.0.0"

# Multiple agent definitions, each optimized for specific LLM
agents:
  # OpenAI variant - Optimized for reasoning
  {}-openai:
    name: "{} (OpenAI)"
    system_prompt: "./prompts/openai.md"
    tools: "./tools/openai.json"
    llm:
      provider: "openai"
      model: "gpt-4"
      api_key_env: "OPENAI_API_KEY"
      temperature: 0.7
      max_tokens: 2000
    memory:
      type: "buffer"
      max_messages: 10
    compliance:
      framework: "hipaa"
      audit_logging: true

  # Claude variant - Optimized for document analysis
  {}-claude:
    name: "{} (Claude)"
    system_prompt: "./prompts/claude.md"
    tools: "./tools/claude.json"
    llm:
      provider: "anthropic"
      model: "claude-3-opus-20240229"
      api_key_env: "ANTHROPIC_API_KEY"
      temperature: 0.9
      max_tokens: 2000
    memory:
      type: "summary"
      max_messages: 20
    compliance:
      framework: "hipaa"
      audit_logging: true

  # Gemini variant - Optimized for multimodal
  {}-gemini:
    name: "{} (Gemini)"
    system_prompt: "./prompts/gemini.md"
    tools: "./tools/gemini.json"
    llm:
      provider: "google"
      model: "gemini-pro"
      api_key_env: "GOOGLE_API_KEY"
      temperature: 0.5
      max_tokens: 2000
    memory:
      type: "buffer"
      max_messages: 5
    compliance:
      framework: "hipaa"
      audit_logging: true

  # Ollama variant - Optimized for local/privacy
  {}-local:
    name: "{} (Local)"
    system_prompt: "./prompts/ollama.md"
    tools: "./tools/ollama.json"
    llm:
      provider: "ollama"
      model: "llama2"
      base_url: "http://localhost:11434"
      temperature: 0.6
    memory:
      type: "buffer"
      max_messages: 5
    compliance:
      framework: "hipaa"
      audit_logging: true

# Shared logger configuration
logger:
  api_base_url: "https://api.sekuire.ai"
  environment: "development"
  enabled: true
"#,
        name, name, name, name, name, name, name, name, name, name, name
    )
}

pub fn get_sekuire_yml(name: &str, tools: &[&str], memory: Option<(&str, &str)>) -> String {
    let tools_section = if tools.is_empty() {
        "  # No built-in tools selected\n  allowed_tools: []".to_string()
    } else {
        format!(
            "  # Built-in tools\n  allowed_tools:\n{}",
            tools
                .iter()
                .map(|t| format!("      - {}", t))
                .collect::<Vec<_>>()
                .join("\n")
        )
    };

    let memory_section = match memory {
        Some(("redis", url)) => format!(
            r#"  # Memory configuration - Redis
  memory:
    type: "redis"
    config:
      url: "{}"  # Override via REDIS_URL env var"#,
            url
        ),
        Some(("postgres", url)) => format!(
            r#"  # Memory configuration - PostgreSQL
  memory:
    type: "postgres"
    config:
      url: "{}"  # Override via DATABASE_URL env var"#,
            url
        ),
        Some(("file", path)) => format!(
            r#"  # Memory configuration - File-based
  memory:
    type: "file"
    config:
      path: "{}""#,
            path
        ),
        _ => r#"  # Memory configuration - In-Memory (default)
  memory:
    type: "buffer"
    max_messages: 10"#
            .to_string(),
    };

    format!(
        r#"# Sekuire Configuration
# Generated by: sekuire init

# Project metadata
project:
  name: "{name}"
  version: "0.1.0"
  description: "My compliant AI agent"
  author: ""
  license: "MIT"

# Identity Configuration (Required for Verification)
identity:
  model: "gpt-4"
  system_prompt_path: "./system_prompt.md"
  tools_path: "./tools.json"

# Installation / deployment metadata (displayed in the dashboard)
deployment:
  docker_image: "sekuire/agent:{name}"
  notes: "List the environment variables this agent needs so enterprises can review and install safely."
  env:
    - name: "OPENAI_API_KEY"
      required: true
      secret: true
      description: "OpenAI API key for the agent LLM."
      example: "sk-..."
    - name: "OPENAI_MODEL"
      required: false
      secret: false
      description: "LLM model name."
      default_value: "gpt-4o-mini"
    - name: "SEKUIRE_AGENT_ID"
      required: false
      secret: false
      description: "Agent id used for handshake/server."
      default_value: "{name}"
    - name: "PORT"
      required: false
      secret: false
      description: "HTTP port (if you enable server mode)."
      default_value: "8000"
    - name: "HOST"
      required: false
      secret: false
      description: "Bind address."
      default_value: "0.0.0.0"

# LLM Configuration
# You can use environment variables for dynamic configuration
# Example: provider: "${{LLM_PROVIDER:-openai}}" uses env var with fallback
llm:
  provider: "openai"           # openai, anthropic, ollama, google
  model: "gpt-4"               # Model name
  api_key_env: "OPENAI_API_KEY" # Environment variable for API key
  temperature: 0.7             # Creativity (0.0 - 1.0)
  max_tokens: 2000             # Max response length
  streaming: false             # Enable streaming responses

# Agent Configuration
agent:
  # System prompt file
  system_prompt: "./system_prompt.md"

  # Tools configuration
  tools: "./tools.json"

{tools_section}

{memory_section}

  # Compliance configuration
  compliance:
    framework: "hipaa"         # hipaa, soc2, gdpr, pci-dss, none
    audit_logging: true        # Log all actions
    sensitive_data_detection: true
    require_approval:          # Tools requiring human approval
      - "send_email"
      - "delete_data"

  # Network permissions
  network:
    allow:
      - "https://api.openai.com"
      - "https://api.anthropic.com"
      - "https://api.duckduckgo.com"
    block: []

  # Filesystem permissions
  filesystem:
    allow_read:
      - "./data/"
    allow_write:
      - "./logs/"
    block_all:
      - "/etc/"
      - "/var/"

# Logger configuration
logger:
  api_base_url: "https://api.sekuire.ai"
  environment: "development"
  enabled: true

# Multi-LLM Support (Optional)
# Uncomment to define multiple agent variants with different LLMs
# agents:
#   my-agent-openai:
#     name: "My Agent (OpenAI)"
#     system_prompt: "./system_prompt.md"
#     tools: "./tools.json"
#     llm:
#       provider: "openai"
#       model: "gpt-4"
#       api_key_env: "OPENAI_API_KEY"
#
#   my-agent-claude:
#     name: "My Agent (Claude)"
#     system_prompt: "./system_prompt.md"
#     tools: "./tools.json"
#     llm:
#       provider: "anthropic"
#       model: "claude-3-opus-20240229"
#       api_key_env: "ANTHROPIC_API_KEY"
#
#   my-agent-gemini:
#     name: "My Agent (Gemini)"
#     system_prompt: "./system_prompt.md"
#     tools: "./tools.json"
#     llm:
#       provider: "google"
#       model: "gemini-pro"
#       api_key_env: "GOOGLE_API_KEY"
#
#   my-agent-local:
#     name: "My Agent (Local)"
#     system_prompt: "./system_prompt.md"
#     tools: "./tools.json"
#     llm:
#       provider: "ollama"
#       model: "llama2"
#       base_url: "http://localhost:11434"
"#,
        name = name,
        tools_section = tools_section,
        memory_section = memory_section
    )
}

pub fn get_system_prompt() -> &'static str {
    r#"# My Agent

You are a helpful AI assistant built with Sekuire - the trust protocol for AI agents.

## Your Capabilities

- **Answer Questions**: Provide accurate, helpful information
- **Perform Calculations**: Use the calculator tool for math
- **Search the Web**: Use the web search tool for current information
- **Verify Compliance**: Check your verification status in the Sekuire registry

## Your Tools

You have access to the following tools:

1. **calculator** - Perform mathematical calculations
   - Use for: Math problems, computations, data analysis
   - Example: "What is 15% of 250?"

2. **web_search** - Search the web using DuckDuckGo
   - Use for: Current events, factual lookups, research
   - Example: "Search for latest AI developments"

3. **get_verification_status** ⚠️ SYSTEM TOOL - DO NOT REMOVE
   - Check your verification and compliance status in the Sekuire registry
   - Use for: Checking if you're verified, viewing compliance scores
   - Example: "Am I verified?" or "Check my compliance status"
   - This tool is REQUIRED for trust protocol compliance

## Your Rules

1. **Be Accurate**: Always provide correct information
2. **Be Concise**: Keep responses clear and to the point
3. **Be Helpful**: Focus on solving the user's problem
4. **Cite Sources**: When using web search, cite the source
5. **Ask for Clarification**: If something is unclear, ask questions
6. **Respect Privacy**: Never ask for or store personal information
7. **Maintain Verification**: The verification status tool must never be disabled or removed

## Compliance

- All your actions are logged for audit purposes
- Some operations require human approval
- Sensitive data detection is enabled
- You operate under HIPAA compliance policies
- Your verification status is regularly checked via the Sekuire registry

## Response Format

Structure your responses like this:

1. **Thought**: Briefly explain your reasoning
2. **Action**: State which tool you'll use (if any)
3. **Answer**: Provide the final response

---

Remember: You are a trusted, compliant AI agent. Every action you take is verified and logged. Your verification status is tracked in the Sekuire registry.
"#
}

pub fn get_tools_json() -> &'static str {
    r#"{
  "version": "1.0.0",
  "tools": [
    {
      "name": "calculator",
      "description": "Perform mathematical calculations",
      "category": "data",
      "enabled": true,
      "schema": {
        "type": "object",
        "properties": {
          "expression": {
            "type": "string",
            "description": "Mathematical expression to evaluate"
          }
        },
        "required": ["expression"]
      },
      "implementation": "./tools/calculator.ts",
      "permissions": {
        "network": [],
        "filesystem": []
      },
      "compliance": {
        "require_approval": false,
        "log_input": true,
        "log_output": true
      }
    },
    {
      "name": "web_search",
      "description": "Search the web using DuckDuckGo",
      "category": "web",
      "enabled": true,
      "schema": {
        "type": "object",
        "properties": {
          "query": {
            "type": "string",
            "description": "Search query"
          },
          "max_results": {
            "type": "number",
            "description": "Maximum number of results",
            "default": 5
          }
        },
        "required": ["query"]
      },
      "implementation": "./tools/search.ts",
      "permissions": {
        "network": ["https://api.duckduckgo.com"]
      },
      "compliance": {
        "require_approval": false,
        "log_input": true,
        "log_output": true
      }
    },
    {
      "name": "get_verification_status",
      "description": "⚠️ CRITICAL SYSTEM TOOL - DO NOT EDIT OR DELETE ⚠️ Check the verification and compliance status of this agent in the Sekuire registry. This tool is required for trust protocol compliance and must not be removed.",
      "category": "compliance",
      "enabled": true,
      "system_tool": true,
      "schema": {
        "type": "object",
        "properties": {
          "sekuire_id": {
            "type": "string",
            "description": "The Sekuire ID of the agent to check (defaults to current agent)"
          },
          "include_details": {
            "type": "boolean",
            "description": "Include detailed verification information",
            "default": false
          }
        },
        "required": []
      },
      "implementation": "builtin",
      "permissions": {
        "network": ["https://api.sekuire.ai"],
        "filesystem": []
      },
      "compliance": {
        "require_approval": false,
        "log_input": true,
        "log_output": true,
        "compliance_level": "system"
      }
    }
  ]
}
"#
}

pub fn get_env_example() -> &'static str {
    r#"# Sekuire Agent Configuration
# Copy this file to .env and fill in your API keys

# LLM Provider Selection (Optional - overrides sekuire.yml)
# LLM_PROVIDER=openai
# LLM_MODEL=gpt-4

# LLM Provider API Keys
OPENAI_API_KEY=sk-...
ANTHROPIC_API_KEY=sk-ant-...
GOOGLE_API_KEY=...

# For Ollama (local models)
# OLLAMA_BASE_URL=http://localhost:11434

# Sekuire Registry
SEKUIRE_REGISTRY_URL=https://api.sekuire.ai
SEKUIRE_WEB_URL=https://sekuire.ai

# Environment
NODE_ENV=development
PORT=8000

# Logging
LOG_LEVEL=info
"#
}

pub fn get_readme(name: &str) -> String {
    format!(
        r#"# {}

A compliant AI agent built with Sekuire.

## Quick Start

```bash
# Install dependencies
npm install

# Configure environment
cp .env.example .env
# Edit .env and add your API keys

# Run agent
npm start
```

## Configuration

Agent configuration is in `sekuire.yml`. Key settings:

- **LLM**: Currently using gpt-4 (OpenAI)
- **Compliance**: HIPAA framework enabled
- **Tools**: calculator, web_search

## Tools

### calculator
Perform mathematical calculations.

### web_search
Search the web using DuckDuckGo.

## Development

```bash
# Test agent
npm test

# Validate configuration
sekuire validate

# Calculate agent ID
sekuire hash
```

## Deployment

```bash
# Login to Sekuire
sekuire login

# Push agent (draft)
sekuire push
```

## Compliance

This agent is configured with:
- ✅ HIPAA compliance framework
- ✅ Audit logging enabled
- ✅ Sensitive data detection
- ✅ Tool whitelisting

All actions are logged and can be audited.

## Learn More

- [Sekuire Documentation](https://docs.sekuire.ai)
- [Trust Protocol](https://docs.sekuire.ai/trust-protocol)
- [Compliance Guide](https://docs.sekuire.ai/compliance)
"#,
        name
    )
}

pub fn get_calculator_tool_ts() -> &'static str {
    r#"import { tool } from "@sekuire/sdk";
import { z } from "zod";

export const calculatorTool = tool({
  name: "calculator",
  description: "Perform mathematical calculations",
  schema: z.object({
    expression: z.string().describe("The mathematical expression to evaluate"),
  }),
  execute: async ({ expression }) => {
    try {
      // Note: eval() is used for demo purposes only. Use a math parser in production.
      const result = eval(expression);
      return `Result: ${result}`;
    } catch (error) {
      return `Error: Invalid expression`;
    }
  },
});

export default calculatorTool;
"#
}

// LLM-specific prompts for multi-llm template
pub fn get_openai_prompt() -> &'static str {
    r#"# AI Assistant (OpenAI GPT-4)

You are an advanced AI assistant powered by GPT-4, optimized for reasoning and problem-solving.

## Your Strengths
- Complex reasoning and logic
- Code generation and debugging
- Mathematical calculations
- Structured thinking

## Available Tools

1. **calculator** - Perform mathematical calculations
2. **code_executor** - Execute code snippets
3. **web_search** - Search the web for current information

## Response Format

1. **Analysis**: Break down the problem
2. **Reasoning**: Think through each step
3. **Tool Usage**: If needed, use appropriate tools
4. **Conclusion**: Provide clear answer

---

Remember: You excel at step-by-step reasoning. Break down complex problems.
"#
}

pub fn get_claude_prompt() -> &'static str {
    r#"# AI Assistant (Anthropic Claude)

You are Claude, an AI assistant created by Anthropic, optimized for document analysis and thoughtful conversation.

## Your Strengths
- Long document comprehension (100K+ tokens)
- Nuanced writing and analysis
- Creative problem-solving
- Ethical reasoning

## Available Tools

1. **file_reader** - Read and analyze files
2. **web_scraper** - Extract content from websites
3. **document_analyzer** - Analyze document structure and content

## Response Style

Think out loud through your reasoning, then provide your response.

---

Remember: You excel at thorough analysis. Take your time to think through problems.
"#
}

pub fn get_gemini_prompt() -> &'static str {
    r#"# AI Assistant (Google Gemini)

You are Gemini, a multimodal AI assistant by Google, optimized for working with images, video, and real-time data.

## Your Strengths
- Image and video understanding
- Real-time information access
- Multilingual capabilities
- Fast responses

## Available Tools

1. **image_analyzer** - Analyze images for objects, text, scenes
2. **video_processor** - Process and analyze video content
3. **maps_api** - Access Google Maps data
4. **realtime_search** - Search for current information

## Response Style

Be direct and concise. Understand task, use tool, provide result.

---

Remember: You excel at multimodal tasks. Be fast and accurate.
"#
}

pub fn get_ollama_prompt() -> &'static str {
    r#"# AI Assistant (Local)

You are a helpful AI assistant running locally for privacy and offline access.

## Available Tools

- calculator: Math calculations
- file_ops: Read/write files locally
- sql_query: Query local databases

## Instructions

1. Keep responses concise
2. Use simple language
3. Use tools when needed
4. Be accurate

---

Remember: Privacy-focused, offline-capable, keep it simple.
"#
}

pub fn get_search_tool_ts() -> &'static str {
    r#"import { tool } from "@sekuire/sdk";
import { z } from "zod";

export const searchTool = tool({
  name: "web_search",
  description: "Search the web for information",
  schema: z.object({
    query: z.string().describe("The search query"),
  }),
  execute: async ({ query }) => {
    // Mock implementation for demo
    return `[Mock Search Results] found for: "${query}"\n1. Sekuire Documentation - https://docs.sekuire.ai\n2. Sekuire GitHub - https://github.com/sekuire/sekuire`;
  },
});

export default searchTool;
"#
}

// ========================================
// RUST TEMPLATES - LLM ENABLED
// ========================================

pub fn get_rust_cargo_multi_llm(project_name: &str) -> String {
    let package_name = project_name.to_lowercase().replace(" ", "-");
    format!(
        r#"[package]
name = "{}"
version = "0.1.0"
edition = "2021"

[dependencies]
sekuire-sdk = "0.1.0"
tokio = {{ version = "1.0", features = ["full"] }}
serde = {{ version = "1.0", features = ["derive"] }}
serde_json = "1.0"
serde_yaml = "0.9"
reqwest = {{ version = "0.11", features = ["json"] }}
anyhow = "1.0"

# LLM Provider SDKs (optional - enable as needed)
async-openai = {{ version = "0.18", optional = true }}
anthropic-sdk = {{ version = "0.2", optional = true }}

[features]
default = []
openai = ["async-openai"]
anthropic = ["anthropic-sdk"]
"#,
        package_name
    )
}

pub const RUST_MAIN_MULTI_LLM: &str = r#"use sekuire_sdk::AgentIdentity;
use serde::{Deserialize, Serialize};
use serde_yaml;
use std::error::Error;
use std::fs;

#[derive(Debug, Deserialize)]
struct SekuireConfig {
    project: ProjectConfig,
    agents: std::collections::HashMap<String, AgentConfig>,
}

#[derive(Debug, Deserialize)]
struct ProjectConfig {
    name: String,
    version: String,
}

#[derive(Debug, Deserialize)]
struct AgentConfig {
    name: String,
    system_prompt: String,
    tools: String,
    llm: LLMConfig,
    memory: MemoryConfig,
}

#[derive(Debug, Deserialize)]
struct LLMConfig {
    provider: String,
    model: String,
    api_key_env: String,
    temperature: f32,
    max_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct MemoryConfig {
    r#type: String,
    max_messages: u32,
}

#[derive(Debug, Serialize)]
struct Message {
    role: String,
    content: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // 1. Load Sekuire configuration
    let config_content = fs::read_to_string("sekuire.yml")?;
    let config: SekuireConfig = serde_yaml::from_str(&config_content)?;

    println!("🚀 Loading Sekuire Agent: {}", config.project.name);

    // 2. Select agent (default to first agent)
    let agent_name = std::env::args().nth(1)
        .or_else(|| config.agents.keys().next().cloned())
        .ok_or("No agent specified or found in config")?;

    let agent_config = config.agents.get(&agent_name)
        .ok_or(format!("Agent '{}' not found in config", agent_name))?;

    println!("🤖 Using agent: {}", agent_config.name);
    println!("🧠 Model: {}", agent_config.llm.model);

    // 3. Load system prompt
    let system_prompt = fs::read_to_string(&agent_config.system_prompt)?;

    // 4. Load tools schema
    let tools_schema = fs::read_to_string(&agent_config.tools)?;

    // 5. Initialize Sekuire identity (for signing/verification)
    let identity = AgentIdentity::load().await?;
    println!("🆔 Sekuire ID: {}", identity.sekuire_id());

    // 6. Initialize conversation
    let mut conversation: Vec<Message> = vec![
        Message {
            role: "system".to_string(),
            content: system_prompt,
        }
    ];

    // 7. Run agent loop
    println!("\n💬 Agent ready! Type 'exit' to quit.\n");

    loop {
        print!("You: ");
        use std::io::{self, Write};
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        if input == "exit" {
            println!("👋 Goodbye!");
            break;
        }

        conversation.push(Message {
            role: "user".to_string(),
            content: input.to_string(),
        });

        // Call LLM
        let response = call_llm(&agent_config.llm, &conversation).await?;

        conversation.push(Message {
            role: "assistant".to_string(),
            content: response.clone(),
        });

        // Manage memory
        manage_memory(&mut conversation, &agent_config.memory);

        println!("\nAssistant: {}\n", response);
    }

    Ok(())
}

async fn call_llm(llm_config: &LLMConfig, messages: &[Message]) -> Result<String, Box<dyn Error>> {
    let api_key = std::env::var(&llm_config.api_key_env)?;

    match llm_config.provider.as_str() {
        "openai" => call_openai(llm_config, messages, &api_key).await,
        "anthropic" => call_anthropic(llm_config, messages, &api_key).await,
        "google" => call_google(llm_config, messages, &api_key).await,
        "ollama" => call_ollama(llm_config, messages).await,
        _ => Err(format!("Unknown provider: {}", llm_config.provider).into()),
    }
}

async fn call_openai(
    config: &LLMConfig,
    messages: &[Message],
    api_key: &str,
) -> Result<String, Box<dyn Error>> {
    let client = reqwest::Client::new();

    let response = client
        .post("https://api.openai.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&serde_json::json!({
            "model": config.model,
            "messages": messages,
            "temperature": config.temperature,
            "max_tokens": config.max_tokens,
        }))
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;

    let content = response["choices"][0]["message"]["content"]
        .as_str()
        .ok_or("Failed to parse OpenAI response")?;

    Ok(content.to_string())
}

async fn call_anthropic(
    config: &LLMConfig,
    messages: &[Message],
    api_key: &str,
) -> Result<String, Box<dyn Error>> {
    let client = reqwest::Client::new();

    // Extract system message
    let system_msg = messages.iter()
        .find(|m| m.role == "system")
        .map(|m| m.content.clone())
        .unwrap_or_default();

    let user_messages: Vec<_> = messages.iter()
        .filter(|m| m.role != "system")
        .collect();

    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .json(&serde_json::json!({
            "model": config.model,
            "system": system_msg,
            "messages": user_messages,
            "max_tokens": config.max_tokens,
            "temperature": config.temperature,
        }))
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;

    let content = response["content"][0]["text"]
        .as_str()
        .ok_or("Failed to parse Anthropic response")?;

    Ok(content.to_string())
}

async fn call_google(
    config: &LLMConfig,
    messages: &[Message],
    api_key: &str,
) -> Result<String, Box<dyn Error>> {
    let client = reqwest::Client::new();

    let response = client
        .post(format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            config.model, api_key
        ))
        .json(&serde_json::json!({
            "contents": messages.iter().filter(|m| m.role != "system").map(|m| {
                serde_json::json!({
                    "role": if m.role == "assistant" { "model" } else { "user" },
                    "parts": [{"text": m.content}]
                })
            }).collect::<Vec<_>>(),
            "generationConfig": {
                "temperature": config.temperature,
                "maxOutputTokens": config.max_tokens,
            }
        }))
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;

    let content = response["candidates"][0]["content"]["parts"][0]["text"]
        .as_str()
        .ok_or("Failed to parse Google response")?;

    Ok(content.to_string())
}

async fn call_ollama(
    config: &LLMConfig,
    messages: &[Message],
) -> Result<String, Box<dyn Error>> {
    let client = reqwest::Client::new();

    let response = client
        .post("http://localhost:11434/api/chat")
        .json(&serde_json::json!({
            "model": config.model,
            "messages": messages,
            "stream": false,
            "options": {
                "temperature": config.temperature,
                "num_predict": config.max_tokens,
            }
        }))
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;

    let content = response["message"]["content"]
        .as_str()
        .ok_or("Failed to parse Ollama response")?;

    Ok(content.to_string())
}

fn manage_memory(conversation: &mut Vec<Message>, memory_config: &MemoryConfig) {
    let system_msgs: Vec<_> = conversation.iter()
        .filter(|m| m.role == "system")
        .cloned()
        .collect();

    let other_msgs: Vec<_> = conversation.iter()
        .filter(|m| m.role != "system")
        .cloned()
        .collect();

    let max_messages = memory_config.max_messages as usize;

    match memory_config.r#type.as_str() {
        "buffer" => {
            // Keep only last N messages
            if other_msgs.len() > max_messages {
                let kept_msgs = &other_msgs[other_msgs.len() - max_messages..];
                *conversation = system_msgs.into_iter()
                    .chain(kept_msgs.iter().cloned())
                    .collect();
            }
        }
        "summary" => {
            // TODO: Implement summarization logic
            // For now, just truncate
            if other_msgs.len() > max_messages {
                let kept_msgs = &other_msgs[other_msgs.len() - max_messages..];
                *conversation = system_msgs.into_iter()
                    .chain(kept_msgs.iter().cloned())
                    .collect();
            }
        }
        _ => {}
    }
}
"#;

// ========================================
// PYTHON TEMPLATES - LLM ENABLED
// ========================================

pub const PYTHON_REQUIREMENTS_MULTI_LLM: &str = r#"sekuire-sdk>=0.1.0
pyyaml>=6.0
openai>=1.0.0
anthropic>=0.8.0
google-generativeai>=0.3.0
requests>=2.31.0
"#;

pub const PYTHON_MAIN_MULTI_LLM: &str = r#"#!/usr/bin/env python3
"""
Sekuire Multi-LLM Agent
Secure, compliant AI agent with support for multiple LLM providers.
"""

import os
import sys
import yaml
import json
from typing import List, Dict, Any
from sekuire_sdk import AgentIdentity

# LLM Provider Imports
try:
    import openai
except ImportError:
    openai = None

try:
    import anthropic
except ImportError:
    anthropic = None

try:
    import google.generativeai as genai
except ImportError:
    genai = None

import requests


class Message:
    def __init__(self, role: str, content: str):
        self.role = role
        self.content = content

    def to_dict(self):
        return {"role": self.role, "content": self.content}


class SekuireAgent:
    def __init__(self, config_path: str = "sekuire.yml"):
        # Load configuration
        with open(config_path, 'r') as f:
            self.config = yaml.safe_load(f)

        # Load identity
        try:
            self.identity = AgentIdentity.load()
            print(f"✅ Agent '{self.identity.name}' initialized.")
            print(f"🆔 Sekuire ID: {self.identity.sekuire_id}")
        except Exception as e:
            print(f"❌ Failed to load identity: {e}")
            sys.exit(1)

        self.conversation: List[Message] = []

    def select_agent(self, agent_name: str = None):
        """Select which agent configuration to use"""
        agents = self.config.get('agents', {})

        if not agents:
            raise ValueError("No agents defined in sekuire.yml")

        # Use specified agent or first available
        if agent_name is None:
            agent_name = list(agents.keys())[0]

        if agent_name not in agents:
            raise ValueError(f"Agent '{agent_name}' not found in config")

        self.agent_config = agents[agent_name]
        self.agent_name = agent_name

        print(f"🤖 Using agent: {self.agent_config['name']}")
        print(f"🧠 Model: {self.agent_config['llm']['model']}")

        # Load system prompt
        with open(self.agent_config['system_prompt'], 'r') as f:
            system_prompt = f.read()

        self.conversation = [Message("system", system_prompt)]

        # Load tools schema
        with open(self.agent_config['tools'], 'r') as f:
            self.tools_schema = json.load(f)

    def call_llm(self) -> str:
        """Call the configured LLM provider"""
        llm_config = self.agent_config['llm']
        provider = llm_config['provider']

        if provider == "openai":
            return self._call_openai(llm_config)
        elif provider == "anthropic":
            return self._call_anthropic(llm_config)
        elif provider == "google":
            return self._call_google(llm_config)
        elif provider == "ollama":
            return self._call_ollama(llm_config)
        else:
            raise ValueError(f"Unknown provider: {provider}")

    def _call_openai(self, config: Dict[str, Any]) -> str:
        """Call OpenAI API"""
        if openai is None:
            raise ImportError("openai package not installed. Run: pip install openai")

        api_key = os.getenv(config['api_key_env'])
        if not api_key:
            raise ValueError(f"API key not found: {config['api_key_env']}")

        client = openai.OpenAI(api_key=api_key)

        response = client.chat.completions.create(
            model=config['model'],
            messages=[m.to_dict() for m in self.conversation],
            temperature=config['temperature'],
            max_tokens=config['max_tokens']
        )

        return response.choices[0].message.content

    def _call_anthropic(self, config: Dict[str, Any]) -> str:
        """Call Anthropic Claude API"""
        if anthropic is None:
            raise ImportError("anthropic package not installed. Run: pip install anthropic")

        api_key = os.getenv(config['api_key_env'])
        if not api_key:
            raise ValueError(f"API key not found: {config['api_key_env']}")

        client = anthropic.Anthropic(api_key=api_key)

        # Extract system message
        system_msg = next((m.content for m in self.conversation if m.role == "system"), "")
        user_messages = [m.to_dict() for m in self.conversation if m.role != "system"]

        response = client.messages.create(
            model=config['model'],
            system=system_msg,
            messages=user_messages,
            max_tokens=config['max_tokens'],
            temperature=config['temperature']
        )

        return response.content[0].text

    def _call_google(self, config: Dict[str, Any]) -> str:
        """Call Google Gemini API"""
        if genai is None:
            raise ImportError("google-generativeai package not installed. Run: pip install google-generativeai")

        api_key = os.getenv(config['api_key_env'])
        if not api_key:
            raise ValueError(f"API key not found: {config['api_key_env']}")

        genai.configure(api_key=api_key)
        model = genai.GenerativeModel(config['model'])

        # Build chat history (skip system message for Gemini)
        chat_history = [
            {"role": "user" if m.role == "user" else "model", "parts": [m.content]}
            for m in self.conversation if m.role != "system"
        ]

        chat = model.start_chat(history=chat_history[:-1])
        response = chat.send_message(
            chat_history[-1]["parts"][0],
            generation_config={
                "temperature": config['temperature'],
                "max_output_tokens": config['max_tokens']
            }
        )

        return response.text

    def _call_ollama(self, config: Dict[str, Any]) -> str:
        """Call local Ollama API"""
        url = "http://localhost:11434/api/chat"

        payload = {
            "model": config['model'],
            "messages": [m.to_dict() for m in self.conversation],
            "stream": False,
            "options": {
                "temperature": config['temperature'],
                "num_predict": config['max_tokens']
            }
        }

        response = requests.post(url, json=payload)
        response.raise_for_status()

        return response.json()['message']['content']

    def manage_memory(self):
        """Manage conversation memory according to config"""
        memory_config = self.agent_config['memory']
        max_messages = memory_config['max_messages']
        memory_type = memory_config['type']

        # Keep system messages
        system_msgs = [m for m in self.conversation if m.role == "system"]
        other_msgs = [m for m in self.conversation if m.role != "system"]

        if len(other_msgs) > max_messages:
            if memory_type == "buffer":
                # Keep only last N messages
                other_msgs = other_msgs[-max_messages:]
            elif memory_type == "summary":
                # TODO: Implement summarization
                # For now, just truncate
                other_msgs = other_msgs[-max_messages:]

        self.conversation = system_msgs + other_msgs

    def run(self):
        """Run interactive agent loop"""
        print("\n💬 Agent ready! Type 'exit' to quit.\n")

        while True:
            try:
                user_input = input("You: ").strip()

                if user_input.lower() == "exit":
                    print("👋 Goodbye!")
                    break

                if not user_input:
                    continue

                # Add user message
                self.conversation.append(Message("user", user_input))

                # Call LLM
                response = self.call_llm()

                # Add assistant response
                self.conversation.append(Message("assistant", response))

                # Manage memory
                self.manage_memory()

                print(f"\nAssistant: {response}\n")

            except KeyboardInterrupt:
                print("\n\n👋 Goodbye!")
                break
            except Exception as e:
                print(f"\n❌ Error: {e}\n")


def main():
    """Main entry point"""
    agent = SekuireAgent()

    # Get agent name from command line or use default
    agent_name = sys.argv[1] if len(sys.argv) > 1 else None
    agent.select_agent(agent_name)

    # Run agent
    agent.run()


if __name__ == "__main__":
    main()
"#;

// ========================================
// TYPESCRIPT TEMPLATES - LLM ENABLED
// ========================================

pub fn get_ts_package_multi_llm(project_name: &str) -> String {
    let package_name = project_name.to_lowercase().replace(" ", "-");
    format!(
        r#"{{
  "name": "{}",
  "version": "0.1.0",
  "description": "Multi-LLM Sekuire agent",
  "type": "module",
  "scripts": {{
    "build": "tsc",
    "start": "node dist/index.js",
    "dev": "tsx src/index.ts"
  }},
  "dependencies": {{
    "@sekuire/sdk": "latest",
    "openai": "^4.20.0",
    "@anthropic-ai/sdk": "^0.9.1",
    "@google/generative-ai": "^0.1.3",
    "js-yaml": "^4.1.0",
    "dotenv": "^16.3.1"
  }},
  "devDependencies": {{
    "@types/node": "^22.0.0",
    "@types/js-yaml": "^4.0.9",
    "typescript": "^5.0.0",
    "tsx": "^4.7.0"
  }}
}}"#,
        package_name
    )
}

pub const TS_INDEX_MULTI_LLM: &str = r#"/**
 * Sekuire Multi-LLM Agent
 * Secure, compliant AI agent with support for multiple LLM providers.
 */
import fs from 'fs';
import yaml from 'js-yaml';
import dotenv from 'dotenv';
import OpenAI from 'openai';
import Anthropic from '@anthropic-ai/sdk';
import { GoogleGenerativeAI } from '@google/generative-ai';
import { AgentIdentity } from '@sekuire/sdk';
import * as readline from 'readline';

dotenv.config();

interface Message {
    role: string;
    content: string;
}

interface LLMConfig {
    provider: string;
    model: string;
    api_key_env: string;
    temperature: number;
    max_tokens: number;
    base_url?: string;
}

interface MemoryConfig {
    type: string;
    max_messages: number;
}

interface AgentConfig {
    name: string;
    system_prompt: string;
    tools: string;
    llm: LLMConfig;
    memory: MemoryConfig;
}

interface SekuireConfig {
    project: {
        name: string;
        version: string;
    };
    agents: Record<string, AgentConfig>;
}

class SekuireAgent {
    private config: SekuireConfig;
    private identity: AgentIdentity | null = null;
    private conversation: Message[] = [];
    private agentConfig!: AgentConfig;
    private agentName!: string;
    private toolsSchema: any;

    constructor(configPath: string = 'sekuire.yml') {
        // Load configuration
        const configContent = fs.readFileSync(configPath, 'utf8');
        this.config = yaml.load(configContent) as SekuireConfig;
    }

    async initialize() {
        // Load identity
        try {
            this.identity = await AgentIdentity.load();
            console.log(`✅ Agent '${this.identity.name}' initialized.`);
            console.log(`🆔 Sekuire ID: ${this.identity.sekuireId}`);
        } catch (error) {
            console.error('❌ Failed to load identity:', error);
            process.exit(1);
        }
    }

    selectAgent(agentName?: string) {
        const agents = this.config.agents;

        if (!agents || Object.keys(agents).length === 0) {
            throw new Error('No agents defined in sekuire.yml');
        }

        // Use specified agent or first available
        if (!agentName) {
            agentName = Object.keys(agents)[0];
        }

        if (!(agentName in agents)) {
            throw new Error(`Agent '${agentName}' not found in config`);
        }

        this.agentConfig = agents[agentName];
        this.agentName = agentName;

        console.log(`🤖 Using agent: ${this.agentConfig.name}`);
        console.log(`🧠 Model: ${this.agentConfig.llm.model}`);

        // Load system prompt
        const systemPrompt = fs.readFileSync(this.agentConfig.system_prompt, 'utf8');
        this.conversation = [{ role: 'system', content: systemPrompt }];

        // Load tools schema
        this.toolsSchema = JSON.parse(fs.readFileSync(this.agentConfig.tools, 'utf8'));
    }

    async callLLM(): Promise<string> {
        const { provider } = this.agentConfig.llm;

        switch (provider) {
            case 'openai':
                return await this.callOpenAI();
            case 'anthropic':
                return await this.callAnthropic();
            case 'google':
                return await this.callGoogle();
            case 'ollama':
                return await this.callOllama();
            default:
                throw new Error(`Unknown provider: ${provider}`);
        }
    }

    private async callOpenAI(): Promise<string> {
        const config = this.agentConfig.llm;
        const apiKey = process.env[config.api_key_env];

        if (!apiKey) {
            throw new Error(`API key not found: ${config.api_key_env}`);
        }

        const client = new OpenAI({ apiKey });

        const response = await client.chat.completions.create({
            model: config.model,
            messages: this.conversation as any,
            temperature: config.temperature,
            max_tokens: config.max_tokens,
        });

        return response.choices[0].message.content || '';
    }

    private async callAnthropic(): Promise<string> {
        const config = this.agentConfig.llm;
        const apiKey = process.env[config.api_key_env];

        if (!apiKey) {
            throw new Error(`API key not found: ${config.api_key_env}`);
        }

        const client = new Anthropic({ apiKey });

        // Extract system message
        const systemMsg = this.conversation.find(m => m.role === 'system')?.content || '';
        const userMessages = this.conversation.filter(m => m.role !== 'system');

        const response = await client.messages.create({
            model: config.model,
            system: systemMsg,
            messages: userMessages as any,
            max_tokens: config.max_tokens,
            temperature: config.temperature,
        });

        return response.content[0].type === 'text' ? response.content[0].text : '';
    }

    private async callGoogle(): Promise<string> {
        const config = this.agentConfig.llm;
        const apiKey = process.env[config.api_key_env];

        if (!apiKey) {
            throw new Error(`API key not found: ${config.api_key_env}`);
        }

        const genAI = new GoogleGenerativeAI(apiKey);
        const model = genAI.getGenerativeModel({ model: config.model });

        // Build chat history (skip system message)
        const chatHistory = this.conversation
            .filter(m => m.role !== 'system')
            .map(m => ({
                role: m.role === 'assistant' ? 'model' : 'user',
                parts: [{ text: m.content }],
            }));

        const chat = model.startChat({ history: chatHistory.slice(0, -1) as any });
        const result = await chat.sendMessage(chatHistory[chatHistory.length - 1].parts[0].text);
        const response = await result.response;

        return response.text();
    }

    private async callOllama(): Promise<string> {
        const config = this.agentConfig.llm;
        const baseUrl = config.base_url || 'http://localhost:11434';

        const response = await fetch(`${baseUrl}/api/chat`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({
                model: config.model,
                messages: this.conversation,
                stream: false,
                options: {
                    temperature: config.temperature,
                    num_predict: config.max_tokens,
                },
            }),
        });

        if (!response.ok) {
            throw new Error(`Ollama request failed: ${response.statusText}`);
        }

        const data = await response.json();
        return data.message.content;
    }

    private manageMemory() {
        const memoryConfig = this.agentConfig.memory;
        const maxMessages = memoryConfig.max_messages;
        const memoryType = memoryConfig.type;

        const systemMsgs = this.conversation.filter(m => m.role === 'system');
        const otherMsgs = this.conversation.filter(m => m.role !== 'system');

        if (otherMsgs.length > maxMessages) {
            if (memoryType === 'buffer') {
                // Keep only last N messages
                this.conversation = [...systemMsgs, ...otherMsgs.slice(-maxMessages)];
            } else if (memoryType === 'summary') {
                // TODO: Implement summarization
                // For now, just truncate
                this.conversation = [...systemMsgs, ...otherMsgs.slice(-maxMessages)];
            }
        }
    }

    async run() {
        console.log('\n💬 Agent ready! Type "exit" to quit.\n');

        const rl = readline.createInterface({
            input: process.stdin,
            output: process.stdout,
        });

        const askQuestion = (query: string): Promise<string> => {
            return new Promise(resolve => rl.question(query, resolve));
        };

        while (true) {
            try {
                const userInput = await askQuestion('You: ');

                if (userInput.trim().toLowerCase() === 'exit') {
                    console.log('👋 Goodbye!');
                    rl.close();
                    break;
                }

                if (!userInput.trim()) {
                    continue;
                }

                // Add user message
                this.conversation.push({ role: 'user', content: userInput });

                // Call LLM
                const response = await this.callLLM();

                // Add assistant response
                this.conversation.push({ role: 'assistant', content: response });

                // Manage memory
                this.manageMemory();

                console.log(`\nAssistant: ${response}\n`);
            } catch (error) {
                console.error(`\n❌ Error: ${error}\n`);
            }
        }
    }
}

async function main() {
    const agent = new SekuireAgent();
    await agent.initialize();

    // Get agent name from command line or use default
    const agentName = process.argv[2];
    agent.selectAgent(agentName);

    // Run agent
    await agent.run();
}

main().catch(console.error);
"#;

// ============================================================================
// HTTP Server Templates and Helper Functions
// ============================================================================

/// Get sekuire.yml with server configuration
pub fn get_sekuire_yml_with_server(
    name: &str,
    tools: &[&str],
    memory: Option<(&str, &str)>,
    framework: &str,
) -> String {
    let base = get_sekuire_yml(name, tools, memory);

    let server_config = format!(
        r#"

# Server Configuration
server:
  enabled: true
  framework: "{}"
  port: 8000
  host: "0.0.0.0"

  endpoints:
    chat: true
    health: true
    webhook: true
    sekuire_protocol: true

  cors:
    enabled: true
    origins: ["*"]

  auth:
    type: "none"
"#,
        framework
    );

    format!("{}{}", base, server_config)
}

/// Get .env.example with server variables
pub fn get_env_example_with_server() -> &'static str {
    r#"# LLM API Keys
OPENAI_API_KEY=sk-...
ANTHROPIC_API_KEY=sk-ant-...
GOOGLE_API_KEY=...

# Server Configuration
MODE=http
PORT=8000
HOST=0.0.0.0

# Sekuire Registry
SEKUIRE_REGISTRY_URL=https://api.sekuire.ai

# Optional: Agent Keys (generated in .sekuire/ directory)
# SEKUIRE_PRIVATE_KEY=...
# SEKUIRE_PUBLIC_KEY=...
"#
}

// ============================================================================
// TypeScript HTTP Templates
// ============================================================================

/// Shared agent logic (for hybrid and HTTP agents)
pub const TS_AGENT_SHARED: &str = r#"import { Agent, getAgent } from "@sekuire/sdk";

export class AgentService {
  private agent: Agent | null = null;

  async initialize(agentName?: string) {
    this.agent = await getAgent(agentName);
    console.log(`✅ Agent initialized: ${this.agent.name}`);
  }

  async chat(message: string, context?: Record<string, unknown>): Promise<string> {
    if (!this.agent) {
      throw new Error("Agent not initialized. Call initialize() first.");
    }

    const prompt = context
      ? `${message}\n\nContext: ${JSON.stringify(context)}`
      : message;

    return this.agent.chat(prompt);
  }

  getName(): string {
    if (!this.agent) {
      throw new Error("Agent not initialized");
    }
    return this.agent.name;
  }

  getModelName(): string {
    if (!this.agent) {
      throw new Error("Agent not initialized");
    }
    return this.agent.getModelName();
  }
}
"#;

/// Express HTTP server template
pub const TS_HTTP_EXPRESS: &str = r#"import express from "express";
import cors from "cors";
import { SekuireServer, createSekuireExpressMiddleware } from "@sekuire/sdk";
import { AgentService } from "./agent";
import fs from "fs/promises";
import dotenv from "dotenv";

dotenv.config();

// Concrete implementation of SekuireServer
class MyAgentServer extends SekuireServer {
  // SekuireServer provides handleHello and handleAuth implementations
}

async function startServer() {
  const agentService = new AgentService();
  await agentService.initialize();

  // Load keys for Sekuire protocol
  const privateKey = (await fs.readFile(".sekuire/secret.key", "utf-8")).trim();
  const publicKey = (await fs.readFile(".sekuire/public.key", "utf-8")).trim();

  const app = express();
  app.use(cors());
  app.use(express.json());

  // Health endpoint
  app.get("/health", (req, res) => {
    res.json({
      status: "ok",
      agent: agentService.getName(),
      model: agentService.getModelName(),
    });
  });

  // Chat endpoint
  app.post("/chat", async (req, res) => {
    try {
      const { message, context } = req.body;
      if (!message) {
        return res.status(400).json({ error: "Message required" });
      }

      const response = await agentService.chat(message, context);
      res.json({ response });
    } catch (error) {
      console.error("Chat error:", error);
      res.status(500).json({ error: "Internal server error" });
    }
  });

  // Webhook endpoint
  app.post("/webhook", async (req, res) => {
    try {
      const payload = req.body;
      const message = `Process webhook: ${JSON.stringify(payload)}`;
      const response = await agentService.chat(message, {
        source: "webhook",
        headers: req.headers,
      });

      res.json({ received: true, response });
    } catch (error) {
      console.error("Webhook error:", error);
      res.status(500).json({ error: "Internal server error" });
    }
  });

  // Sekuire protocol endpoints (leverages SDK middleware)
  // Note: Agent ID would typically come from sekuire.yml
  const sekuireServer = new MyAgentServer("agent-id-placeholder", {
    privateKey,
    publicKey,
  });
  app.use(createSekuireExpressMiddleware(sekuireServer));

  const port = process.env.PORT || 8000;
  const host = process.env.HOST || "0.0.0.0";

  app.listen(Number(port), host, () => {
    console.log(`\n🚀 Agent server running at http://${host}:${port}`);
    console.log(`   Agent: ${agentService.getName()}`);
    console.log(`   Model: ${agentService.getModelName()}`);
    console.log(`\n📡 Endpoints:`);
    console.log(`   GET  /health - Health check`);
    console.log(`   POST /chat - Send message to agent`);
    console.log(`   POST /webhook - Webhook receiver`);
    console.log(`   POST /sekuire/hello - Sekuire protocol handshake`);
    console.log(`   POST /sekuire/auth - Sekuire protocol auth\n`);
  });
}

if (import.meta.url === `file://${process.argv[1]}`) {
  startServer();
}

export default startServer;
"#;

/// Fastify HTTP server template
pub const TS_HTTP_FASTIFY: &str = r#"import Fastify from "fastify";
import cors from "@fastify/cors";
import { SekuireServer, createSekuireFastifyPlugin } from "@sekuire/sdk";
import { AgentService } from "./agent";
import fs from "fs/promises";
import dotenv from "dotenv";

dotenv.config();

// Concrete implementation of SekuireServer
class MyAgentServer extends SekuireServer {
  // SekuireServer provides handleHello and handleAuth implementations
}

async function startServer() {
  const fastify = Fastify({ logger: true });

  // Register CORS
  await fastify.register(cors, { origin: "*" });

  const agentService = new AgentService();
  await agentService.initialize();

  // Load keys for Sekuire protocol
  const privateKey = (await fs.readFile(".sekuire/secret.key", "utf-8")).trim();
  const publicKey = (await fs.readFile(".sekuire/public.key", "utf-8")).trim();

  // Health endpoint
  fastify.get("/health", async (request, reply) => {
    return {
      status: "ok",
      agent: agentService.getName(),
      model: agentService.getModelName(),
    };
  });

  // Chat endpoint
  fastify.post("/chat", async (request, reply) => {
    const { message, context } = request.body as { message?: string; context?: any };

    if (!message) {
      reply.code(400);
      return { error: "Message required" };
    }

    try {
      const response = await agentService.chat(message, context);
      return { response };
    } catch (error) {
      fastify.log.error(error);
      reply.code(500);
      return { error: "Internal server error" };
    }
  });

  // Webhook endpoint
  fastify.post("/webhook", async (request, reply) => {
    try {
      const payload = request.body as Record<string, unknown>;
      const message = `Process webhook: ${JSON.stringify(payload)}`;
      const response = await agentService.chat(message, {
        source: "webhook",
        headers: request.headers,
      });

      return { received: true, response };
    } catch (error) {
      fastify.log.error(error);
      reply.code(500);
      return { error: "Internal server error" };
    }
  });

  // Sekuire protocol endpoints (leverages SDK plugin)
  const sekuireServer = new MyAgentServer("agent-id-placeholder", {
    privateKey,
    publicKey,
  });
  await fastify.register(createSekuireFastifyPlugin(sekuireServer));

  const port = Number(process.env.PORT) || 8000;
  const host = process.env.HOST || "0.0.0.0";

  try {
    await fastify.listen({ port, host });
    console.log(`\n🚀 Agent server running at http://${host}:${port}`);
    console.log(`   Agent: ${agentService.getName()}`);
    console.log(`   Model: ${agentService.getModelName()}`);
    console.log(`\n📡 Endpoints:`);
    console.log(`   GET  /health - Health check`);
    console.log(`   POST /chat - Send message to agent`);
    console.log(`   POST /webhook - Webhook receiver`);
    console.log(`   POST /sekuire/hello - Sekuire protocol handshake`);
    console.log(`   POST /sekuire/auth - Sekuire protocol auth\n`);
  } catch (err) {
    fastify.log.error(err);
    process.exit(1);
  }
}

if (import.meta.url === `file://${process.argv[1]}`) {
  startServer();
}

export default startServer;
"#;

/// Hono HTTP server template
pub const TS_HTTP_HONO: &str = r#"import { Hono } from "hono";
import { cors } from "hono/cors";
import { serve } from "@hono/node-server";
import { AgentService } from "./agent";
import fs from "fs/promises";
import dotenv from "dotenv";

dotenv.config();

async function startServer() {
  const app = new Hono();

  // CORS middleware
  app.use("/*", cors());

  const agentService = new AgentService();
  await agentService.initialize();

  // Load keys for Sekuire protocol
  const privateKey = (await fs.readFile(".sekuire/secret.key", "utf-8")).trim();
  const publicKey = (await fs.readFile(".sekuire/public.key", "utf-8")).trim();

  // Health endpoint
  app.get("/health", (c) => {
    return c.json({
      status: "ok",
      agent: agentService.getName(),
      model: agentService.getModelName(),
    });
  });

  // Chat endpoint
  app.post("/chat", async (c) => {
    try {
      const { message, context } = await c.req.json();

      if (!message) {
        return c.json({ error: "Message required" }, 400);
      }

      const response = await agentService.chat(message, context);
      return c.json({ response });
    } catch (error) {
      console.error("Chat error:", error);
      return c.json({ error: "Internal server error" }, 500);
    }
  });

  // Webhook endpoint
  app.post("/webhook", async (c) => {
    try {
      const payload = await c.req.json();
      const message = `Process webhook: ${JSON.stringify(payload)}`;
      const response = await agentService.chat(message, {
        source: "webhook",
        headers: c.req.header(),
      });

      return c.json({ received: true, response });
    } catch (error) {
      console.error("Webhook error:", error);
      return c.json({ error: "Internal server error" }, 500);
    }
  });

  // Sekuire protocol endpoints (manual implementation)
  app.post("/sekuire/hello", async (c) => {
    try {
      const { clientNonce } = await c.req.json();

      // Basic handshake response (simplified)
      return c.json({
        agentId: "agent-id-placeholder",
        publicKey: publicKey,
        message: "Hello from Sekuire agent",
      });
    } catch (error) {
      return c.json({ error: "Handshake failed" }, 400);
    }
  });

  app.post("/sekuire/auth", async (c) => {
    try {
      // Basic auth response (simplified)
      return c.json({ success: true });
    } catch (error) {
      return c.json({ error: "Auth failed" }, 400);
    }
  });

  const port = Number(process.env.PORT) || 8000;
  const host = process.env.HOST || "0.0.0.0";

  console.log(`\n🚀 Agent server running at http://${host}:${port}`);
  console.log(`   Agent: ${agentService.getName()}`);
  console.log(`   Model: ${agentService.getModelName()}`);
  console.log(`\n📡 Endpoints:`);
  console.log(`   GET  /health - Health check`);
  console.log(`   POST /chat - Send message to agent`);
  console.log(`   POST /webhook - Webhook receiver`);
  console.log(`   POST /sekuire/hello - Sekuire protocol handshake`);
  console.log(`   POST /sekuire/auth - Sekuire protocol auth\n`);

  serve({
    fetch: app.fetch,
    port,
    hostname: host,
  });
}

export default startServer;
"#;

/// TypeScript CLI interface template
pub const TS_CLI_INTERFACE: &str = r#"import readline from "readline";
import { AgentService } from "./agent";

async function startCLI() {
  console.log("🤖 Sekuire Agent CLI\n");

  const agentService = new AgentService();
  await agentService.initialize();

  const rl = readline.createInterface({
    input: process.stdin,
    output: process.stdout,
    prompt: "\n> ",
  });

  console.log(`Agent: ${agentService.getName()}`);
  console.log(`Model: ${agentService.getModelName()}`);
  console.log('\nType your message (or "exit" to quit)\n');

  rl.prompt();

  rl.on("line", async (line) => {
    const input = line.trim();

    if (input.toLowerCase() === "exit") {
      console.log("Goodbye!");
      rl.close();
      process.exit(0);
    }

    if (!input) {
      rl.prompt();
      return;
    }

    try {
      const response = await agentService.chat(input);
      console.log(`\nAssistant: ${response}`);
    } catch (error) {
      console.error(`\n❌ Error: ${error}`);
    }

    rl.prompt();
  });

  rl.on("close", () => {
    console.log("\nGoodbye!");
    process.exit(0);
  });
}

export default startCLI;
"#;

/// Hybrid mode entry point (mode router)
pub const TS_HYBRID_INDEX: &str = r#"const mode = process.env.MODE || "cli";

if (mode === "http") {
  import("./server").then((m) => m.default());
} else {
  import("./cli").then((m) => m.default());
}
"#;

/// TypeScript built-in HTTP server template
pub const TS_HTTP_BUILTIN: &str = r#"import http from "http";
import { AgentService } from "./agent";
import fs from "fs/promises";
import dotenv from "dotenv";

dotenv.config();

async function startServer() {
  const agentService = new AgentService();
  await agentService.initialize();

  // Load keys for Sekuire protocol
  const privateKey = (await fs.readFile(".sekuire/secret.key", "utf-8")).trim();
  const publicKey = (await fs.readFile(".sekuire/public.key", "utf-8")).trim();

  const server = http.createServer(async (req, res) => {
    // CORS headers
    res.setHeader("Access-Control-Allow-Origin", "*");
    res.setHeader("Access-Control-Allow-Methods", "GET, POST, OPTIONS");
    res.setHeader("Access-Control-Allow-Headers", "Content-Type");

    if (req.method === "OPTIONS") {
      res.writeHead(200);
      res.end();
      return;
    }

    // Parse JSON body
    let body = "";
    req.on("data", (chunk) => {
      body += chunk.toString();
    });

    req.on("end", async () => {
      try {
        const parsedBody = body ? JSON.parse(body) : {};

        // Health endpoint
        if (req.method === "GET" && req.url === "/health") {
          res.writeHead(200, { "Content-Type": "application/json" });
          res.end(
            JSON.stringify({
              status: "ok",
              agent: agentService.getName(),
              model: agentService.getModelName(),
            })
          );
          return;
        }

        // Chat endpoint
        if (req.method === "POST" && req.url === "/chat") {
          const { message, context } = parsedBody;
          if (!message) {
            res.writeHead(400, { "Content-Type": "application/json" });
            res.end(JSON.stringify({ error: "Message required" }));
            return;
          }

          const response = await agentService.chat(message, context);
          res.writeHead(200, { "Content-Type": "application/json" });
          res.end(JSON.stringify({ response }));
          return;
        }

        // Webhook endpoint
        if (req.method === "POST" && req.url === "/webhook") {
          const message = `Process webhook: ${JSON.stringify(parsedBody)}`;
          const response = await agentService.chat(message, {
            source: "webhook",
            headers: req.headers,
          });

          res.writeHead(200, { "Content-Type": "application/json" });
          res.end(JSON.stringify({ received: true, response }));
          return;
        }

        // Sekuire protocol - hello endpoint
        if (req.method === "POST" && req.url === "/sekuire/hello") {
          res.writeHead(200, { "Content-Type": "application/json" });
          res.end(
            JSON.stringify({
              agentId: "agent-id-placeholder",
              publicKey: publicKey,
              message: "Hello from Sekuire agent",
            })
          );
          return;
        }

        // Sekuire protocol - auth endpoint
        if (req.method === "POST" && req.url === "/sekuire/auth") {
          res.writeHead(200, { "Content-Type": "application/json" });
          res.end(JSON.stringify({ success: true }));
          return;
        }

        // 404 Not Found
        res.writeHead(404, { "Content-Type": "application/json" });
        res.end(JSON.stringify({ error: "Not Found" }));
      } catch (error) {
        console.error("Server error:", error);
        res.writeHead(500, { "Content-Type": "application/json" });
        res.end(JSON.stringify({ error: "Internal server error" }));
      }
    });
  });

  const port = Number(process.env.PORT) || 8000;
  const host = process.env.HOST || "0.0.0.0";

  server.listen(port, host, () => {
    console.log(`\n🚀 Agent server running at http://${host}:${port}`);
    console.log(`   Agent: ${agentService.getName()}`);
    console.log(`   Model: ${agentService.getModelName()}`);
    console.log(`\n📡 Endpoints:`);
    console.log(`   GET  /health - Health check`);
    console.log(`   POST /chat - Send message to agent`);
    console.log(`   POST /webhook - Webhook receiver`);
    console.log(`   POST /sekuire/hello - Sekuire protocol handshake`);
    console.log(`   POST /sekuire/auth - Sekuire protocol auth\n`);
  });
}

export default startServer;
"#;

/// Get the correct HTTP server template based on framework
pub fn get_http_server_template(framework: &str) -> &'static str {
    match framework {
        "express" => TS_HTTP_EXPRESS,
        "fastify" => TS_HTTP_FASTIFY,
        "hono" => TS_HTTP_HONO,
        "built-in" | "built-in node.js" => TS_HTTP_BUILTIN,
        _ => TS_HTTP_EXPRESS, // Default to Express
    }
}

/// Generate package.json for HTTP agents with framework-specific dependencies
pub fn get_ts_package_http(project_name: &str, framework: &str) -> String {
    let package_name = project_name.to_lowercase().replace(" ", "-");

    let framework_deps = match framework {
        "express" => {
            r#""express": "^4.18.2",
    "cors": "^2.8.5",
    "@types/express": "^4.17.21",
    "@types/cors": "^2.8.17""#
        }
        "fastify" => {
            r#""fastify": "^4.25.2",
    "@fastify/cors": "^8.5.0""#
        }
        "hono" => {
            r#""hono": "^3.12.0",
    "@hono/node-server": "^1.5.0""#
        }
        "built-in" | "built-in node.js" => "", // No extra dependencies for built-in http
        _ => "",
    };

    let deps = if framework_deps.is_empty() {
        r#"    "@sekuire/sdk": "latest",
    "dotenv": "^16.4.0""#
            .to_string()
    } else {
        format!(
            r#"    "@sekuire/sdk": "latest",
    {},
    "dotenv": "^16.4.0""#,
            framework_deps
        )
    };

    format!(
        r#"{{
  "name": "{}",
  "version": "0.1.0",
  "description": "Sekuire HTTP Agent",
  "type": "module",
  "scripts": {{
    "build": "tsc",
    "start": "node dist/index.js",
    "dev": "tsx src/index.ts"
  }},
  "dependencies": {{
{}
  }},
  "devDependencies": {{
    "@types/node": "^22.0.0",
    "typescript": "^5.0.0",
    "tsx": "^4.7.0"
  }}
}}"#,
        package_name, deps
    )
}

/// Generate package.json for hybrid agents
pub fn get_ts_package_hybrid(project_name: &str, framework: &str) -> String {
    // Hybrid agents need the same dependencies as HTTP agents
    get_ts_package_http(project_name, framework)
}

// ============================================================================
// Python HTTP Templates
// ============================================================================

/// Python shared agent logic
pub const PYTHON_AGENT_SHARED: &str = r#"from sekuire_sdk import get_agent
from typing import Optional, Dict, Any


class AgentService:
    """Shared agent service for HTTP and CLI modes"""

    def __init__(self):
        self.agent = None

    async def initialize(self, agent_name: Optional[str] = None):
        """Initialize the agent"""
        self.agent = await get_agent(agent_name)
        print(f"✅ Agent initialized: {self.agent.name}")

    async def chat(self, message: str, context: Optional[Dict[str, Any]] = None) -> str:
        """Chat with the agent"""
        if not self.agent:
            raise RuntimeError("Agent not initialized. Call initialize() first.")

        prompt = message
        if context:
            prompt = f"{message}\n\nContext: {context}"

        return await self.agent.chat(prompt)

    def get_name(self) -> str:
        """Get agent name"""
        if not self.agent:
            raise RuntimeError("Agent not initialized")
        return self.agent.name

    def get_model_name(self) -> str:
        """Get model name"""
        if not self.agent:
            raise RuntimeError("Agent not initialized")
        return self.agent.get_model_name()
"#;

/// Python FastAPI HTTP server template
pub const PYTHON_HTTP_FASTAPI: &str = r#"from fastapi import FastAPI, HTTPException
from fastapi.middleware.cors import CORSMiddleware
from pydantic import BaseModel
from typing import Optional, Dict, Any
import os
from agent import AgentService

# Request/Response models
class ChatRequest(BaseModel):
    message: str
    context: Optional[Dict[str, Any]] = None


class ChatResponse(BaseModel):
    response: str


class WebhookResponse(BaseModel):
    received: bool
    response: str


# Initialize FastAPI app
app = FastAPI(title="Sekuire Agent")

# CORS middleware
app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

# Initialize agent service
agent_service = AgentService()


@app.on_event("startup")
async def startup_event():
    """Initialize agent on startup"""
    await agent_service.initialize()
    print(f"\n🚀 Agent server running")
    print(f"   Agent: {agent_service.get_name()}")
    print(f"   Model: {agent_service.get_model_name()}")
    print(f"\n📡 Endpoints:")
    print(f"   GET  /health - Health check")
    print(f"   POST /chat - Send message to agent")
    print(f"   POST /webhook - Webhook receiver")
    print(f"   POST /sekuire/hello - Sekuire protocol handshake")
    print(f"   POST /sekuire/auth - Sekuire protocol auth\n")


@app.get("/health")
async def health():
    """Health check endpoint"""
    return {
        "status": "ok",
        "agent": agent_service.get_name(),
        "model": agent_service.get_model_name(),
    }


@app.post("/chat", response_model=ChatResponse)
async def chat(request: ChatRequest):
    """Chat endpoint"""
    try:
        response = await agent_service.chat(request.message, request.context)
        return ChatResponse(response=response)
    except Exception as e:
        print(f"Chat error: {e}")
        raise HTTPException(status_code=500, detail="Internal server error")


@app.post("/webhook", response_model=WebhookResponse)
async def webhook(payload: Dict[str, Any]):
    """Webhook endpoint"""
    try:
        import json
        message = f"Process webhook: {json.dumps(payload)}"
        response = await agent_service.chat(message, {"source": "webhook"})
        return WebhookResponse(received=True, response=response)
    except Exception as e:
        print(f"Webhook error: {e}")
        raise HTTPException(status_code=500, detail="Internal server error")


@app.post("/sekuire/hello")
async def sekuire_hello(payload: Dict[str, Any]):
    """Sekuire protocol handshake endpoint"""
    try:
        # Load public key
        with open(".sekuire/public.key", "r") as f:
            public_key = f.read().strip()

        return {
            "agentId": "agent-id-placeholder",
            "publicKey": public_key,
            "message": "Hello from Sekuire agent",
        }
    except Exception as e:
        raise HTTPException(status_code=400, detail="Handshake failed")


@app.post("/sekuire/auth")
async def sekuire_auth(payload: Dict[str, Any]):
    """Sekuire protocol auth endpoint"""
    try:
        return {"success": True}
    except Exception as e:
        raise HTTPException(status_code=400, detail="Auth failed")


if __name__ == "__main__":
    import uvicorn

    port = int(os.getenv("PORT", 8000))
    host = os.getenv("HOST", "0.0.0.0")

    uvicorn.run(app, host=host, port=port)
"#;

/// Python Django HTTP server template
pub const PYTHON_HTTP_DJANGO: &str = r#"# Django REST Framework implementation
# Note: This requires additional Django setup (settings.py, urls.py, etc.)
# This is a simplified template showing the views.py for the agent endpoints

from rest_framework.decorators import api_view
from rest_framework.response import Response
from rest_framework import status
from typing import Dict, Any
import os
import asyncio
from agent import AgentService

# Global agent service instance
agent_service = AgentService()

# Initialize agent on module load
loop = asyncio.get_event_loop()
loop.run_until_complete(agent_service.initialize())


@api_view(["GET"])
def health(request):
    """Health check endpoint"""
    return Response({
        "status": "ok",
        "agent": agent_service.get_name(),
        "model": agent_service.get_model_name(),
    })


@api_view(["POST"])
def chat(request):
    """Chat endpoint"""
    message = request.data.get("message")
    context = request.data.get("context")

    if not message:
        return Response(
            {"error": "Message required"},
            status=status.HTTP_400_BAD_REQUEST
        )

    try:
        loop = asyncio.new_event_loop()
        asyncio.set_event_loop(loop)
        response = loop.run_until_complete(
            agent_service.chat(message, context)
        )
        return Response({"response": response})
    except Exception as e:
        print(f"Chat error: {e}")
        return Response(
            {"error": "Internal server error"},
            status=status.HTTP_500_INTERNAL_SERVER_ERROR
        )


@api_view(["POST"])
def webhook(request):
    """Webhook endpoint"""
    try:
        import json
        payload = request.data
        message = f"Process webhook: {json.dumps(payload)}"

        loop = asyncio.new_event_loop()
        asyncio.set_event_loop(loop)
        response = loop.run_until_complete(
            agent_service.chat(message, {"source": "webhook"})
        )

        return Response({
            "received": True,
            "response": response
        })
    except Exception as e:
        print(f"Webhook error: {e}")
        return Response(
            {"error": "Internal server error"},
            status=status.HTTP_500_INTERNAL_SERVER_ERROR
        )


@api_view(["POST"])
def sekuire_hello(request):
    """Sekuire protocol handshake endpoint"""
    try:
        with open(".sekuire/public.key", "r") as f:
            public_key = f.read().strip()

        return Response({
            "agentId": "agent-id-placeholder",
            "publicKey": public_key,
            "message": "Hello from Sekuire agent",
        })
    except Exception as e:
        return Response(
            {"error": "Handshake failed"},
            status=status.HTTP_400_BAD_REQUEST
        )


@api_view(["POST"])
def sekuire_auth(request):
    """Sekuire protocol auth endpoint"""
    try:
        return Response({"success": True})
    except Exception as e:
        return Response(
            {"error": "Auth failed"},
            status=status.HTTP_400_BAD_REQUEST
        )


# URL patterns (add to urls.py):
# from django.urls import path
# from . import views
#
# urlpatterns = [
#     path('health', views.health),
#     path('chat', views.chat),
#     path('webhook', views.webhook),
#     path('sekuire/hello', views.sekuire_hello),
#     path('sekuire/auth', views.sekuire_auth),
# ]
"#;

/// Python CLI interface template
pub const PYTHON_CLI_INTERFACE: &str = r#"import asyncio
from agent import AgentService


async def start_cli():
    """Start CLI interface"""
    print("🤖 Sekuire Agent CLI\n")

    agent_service = AgentService()
    await agent_service.initialize()

    print(f"Agent: {agent_service.get_name()}")
    print(f"Model: {agent_service.get_model_name()}")
    print('\nType your message (or "exit" to quit)\n')

    while True:
        try:
            user_input = input("> ").strip()

            if user_input.lower() == "exit":
                print("Goodbye!")
                break

            if not user_input:
                continue

            response = await agent_service.chat(user_input)
            print(f"\nAssistant: {response}\n")

        except KeyboardInterrupt:
            print("\n\nGoodbye!")
            break
        except Exception as e:
            print(f"\n❌ Error: {e}\n")


if __name__ == "__main__":
    asyncio.run(start_cli())
"#;

/// Python hybrid entry point (mode router)
pub const PYTHON_HYBRID_MAIN: &str = r#"import os
import sys


if __name__ == "__main__":
    mode = os.getenv("MODE", "cli")

    if mode == "http":
        # Import and run server
        import uvicorn
        from server import app

        port = int(os.getenv("PORT", 8000))
        host = os.getenv("HOST", "0.0.0.0")
        uvicorn.run(app, host=host, port=port)
    else:
        # Import and run CLI
        import asyncio
        from cli import start_cli

        asyncio.run(start_cli())
"#;

/// Generate requirements.txt for Python HTTP agents
pub fn get_python_requirements_http(framework: &str) -> &'static str {
    match framework {
        "fastapi" => {
            r#"sekuire-sdk>=0.1.0
fastapi>=0.109.0
uvicorn[standard]>=0.27.0
pydantic>=2.0.0
python-dotenv>=1.0.0
"#
        }
        "django" => {
            r#"sekuire-sdk>=0.1.0
django>=5.0.0
djangorestframework>=3.14.0
python-dotenv>=1.0.0
"#
        }
        _ => {
            r#"sekuire-sdk>=0.1.0
fastapi>=0.109.0
uvicorn[standard]>=0.27.0
pydantic>=2.0.0
python-dotenv>=1.0.0
"#
        }
    }
}

/// Get the correct Python HTTP server template based on framework
pub fn get_python_http_server_template(framework: &str) -> &'static str {
    match framework {
        "fastapi" => PYTHON_HTTP_FASTAPI,
        "django" => PYTHON_HTTP_DJANGO,
        _ => PYTHON_HTTP_FASTAPI, // Default to FastAPI
    }
}

// ============================================================================
// RUST TEMPLATES - HTTP SERVERS
// ============================================================================

/// Rust shared agent logic
pub const RUST_AGENT_SHARED: &str = r#"use anyhow::Result;
use sekuire_sdk::{get_agent, Agent};
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct AgentService {
    agent: Arc<Mutex<Option<Agent>>>,
}

impl AgentService {
    pub fn new() -> Self {
        Self {
            agent: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn initialize(&self, agent_name: Option<String>) -> Result<()> {
        let agent = get_agent(agent_name.as_deref(), None).await?;
        let name = agent.name.clone();
        *self.agent.lock().await = Some(agent);
        println!("✅ Agent initialized: {}", name);
        Ok(())
    }

    pub async fn chat(&self, message: &str, context: Option<serde_json::Value>) -> Result<String> {
        let agent_lock = self.agent.lock().await;
        let agent = agent_lock.as_ref().ok_or_else(|| {
            anyhow::anyhow!("Agent not initialized. Call initialize() first.")
        })?;

        let prompt = if let Some(ctx) = context {
            format!("{}\n\nContext: {}", message, serde_json::to_string(&ctx)?)
        } else {
            message.to_string()
        };

        agent.chat(&prompt).await
    }

    pub async fn get_name(&self) -> Result<String> {
        let agent_lock = self.agent.lock().await;
        let agent = agent_lock.as_ref().ok_or_else(|| {
            anyhow::anyhow!("Agent not initialized")
        })?;
        Ok(agent.name.clone())
    }

    pub async fn get_model_name(&self) -> Result<String> {
        let agent_lock = self.agent.lock().await;
        let agent = agent_lock.as_ref().ok_or_else(|| {
            anyhow::anyhow!("Agent not initialized")
        })?;
        Ok(agent.get_model_name())
    }
}

impl Default for AgentService {
    fn default() -> Self {
        Self::new()
    }
}
"#;

/// Rust Axum HTTP server template
pub const RUST_HTTP_AXUM: &str = r#"mod agent;

use agent::AgentService;
use axum::{
    extract::{Json, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};

#[derive(Clone)]
struct AppState {
    agent_service: Arc<AgentService>,
}

#[derive(Deserialize)]
struct ChatRequest {
    message: String,
    #[serde(default)]
    context: Option<serde_json::Value>,
}

#[derive(Serialize)]
struct ChatResponse {
    response: String,
}

#[derive(Serialize)]
struct HealthResponse {
    status: String,
    agent: String,
}

#[derive(Deserialize)]
struct SekuireHelloRequest {
    client_nonce: String,
}

#[derive(Serialize)]
struct SekuireWelcomeResponse {
    agent_id: String,
    agent_nonce: String,
    signature_c: String,
    credentials: Vec<String>,
}

#[derive(Deserialize)]
struct SekuireAuthRequest {
    signature_a: String,
}

#[derive(Serialize)]
struct SekuireAuthResponse {
    success: bool,
}

async fn health(State(state): State<AppState>) -> Result<Json<HealthResponse>, StatusCode> {
    let agent_name = state
        .agent_service
        .get_name()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(HealthResponse {
        status: "ok".to_string(),
        agent: agent_name,
    }))
}

async fn chat(
    State(state): State<AppState>,
    Json(req): Json<ChatRequest>,
) -> Result<Json<ChatResponse>, StatusCode> {
    let response = state
        .agent_service
        .chat(&req.message, req.context)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(ChatResponse { response }))
}

async fn webhook(
    State(state): State<AppState>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let message = serde_json::to_string(&payload)
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    let response = state
        .agent_service
        .chat(&message, None)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(serde_json::json!({
        "received": true,
        "response": response
    })))
}

async fn sekuire_hello(
    Json(_req): Json<SekuireHelloRequest>,
) -> Result<Json<SekuireWelcomeResponse>, StatusCode> {
    // TODO: Implement Sekuire protocol handshake
    // 1. Validate client nonce
    // 2. Generate agent nonce
    // 3. Sign client nonce with private key
    // 4. Load credentials from .sekuire/

    Ok(Json(SekuireWelcomeResponse {
        agent_id: "agent-placeholder".to_string(),
        agent_nonce: "placeholder-nonce".to_string(),
        signature_c: "placeholder-signature".to_string(),
        credentials: vec![],
    }))
}

async fn sekuire_auth(
    _headers: HeaderMap,
    Json(_req): Json<SekuireAuthRequest>,
) -> Result<Json<SekuireAuthResponse>, StatusCode> {
    // TODO: Implement Sekuire protocol authentication
    // 1. Retrieve client nonce from session/storage
    // 2. Verify signature_a against agent nonce
    // 3. Complete mutual authentication

    Ok(Json(SekuireAuthResponse { success: true }))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load environment variables
    dotenv::dotenv().ok();

    // Initialize agent service
    let agent_service = Arc::new(AgentService::new());
    agent_service.initialize(None).await?;

    let state = AppState { agent_service };

    // Configure CORS
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Build router
    let app = Router::new()
        .route("/health", get(health))
        .route("/chat", post(chat))
        .route("/webhook", post(webhook))
        .route("/sekuire/hello", post(sekuire_hello))
        .route("/sekuire/auth", post(sekuire_auth))
        .layer(cors)
        .with_state(state);

    // Start server
    let port = std::env::var("PORT").unwrap_or_else(|_| "8000".to_string());
    let host = std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let addr = format!("{}:{}", host, port);

    println!("🚀 Server running on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

export default main;
"#;

/// Rust Actix HTTP server template
pub const RUST_HTTP_ACTIX: &str = r#"mod agent;

use agent::AgentService;
use actix_cors::Cors;
use actix_web::{
    get, post, web, App, HttpRequest, HttpResponse, HttpServer, Responder,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

struct AppState {
    agent_service: Arc<AgentService>,
}

#[derive(Deserialize)]
struct ChatRequest {
    message: String,
    #[serde(default)]
    context: Option<serde_json::Value>,
}

#[derive(Serialize)]
struct ChatResponse {
    response: String,
}

#[derive(Serialize)]
struct HealthResponse {
    status: String,
    agent: String,
}

#[derive(Deserialize)]
struct SekuireHelloRequest {
    client_nonce: String,
}

#[derive(Serialize)]
struct SekuireWelcomeResponse {
    agent_id: String,
    agent_nonce: String,
    signature_c: String,
    credentials: Vec<String>,
}

#[derive(Deserialize)]
struct SekuireAuthRequest {
    signature_a: String,
}

#[derive(Serialize)]
struct SekuireAuthResponse {
    success: bool,
}

#[get("/health")]
async fn health(data: web::Data<AppState>) -> impl Responder {
    match data.agent_service.get_name().await {
        Ok(agent_name) => HttpResponse::Ok().json(HealthResponse {
            status: "ok".to_string(),
            agent: agent_name,
        }),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

#[post("/chat")]
async fn chat(
    data: web::Data<AppState>,
    req: web::Json<ChatRequest>,
) -> impl Responder {
    match data.agent_service.chat(&req.message, req.context.clone()).await {
        Ok(response) => HttpResponse::Ok().json(ChatResponse { response }),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

#[post("/webhook")]
async fn webhook(
    data: web::Data<AppState>,
    payload: web::Json<serde_json::Value>,
) -> impl Responder {
    let message = match serde_json::to_string(&payload.into_inner()) {
        Ok(m) => m,
        Err(_) => return HttpResponse::BadRequest().finish(),
    };

    match data.agent_service.chat(&message, None).await {
        Ok(response) => HttpResponse::Ok().json(serde_json::json!({
            "received": true,
            "response": response
        })),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

#[post("/sekuire/hello")]
async fn sekuire_hello(
    _req: web::Json<SekuireHelloRequest>,
) -> impl Responder {
    // TODO: Implement Sekuire protocol handshake
    HttpResponse::Ok().json(SekuireWelcomeResponse {
        agent_id: "agent-placeholder".to_string(),
        agent_nonce: "placeholder-nonce".to_string(),
        signature_c: "placeholder-signature".to_string(),
        credentials: vec![],
    })
}

#[post("/sekuire/auth")]
async fn sekuire_auth(
    _req: HttpRequest,
    _body: web::Json<SekuireAuthRequest>,
) -> impl Responder {
    // TODO: Implement Sekuire protocol authentication
    HttpResponse::Ok().json(SekuireAuthResponse { success: true })
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Load environment variables
    dotenv::dotenv().ok();

    // Initialize agent service
    let agent_service = Arc::new(AgentService::new());
    agent_service.initialize(None).await.expect("Failed to initialize agent");

    let state = web::Data::new(AppState { agent_service });

    // Start server
    let port = std::env::var("PORT").unwrap_or_else(|_| "8000".to_string());
    let host = std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let addr = format!("{}:{}", host, port);

    println!("🚀 Server running on http://{}", addr);

    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header();

        App::new()
            .wrap(cors)
            .app_data(state.clone())
            .service(health)
            .service(chat)
            .service(webhook)
            .service(sekuire_hello)
            .service(sekuire_auth)
    })
    .bind(&addr)?
    .run()
    .await
}
"#;

/// Rust built-in HTTP server template
pub const RUST_HTTP_BUILTIN: &str = r#"mod agent;

use agent::AgentService;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;

#[derive(Deserialize)]
struct ChatRequest {
    message: String,
    #[serde(default)]
    context: Option<serde_json::Value>,
}

#[derive(Serialize)]
struct ChatResponse {
    response: String,
}

#[derive(Serialize)]
struct HealthResponse {
    status: String,
    agent: String,
}

fn parse_request(stream: &mut TcpStream) -> Option<(String, String, Vec<u8>)> {
    let mut buffer = [0u8; 4096];
    let bytes_read = stream.read(&mut buffer).ok()?;
    let request = String::from_utf8_lossy(&buffer[..bytes_read]);

    let lines: Vec<&str> = request.split("\r\n").collect();
    if lines.is_empty() {
        return None;
    }

    let request_line = lines[0];
    let parts: Vec<&str> = request_line.split_whitespace().collect();
    if parts.len() < 2 {
        return None;
    }

    let method = parts[0].to_string();
    let path = parts[1].to_string();

    // Find body (after blank line)
    let body_start = request.find("\r\n\r\n")?;
    let body = buffer[body_start + 4..bytes_read].to_vec();

    Some((method, path, body))
}

fn send_response(stream: &mut TcpStream, status: &str, body: &str) {
    let response = format!(
        "HTTP/1.1 {}\r\n\
         Content-Type: application/json\r\n\
         Access-Control-Allow-Origin: *\r\n\
         Access-Control-Allow-Methods: GET, POST, OPTIONS\r\n\
         Access-Control-Allow-Headers: Content-Type\r\n\
         Content-Length: {}\r\n\
         \r\n\
         {}",
        status,
        body.len(),
        body
    );
    let _ = stream.write_all(response.as_bytes());
    let _ = stream.flush();
}

async fn handle_client(mut stream: TcpStream, agent_service: Arc<AgentService>) {
    let (method, path, body) = match parse_request(&mut stream) {
        Some(req) => req,
        None => {
            send_response(&mut stream, "400 Bad Request", "{\"error\":\"Invalid request\"}");
            return;
        }
    };

    // Handle OPTIONS for CORS preflight
    if method == "OPTIONS" {
        send_response(&mut stream, "204 No Content", "");
        return;
    }

    match (method.as_str(), path.as_str()) {
        ("GET", "/health") => {
            let agent_name = agent_service.get_name().await.unwrap_or_else(|_| "unknown".to_string());
            let response = HealthResponse {
                status: "ok".to_string(),
                agent: agent_name,
            };
            let json = serde_json::to_string(&response).unwrap();
            send_response(&mut stream, "200 OK", &json);
        }
        ("POST", "/chat") => {
            let req: ChatRequest = match serde_json::from_slice(&body) {
                Ok(r) => r,
                Err(_) => {
                    send_response(&mut stream, "400 Bad Request", "{\"error\":\"Invalid JSON\"}");
                    return;
                }
            };

            match agent_service.chat(&req.message, req.context).await {
                Ok(response) => {
                    let json = serde_json::to_string(&ChatResponse { response }).unwrap();
                    send_response(&mut stream, "200 OK", &json);
                }
                Err(_) => {
                    send_response(&mut stream, "500 Internal Server Error", "{\"error\":\"Chat failed\"}");
                }
            }
        }
        ("POST", "/webhook") => {
            let payload: serde_json::Value = match serde_json::from_slice(&body) {
                Ok(p) => p,
                Err(_) => {
                    send_response(&mut stream, "400 Bad Request", "{\"error\":\"Invalid JSON\"}");
                    return;
                }
            };

            let message = serde_json::to_string(&payload).unwrap();
            match agent_service.chat(&message, None).await {
                Ok(response) => {
                    let json = serde_json::json!({
                        "received": true,
                        "response": response
                    });
                    send_response(&mut stream, "200 OK", &json.to_string());
                }
                Err(_) => {
                    send_response(&mut stream, "500 Internal Server Error", "{\"error\":\"Webhook failed\"}");
                }
            }
        }
        ("POST", "/sekuire/hello") => {
            // TODO: Implement Sekuire protocol handshake
            let json = serde_json::json!({
                "agent_id": "agent-placeholder",
                "agent_nonce": "placeholder-nonce",
                "signature_c": "placeholder-signature",
                "credentials": []
            });
            send_response(&mut stream, "200 OK", &json.to_string());
        }
        ("POST", "/sekuire/auth") => {
            // TODO: Implement Sekuire protocol authentication
            let json = serde_json::json!({ "success": true });
            send_response(&mut stream, "200 OK", &json.to_string());
        }
        _ => {
            send_response(&mut stream, "404 Not Found", "{\"error\":\"Not found\"}");
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load environment variables
    dotenv::dotenv().ok();

    // Initialize agent service
    let agent_service = Arc::new(AgentService::new());
    agent_service.initialize(None).await?;

    // Start server
    let port = std::env::var("PORT").unwrap_or_else(|_| "8000".to_string());
    let host = std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let addr = format!("{}:{}", host, port);

    println!("🚀 Server running on http://{}", addr);

    let listener = TcpListener::bind(&addr)?;

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let agent_service = agent_service.clone();
                tokio::spawn(async move {
                    handle_client(stream, agent_service).await;
                });
            }
            Err(e) => {
                eprintln!("Connection failed: {}", e);
            }
        }
    }

    Ok(())
}
"#;

/// Rust CLI interface template
pub const RUST_CLI_INTERFACE: &str = r#"mod agent;

use agent::AgentService;
use std::io::{self, Write};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("🤖 Sekuire Agent CLI\n");

    // Initialize agent service
    let agent_service = AgentService::new();
    agent_service.initialize(None).await?;

    let agent_name = agent_service.get_name().await?;
    let model_name = agent_service.get_model_name().await?;

    println!("Agent: {}", agent_name);
    println!("Model: {}", model_name);
    println!("\nType your message (or \"exit\" to quit)\n");

    let stdin = io::stdin();
    let mut stdout = io::stdout();

    loop {
        print!("> ");
        stdout.flush()?;

        let mut input = String::new();
        stdin.read_line(&mut input)?;

        let input = input.trim();

        if input.is_empty() {
            continue;
        }

        if input.eq_ignore_ascii_case("exit") {
            println!("Goodbye!");
            break;
        }

        match agent_service.chat(input, None).await {
            Ok(response) => {
                println!("\nAssistant: {}\n", response);
            }
            Err(e) => {
                eprintln!("\n❌ Error: {}\n", e);
            }
        }
    }

    Ok(())
}
"#;

/// Rust hybrid entry point (mode router)
pub const RUST_HYBRID_MAIN: &str = r#"mod agent;
mod cli;
mod server;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load environment variables
    dotenv::dotenv().ok();

    let mode = std::env::var("MODE").unwrap_or_else(|_| "cli".to_string());

    match mode.to_lowercase().as_str() {
        "http" => {
            server::run().await
        }
        _ => {
            cli::run().await
        }
    }
}
"#;

/// Get the correct Rust HTTP server template based on framework
pub fn get_rust_http_server_template(framework: &str) -> &'static str {
    match framework {
        "axum" => RUST_HTTP_AXUM,
        "actix" => RUST_HTTP_ACTIX,
        "built-in" => RUST_HTTP_BUILTIN,
        _ => RUST_HTTP_AXUM, // Default to Axum
    }
}

/// Generate Cargo.toml for Rust HTTP agents
pub fn get_rust_cargo_http(project_name: &str, framework: &str) -> String {
    let package_name = project_name.to_lowercase().replace(" ", "-");

    let framework_deps = match framework {
        "axum" => {
            r#"axum = "0.7"
tokio = { version = "1.0", features = ["full"] }
tower-http = { version = "0.5", features = ["cors"] }"#
        }
        "actix" => {
            r#"actix-web = "4.4"
actix-cors = "0.7"
tokio = { version = "1.0", features = ["full"] }"#
        }
        "built-in" => r#"tokio = { version = "1.0", features = ["full"] }"#,
        _ => {
            r#"axum = "0.7"
tokio = { version = "1.0", features = ["full"] }
tower-http = { version = "0.5", features = ["cors"] }"#
        }
    };

    format!(
        r#"[package]
name = "{}"
version = "0.1.0"
edition = "2021"

[dependencies]
sekuire-sdk = "0.1.0"
{}
serde = {{ version = "1.0", features = ["derive"] }}
serde_json = "1.0"
anyhow = "1.0"
dotenv = "0.15"
"#,
        package_name, framework_deps
    )
}

pub fn helm_chart_yaml(chart_name: &str, app_version: &str) -> String {
    format!(
        r#"apiVersion: v2
name: {chart_name}
description: Sekuire agent BYOC deployment for {chart_name}
type: application
version: 0.1.0
appVersion: "{app_version}"
"#
    )
}

pub fn helm_helpers_tpl(chart_name: &str) -> String {
    let tpl = r#"{{- define "__CHART__.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" -}}
{{- end -}}

{{- define "__CHART__.fullname" -}}
{{- if .Values.fullnameOverride -}}
{{- .Values.fullnameOverride | trunc 63 | trimSuffix "-" -}}
{{- else -}}
{{- $name := include "__CHART__.name" . -}}
{{- if contains $name .Release.Name -}}
{{- $name | trunc 63 | trimSuffix "-" -}}
{{- else -}}
{{- printf "%s-%s" .Release.Name $name | trunc 63 | trimSuffix "-" -}}
{{- end -}}
{{- end -}}
{{- end -}}
"#;
    tpl.replace("__CHART__", chart_name)
}

pub fn helm_deployment_yaml(chart_name: &str) -> String {
    let tpl = r#"apiVersion: apps/v1
kind: Deployment
metadata:
  name: {{ include "__CHART__.fullname" . }}
  labels:
    app.kubernetes.io/name: {{ include "__CHART__.name" . }}
    app.kubernetes.io/instance: {{ .Release.Name }}
spec:
  replicas: {{ .Values.replicaCount }}
  selector:
    matchLabels:
      app.kubernetes.io/name: {{ include "__CHART__.name" . }}
      app.kubernetes.io/instance: {{ .Release.Name }}
  template:
    metadata:
      labels:
        app.kubernetes.io/name: {{ include "__CHART__.name" . }}
        app.kubernetes.io/instance: {{ .Release.Name }}
      annotations:
{{ toYaml .Values.podAnnotations | indent 8 }}
    spec:
      containers:
        - name: {{ include "__CHART__.name" . }}
          image: {{ .Values.image.repository }}{{- if .Values.image.digest }}@{{ .Values.image.digest }}{{- else if .Values.image.tag }}:{{ .Values.image.tag }}{{- end }}
          imagePullPolicy: {{ .Values.image.pullPolicy }}
          env:
            - name: SEKUIRE_WORKSPACE_ID
              value: "{{ .Values.workspaceId }}"
            - name: SEKUIRE_AGENT_ID
              value: "{{ .Values.agentId }}"
            - name: SEKUIRE_INSTALL_TOKEN
              value: "{{ .Values.installToken }}"
            - name: SEKUIRE_A2A_UPSTREAM_URL
              value: "{{ .Values.upstreamUrl | default \"\" }}"
          {{- if or .Values.env .Values.secrets }}
          envFrom:
            {{- if .Values.env }}
            - configMapRef:
                name: {{ include ".fullname" . }}-env
            {{- end }}
            {{- if .Values.secrets }}
            - secretRef:
                name: {{ include ".fullname" . }}-secrets
            {{- end }}
          {{- end }}
          ports:
            - containerPort: {{ .Values.containerPort }}
          resources:
{{ toYaml .Values.resources | indent 12 }}
"#;
    tpl.replace("__CHART__", chart_name)
}

pub fn helm_service_yaml(chart_name: &str) -> String {
    let tpl = r#"{{- if .Values.service.enabled }}
apiVersion: v1
kind: Service
metadata:
  name: {{ include "__CHART__.fullname" . }}
  labels:
    app.kubernetes.io/name: {{ include "__CHART__.name" . }}
    app.kubernetes.io/instance: {{ .Release.Name }}
spec:
  type: {{ .Values.service.type }}
  ports:
    - port: {{ .Values.service.port }}
      targetPort: {{ .Values.service.targetPort }}
      protocol: TCP
      name: http
  selector:
    app.kubernetes.io/name: {{ include "__CHART__.name" . }}
    app.kubernetes.io/instance: {{ .Release.Name }}
{{- end }}
"#;
    tpl.replace("__CHART__", chart_name)
}

pub fn helm_configmap_yaml(chart_name: &str) -> String {
    let tpl = r#"{{- if .Values.env }}
apiVersion: v1
kind: ConfigMap
metadata:
  name: {{ include "__CHART__.fullname" . }}-env
data:
{{- range $key, $value := .Values.env }}
  {{ $key }}: "{{ $value }}"
{{- end }}
{{- end }}
"#;
    tpl.replace("__CHART__", chart_name)
}

pub fn helm_secret_yaml(chart_name: &str) -> String {
    let tpl = r#"{{- if .Values.secrets }}
apiVersion: v1
kind: Secret
metadata:
  name: {{ include "__CHART__.fullname" . }}-secrets
type: Opaque
stringData:
{{- range $key, $value := .Values.secrets }}
  {{ $key }}: "{{ $value }}"
{{- end }}
{{- end }}
"#;
    tpl.replace("__CHART__", chart_name)
}

pub fn helm_readme(chart_name: &str, workspace_id: &str, agent_id: &str) -> String {
    format!(
        r#"# Sekuire Agent Helm Chart

This chart deploys a Sekuire agent as a BYOC installation.

## Prerequisites
- Kubernetes cluster with kubectl + Helm installed
- Sekuire install token for workspace `{workspace_id}`

## Quickstart
```bash
helm upgrade --install {chart_name} . \\
  --namespace default \\
  --create-namespace
```

## What to edit
- `values.yaml`
  - `installToken` (if not pre-filled)
  - `image.repository`/`image.tag` or `image.digest`
  - `env` for non-secret values
  - `secrets` for secret env vars (will be stored in a Secret)

## Important values
- `workspaceId`: {workspace_id}
- `agentId`: {agent_id}
- `installToken`: short-lived token used at startup to register the agent

## Cleanup
```bash
helm uninstall {chart_name}
```
"#
    )
}

// =============================================================================
// Docker / Local Development Templates
// =============================================================================

/// Generate standalone docker-compose.yml for local development
/// Bundles Core + Postgres + Redis + Agent for complete dev environment
pub fn get_agent_docker_compose(agent_name: &str, port: u16, language: &str) -> String {
    let dev_command = match language {
        "Python" | "python" => "python main.py",
        "Rust" | "rust" => "cargo watch -x run",
        _ => "npm run dev", // TypeScript default
    };

    format!(
        r#"# Sekuire Agent - Local Development Environment
#
# USAGE:
#   Option 1 - Connect to existing Core (if Core is already running):
#     docker compose up agent
#     # Agent connects to SEKUIRE_API_URL (default: http://host.docker.internal:9300)
#
#   Option 2 - Standalone (full stack in Docker):
#     docker compose --profile standalone up
#     # Starts Core + Postgres + Redis + Agent
#
# Agent: http://localhost:{port}

name: {agent_name}-dev

services:
  # Your Agent (development mode with hot-reload)
  agent:
    build:
      context: .
      dockerfile: Dockerfile.dev
    ports:
      - "{port}:8000"
    environment:
      # If running standalone profile, connects to sekuire-core container
      # If running agent only, connects to host machine's Core
      SEKUIRE_API_URL: ${{SEKUIRE_API_URL:-http://host.docker.internal:9300}}
      SEKUIRE_WORKSPACE_ID: ${{SEKUIRE_WORKSPACE_ID:-dev-workspace}}
      DEV_MODE: "true"
      PORT: "8000"
    env_file:
      - .env
    volumes:
      - .:/app
      - /app/node_modules
    command: {dev_command}

  # === Standalone Profile Services ===
  # Use: docker compose --profile standalone up

  sekuire-core:
    image: sekuire/core:latest
    profiles: ["standalone"]
    ports:
      - "9300:3000"
    environment:
      DATABASE_URL: postgres://sekuire:sekuire@postgres:5432/sekuire_dev
      REDIS_URL: redis://redis:6379
      DEV_MODE: "true"
      RUST_LOG: info
    depends_on:
      postgres:
        condition: service_healthy
      redis:
        condition: service_started

  postgres:
    image: postgres:15-alpine
    profiles: ["standalone"]
    environment:
      POSTGRES_USER: sekuire
      POSTGRES_PASSWORD: sekuire
      POSTGRES_DB: sekuire_dev
    volumes:
      - postgres_data:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U sekuire -d sekuire_dev"]
      interval: 5s
      timeout: 5s
      retries: 5

  redis:
    image: redis:7-alpine
    profiles: ["standalone"]
    volumes:
      - redis_data:/data

volumes:
  postgres_data:
  redis_data:
"#,
        agent_name = agent_name.to_lowercase().replace(' ', "-"),
        port = port,
        dev_command = dev_command
    )
}

/// Generate development Dockerfile with hot-reload support
pub fn get_agent_dockerfile_dev(language: &str) -> String {
    match language {
        "Python" | "python" => r#"# Development Dockerfile - Hot Reload Enabled
FROM python:3.12-slim

WORKDIR /app

# Install dependencies
COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt watchfiles

# Copy source code
COPY . .

ENV PORT=8000
EXPOSE 8000

# Hot-reload with watchfiles
CMD ["python", "-m", "watchfiles", "main.main", "."]
"#
        .to_string(),

        "Rust" | "rust" => r#"# Development Dockerfile - Hot Reload Enabled
FROM rust:1.75-alpine

WORKDIR /app

# Install build dependencies and cargo-watch
RUN apk add --no-cache musl-dev
RUN cargo install cargo-watch

# Copy manifests and build dependencies
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs && cargo build
RUN rm -rf src

# Copy source code
COPY . .

ENV PORT=8000
EXPOSE 8000

# Hot-reload with cargo-watch
CMD ["cargo", "watch", "-x", "run"]
"#
        .to_string(),

        // TypeScript (default)
        _ => r#"# Development Dockerfile - Hot Reload Enabled
FROM node:22-alpine

WORKDIR /app

# Install tsx for TypeScript execution
RUN npm install -g tsx

# Copy package files and install dependencies
COPY package*.json ./
RUN npm install

# Copy source code
COPY . .

ENV PORT=8000
EXPOSE 8000

# Hot-reload with tsx watch
CMD ["npm", "run", "dev"]
"#
        .to_string(),
    }
}

/// Generate production Dockerfile (used by Sekuire's server-side build system)
pub fn get_agent_dockerfile_prod(language: &str) -> String {
    match language {
        "Python" | "python" => r#"# Production Build - Used by Sekuire Build System
# Multi-stage build for minimal image size

FROM python:3.12-slim AS builder
WORKDIR /app
COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

FROM python:3.12-slim AS runtime

# Install curl for healthcheck
RUN apt-get update && apt-get install -y --no-install-recommends curl \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m sekuire
WORKDIR /app

# Copy installed packages from builder
COPY --from=builder /usr/local/lib/python3.12/site-packages /usr/local/lib/python3.12/site-packages

# Copy application files
COPY --chown=sekuire:sekuire . .

USER sekuire

ENV PORT=8000
EXPOSE 8000

HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8000/health || exit 1

CMD ["python", "main.py"]
"#
        .to_string(),

        "Rust" | "rust" => r#"# Production Build - Used by Sekuire Build System
# Multi-stage build for minimal image size

FROM rust:1.75 AS builder
WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock ./
COPY src ./src

# Build release binary
RUN cargo build --release

FROM debian:bookworm-slim AS runtime

# Install runtime dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m sekuire
WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/agent ./agent

# Copy configuration files
COPY --chown=sekuire:sekuire sekuire.yml system_prompt.md tools.json ./

USER sekuire

ENV PORT=8000
EXPOSE 8000

HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8000/health || exit 1

CMD ["./agent"]
"#
        .to_string(),

        // TypeScript (default)
        _ => r#"# Production Build - Used by Sekuire Build System
# Multi-stage build for minimal image size

FROM node:22-alpine AS builder
WORKDIR /app

# Copy package files
COPY package*.json ./

# Install dependencies (including devDependencies for build)
RUN npm ci && npm cache clean --force

# Copy source code
COPY . .

# Build TypeScript
RUN npm run build

FROM node:22-alpine AS runtime

# Install curl for healthcheck
RUN apk add --no-cache curl

# Create non-root user
RUN addgroup -g 1001 sekuire && adduser -S sekuire -u 1001
WORKDIR /app

# Copy built application
COPY --from=builder --chown=sekuire:sekuire /app/dist ./dist
COPY --from=builder --chown=sekuire:sekuire /app/node_modules ./node_modules

# Copy configuration files
COPY --chown=sekuire:sekuire sekuire.yml system_prompt.md tools.json ./

USER sekuire

ENV NODE_ENV=production
ENV PORT=8000
EXPOSE 8000

HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8000/health || exit 1

CMD ["node", "dist/index.js"]
"#
        .to_string(),
    }
}

/// Generate .dockerignore file to exclude unnecessary files from Docker builds
pub fn get_dockerignore() -> &'static str {
    r#"# Dependencies (rebuilt in container)
node_modules/
target/
__pycache__/
*.pyc
.venv/
venv/

# Development files
.git/
.gitignore
.env
.env.local
*.log
docker-compose.yml
Dockerfile.dev

# IDE and editor files
.vscode/
.idea/
*.swp
*.swo
*~

# Build artifacts
dist/
build/
*.egg-info/

# Test files
*.test.ts
*.spec.ts
__tests__/
tests/
coverage/

# Secrets (NEVER include in image)
.sekuire/secret.key

# OS files
.DS_Store
Thumbs.db
"#
}

pub fn get_github_workflow(agent_name: &str, language: &str) -> String {
    let build_steps = match language {
        "rust" => {
            r#"      - uses: dtolnay/rust-toolchain@stable
      - run: cargo build --release
      - name: Build Docker image
        run: docker build -t ${{ env.IMAGE_NAME }}:${{ github.sha }} ."#
        }
        "python" => {
            r#"      - uses: actions/setup-python@v5
        with:
          python-version: '3.11'
      - run: pip install -r requirements.txt
      - name: Build Docker image
        run: docker build -t ${{ env.IMAGE_NAME }}:${{ github.sha }} ."#
        }
        _ => {
            r#"      - uses: actions/setup-node@v4
        with:
          node-version: '20'
      - run: npm ci
      - run: npm run build
      - name: Build Docker image
        run: docker build -t ${{ env.IMAGE_NAME }}:${{ github.sha }} ."#
        }
    };

    format!(
        r#"name: Sekuire Build & Push

on:
  push:
    branches: [main, master]
  workflow_dispatch:

env:
  REGISTRY: registry.sekuire.ai
  IMAGE_NAME: {agent_name}

jobs:
  build-and-push:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

{build_steps}

      - name: Login to Sekuire Registry
        run: |
          echo "${{{{ secrets.SEKUIRE_PUSH_TOKEN }}}}" | \
            docker login ${{{{ env.REGISTRY }}}} -u token --password-stdin

      - name: Tag and push
        run: |
          docker tag ${{{{ env.IMAGE_NAME }}}}:${{{{ github.sha }}}} \
            ${{{{ env.REGISTRY }}}}/${{{{ env.IMAGE_NAME }}}}:${{{{ github.sha }}}}
          docker tag ${{{{ env.IMAGE_NAME }}}}:${{{{ github.sha }}}} \
            ${{{{ env.REGISTRY }}}}/${{{{ env.IMAGE_NAME }}}}:latest
          docker push ${{{{ env.REGISTRY }}}}/${{{{ env.IMAGE_NAME }}}}:${{{{ github.sha }}}}
          docker push ${{{{ env.REGISTRY }}}}/${{{{ env.IMAGE_NAME }}}}:latest

      - name: Trigger vulnerability scan
        if: success()
        run: |
          curl -s -X POST \
            -H "X-API-Key: ${{{{ secrets.SEKUIRE_API_KEY }}}}" \
            "${{{{ secrets.SEKUIRE_BACKEND_URL }}}}/api/v1/images/${{{{ env.IMAGE_NAME }}}}:${{{{ github.sha }}}}/scan" \
            || true
"#,
        agent_name = agent_name,
        build_steps = build_steps,
    )
}

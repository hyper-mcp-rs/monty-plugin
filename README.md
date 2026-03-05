# Rust Plugin Template

A WebAssembly plugin template for building MCP (Model Context Protocol) plugins in Rust using the hyper-mcp framework.

## Overview

This template provides a starter project for creating MCP plugins that run as WebAssembly modules. It includes all necessary dependencies and boilerplate code to implement MCP protocol handlers.

## Project Structure

```
.
├── .github/workflows    # Example Github Actions workflows
|-- src/
│   ├── lib.rs           # Main plugin implementation
│   └── pdk/             # Plugin Development Kit types and utilities
├── Cargo.toml           # Rust dependencies and project metadata
├── Dockerfile           # Simple dockerfile for deploying to WASM
└── .cargo/              # Cargo configuration
```

## Getting Started

### Prerequisites

- Rust 1.88 or later
- `wasm32-wasip1` target installed:
  ```sh
  rustup target add wasm32-wasip1
  ```

### Development

1. **Clone or use this template** to start your plugin project

2. **Implement plugin handlers** in `src/lib.rs`:

   > **Note:** You only need to implement the handlers relevant to your plugin. For example, if your plugin only provides tools, implement only `list_tools()` and `call_tool()`. All other handlers have default implementations that work out of the box.

   - `list_tools()` - Describe available tools
   - `call_tool()` - Execute a tool
   - `list_resources()` - List available resources
   - `read_resource()` - Read resource contents
   - `list_prompts()` - List available prompts
   - `get_prompt()` - Get prompt details
   - `complete()` - Provide auto-completion suggestions

3. **Build locally** (requires WASM target):
   ```sh
   cargo build --release --target wasm32-wasip1
   ```
   The compiled WASM module will be at: `target/wasm32-wasip1/release/plugin.wasm`

### Dependencies

The template includes key dependencies:

- **extism-pdk** - Plugin Development Kit for Extism
- **serde/serde_json** - JSON serialization/deserialization
- **anyhow** - Error handling
- **base64** - Base64 encoding/decoding
- **chrono** - Date/time handling

## Plugin Handler Functions

Your plugin can implement any combination of the following handlers. **Only implement the handlers your plugin needs** - the template provides sensible defaults for everything else:

| Handler | Purpose | Required For |
|---------|---------|--------------|
| `list_tools()` | Declare available tools | Tool-providing plugins |
| `call_tool()` | Execute a tool | Tool-providing plugins |
| `list_resources()` | Declare available resources | Resource-providing plugins |
| `list_resource_templates()` | Declare resource templates | Dynamic resource plugins |
| `read_resource()` | Read resource contents | Resource-providing plugins |
| `list_prompts()` | Declare available prompts | Prompt-providing plugins |
| `get_prompt()` | Retrieve a specific prompt | Prompt-providing plugins |
| `complete()` | Provide auto-completions | Plugins supporting completions |
| `on_roots_list_changed()` | Handle root changes | Plugins reacting to root changes |

**Example: Tools-only plugin**

If your plugin only provides tools, you only need to implement:

```rust
pub(crate) fn list_tools(_input: ListToolsRequest) -> Result<ListToolsResult> {
    // Return your tools
}

pub(crate) fn call_tool(input: CallToolRequest) -> Result<CallToolResult> {
    // Execute the requested tool
}
```

All other handlers will use their default implementations.

## Host Functions

Your plugin can call these host functions to interact with the client and MCP server. Import them from the `pdk` module:

```rust
use crate::pdk::imports::*;
```

### User Interaction

**`create_elicitation(input: ElicitRequestParamWithTimeout) -> Result<ElicitResult>`**

Request user input through the client's elicitation interface. Use this when your plugin needs user guidance, decisions, or confirmations during execution.

```rust
let result = create_elicitation(ElicitRequestParamWithTimeout {
    request: ElicitRequestParam {
        // Define what input you're requesting
        ..Default::default()
    },
    timeout_ms: Some(30000), // 30 second timeout
})?;
```

### Message Generation

**`create_message(input: CreateMessageRequestParam) -> Result<CreateMessageResult>`**

Request message creation through the client's sampling interface. Use this when your plugin needs intelligent text generation or analysis with AI assistance.

```rust
let result = create_message(CreateMessageRequestParam {
    messages: vec![/* conversation history */],
    model_preferences: Some(/* model preferences */),
    system: Some("You are a helpful assistant".to_string()),
    ..Default::default()
})?;
```

### Resource Discovery

**`list_roots() -> Result<ListRootsResult>`**

List the client's root directories or resources. Use this to discover what root resources (typically file system roots) are available and understand the scope of resources your plugin can access.

```rust
let roots = list_roots()?;
for root in roots.roots {
    println!("Root: {} at {}", root.name, root.uri);
}
```

### Logging

**`notify_logging_message(input: LoggingMessageNotificationParam) -> Result<()>`**

Send diagnostic, informational, warning, or error messages to the client. The client's logging level determines which messages are processed and displayed.

```rust
notify_logging_message(LoggingMessageNotificationParam {
    level: "info".to_string(),
    logger: Some("my_plugin".to_string()),
    data: serde_json::json!({"message": "Processing started"}),
})?;
```

### Progress Reporting

**`notify_progress(input: ProgressNotificationParam) -> Result<()>`**

Report progress during long-running operations. Allows clients to display progress bars or status information to users.

```rust
notify_progress(ProgressNotificationParam {
    progress: 50,
    total: Some(100),
})?;
```

### List Change Notifications

Notify the client when your plugin's available items change:

**`notify_tool_list_changed() -> Result<()>`**
- Call this when you add, remove, or modify available tools

**`notify_resource_list_changed() -> Result<()>`**
- Call this when you add, remove, or modify available resources

**`notify_prompt_list_changed() -> Result<()>`**
- Call this when you add, remove, or modify available prompts

**`notify_resource_updated(input: ResourceUpdatedNotificationParam) -> Result<()>`**
- Call this when you modify the contents of a specific resource

```rust
// When your plugin's tools change
notify_tool_list_changed()?;

// When a specific resource is updated
notify_resource_updated(ResourceUpdatedNotificationParam {
    uri: "resource://my-resource".to_string(),
})?;
```

### Example: Interactive Tool with Progress

```rust
pub(crate) fn call_tool(input: CallToolRequest) -> Result<CallToolResult> {
    match input.name.as_str() {
        "long_task" => {
            // Log start
            notify_logging_message(LoggingMessageNotificationParam {
                level: "info".to_string(),
                data: serde_json::json!({"message": "Starting long task"}),
                ..Default::default()
            })?;

            // Do work with progress updates
            for i in 0..10 {
                // ... do work ...
                notify_progress(ProgressNotificationParam {
                    progress: (i + 1) * 10,
                    total: Some(100),
                })?;
            }

            Ok(CallToolResult {
                content: vec![Content {
                    type_: "text".to_string(),
                    text: Some("Task completed".to_string()),
                    ..Default::default()
                }],
                ..Default::default()
            })
        },
        _ => Err(anyhow!("Unknown tool")),
    }
}
```

## Building for Distribution

### Using Docker

The included `Dockerfile` provides a simple build that packages your plugin into a container:

```sh
cargo auditable build --release --target wasm32-wasip1
cp target/wasm32-wasip1/release/plugin.wasm plugin.wasm
docker push your-registry/your-plugin-name
```

### Manual Build

To build manually without Docker:

```sh
# Install dependencies
rustup target add wasm32-wasip1
cargo install cargo-auditable

# Build
cargo auditable build --release --target wasm32-wasip1

# Result is at: target/wasm32-wasip1/release/plugin.wasm
```

## Implementation Guide

### Creating a Tool

Here's an example of implementing a simple tool:

```rust
pub(crate) fn list_tools(_input: ListToolsRequest) -> Result<ListToolsResult> {
    Ok(ListToolsResult {
        tools: vec![
            Tool {
                name: "greet".to_string(),
                description: Some("Greet a person".to_string()),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "name": {
                            "type": "string",
                            "description": "The person's name"
                        }
                    },
                    "required": ["name"]
                }),
            },
        ],
        ..Default::default()
    })
}

pub(crate) fn call_tool(input: CallToolRequest) -> Result<CallToolResult> {
    match input.name.as_str() {
        "greet" => {
            let name = input.arguments
                .get("name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("name argument required"))?;

            Ok(CallToolResult {
                content: vec![Content {
                    type_: "text".to_string(),
                    text: Some(format!("Hello, {}!", name)),
                    ..Default::default()
                }],
                ..Default::default()
            })
        },
        _ => Err(anyhow!("Unknown tool: {}", input.name)),
    }
}
```

### Creating a Resource

Example of implementing a resource:

```rust
pub(crate) fn list_resources(_input: ListResourcesRequest) -> Result<ListResourcesResult> {
    Ok(ListResourcesResult {
        resources: vec![
            ResourceDescription {
                uri: "resource://example".to_string(),
                name: Some("Example Resource".to_string()),
                description: Some("An example resource".to_string()),
                mime_type: Some("text/plain".to_string()),
            },
        ],
        ..Default::default()
    })
}

pub(crate) fn read_resource(input: ReadResourceRequest) -> Result<ReadResourceResult> {
    match input.uri.as_str() {
        "resource://example" => Ok(ReadResourceResult {
            contents: vec![ResourceContents {
                mime_type: Some("text/plain".to_string()),
                text: Some("Resource content here".to_string()),
                ..Default::default()
            }],
        }),
        _ => Err(anyhow!("Unknown resource: {}", input.uri)),
    }
}
```

## Configuration in hyper-mcp

After building and publishing your plugin, configure it in hyper-mcp:

```json
{
  "plugins": {
    "my_plugin": {
      "url": "oci://your-registry/your-plugin-name:latest"
    }
  }
}
```

For local development/testing:

```json
{
  "plugins": {
    "my_plugin": {
      "url": "file:///path/to/target/wasm32-wasip1/release/plugin.wasm"
    }
  }
}
```

## Testing

To test your plugin locally:

1. Build it: `cargo build --release --target wasm32-wasip1`
2. Update hyper-mcp's config to point to `file://` URL
3. Start hyper-mcp with `RUST_LOG=debug`
4. Test through Claude Desktop, Cursor IDE, or another MCP client

## Resources

- [hyper-mcp Documentation](https://github.com/hyper-mcp-rs/hyper-mcp)
- [MCP Protocol Specification](https://spec.modelcontextprotocol.io/)
- [Extism Plugin Development Kit](https://docs.extism.org/docs/pdk)
- [Example Plugins](https://github.com/search?q=topic%3Aplugins+org%3Ahyper-mcp-rs&type=repositories)

## License

Same as hyper-mcp - Apache 2.0

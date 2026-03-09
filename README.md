# monty-plugin

A [hyper-mcp](https://github.com/pydantic/hyper-mcp) plugin that executes Python code in a sandboxed [Monty](https://github.com/pydantic/monty) interpreter, compiled to WebAssembly.

## Overview

monty-plugin exposes a single MCP tool called **`run`** that accepts Python source code (and optional input variables), executes it inside the Monty interpreter, and returns the captured stdout output along with the result value.

Because the plugin compiles to a `.wasm` module (`wasm32-wasip1`), it runs in a fully sandboxed environment — there is no access to the host system beyond what the plugin explicitly provides.

## Tool: `run`

### Parameters

| Name     | Type                          | Required | Description                                    |
| -------- | ----------------------------- | -------- | ---------------------------------------------- |
| `code`   | `string`                      | ✅       | The Python code to execute.                    |
| `inputs` | `map<string, MontyObject>`    | No       | Input variables passed into the Python runtime. |

### Response

| Field    | Type          | Description                                              |
| -------- | ------------- | -------------------------------------------------------- |
| `output` | `string`      | Captured print/stdout output produced by the Python code. |
| `result` | `MontyObject` | The return value of the Python code.                     |

## Built-in Functions

The following functions are available to Python code running inside the plugin:

### `http_request`

```text
http_request(
    url: str,
    method: str | None = None,
    headers: dict[str, str] | None = None,
    body: str | bytes | None = None,
) -> tuple[int, dict[str, str], str | bytes]
```

Make HTTP requests from within the sandbox. Returns a tuple of `(status_code, response_headers, response_body)`.

### `notify_progress`

```text
notify_progress(
    message: str | None,
    progress: int | float,
    total: int | float | None = None,
) -> None
```

Report progress back to the MCP client. Requires a `progressToken` in the request context.

## Supported `pathlib.Path` Operations

The plugin implements a subset of Python's `pathlib.Path` API:

- `Path.exists(*, follow_symlinks=True) -> bool`
- `Path.is_file(*, follow_symlinks=True) -> bool`
- `Path.is_dir(*, follow_symlinks=True) -> bool`
- `Path.is_symlink() -> bool`
- `Path.read_text(encoding=None, errors=None, newline=None) -> str`
- `Path.read_bytes() -> bytes`
- `Path.write_text(data, encoding=None, errors=None, newline=None) -> None`
- `Path.write_bytes(data) -> None`
- `Path.mkdir(mode=0o777, parents=False, exist_ok=False) -> None`
- `Path.unlink(missing_ok=False) -> None`
- `Path.rmdir() -> None`
- `Path.iterdir() -> list[str]`
- `Path.stat(*, follow_symlinks=True) -> os.stat_result`
- `Path.rename(target) -> None`
- `Path.resolve(strict=False) -> str`
- `Path.absolute() -> str`

## Supported `os` Operations

- `os.environ -> dict[str, str]`
- `os.getenv(key, default=None) -> str | None`

## Building

### Prerequisites

- **Rust 1.94+** (pinned via `rust-toolchain.toml`)
- The `wasm32-wasip1` target

### Install the WASM target

```sh
rustup target add wasm32-wasip1
```

### Build the plugin

```sh
cargo build --release --target wasm32-wasip1
```

The compiled plugin will be at `target/wasm32-wasip1/release/plugin.wasm`.

## Usage with hyper-mcp

Add the plugin to your hyper-mcp configuration. You can reference the OCI artifact published to GitHub Container Registry:

```json
{
  "name": "monty",
  "oci": "ghcr.io/pydantic/monty-plugin:latest"
}
```

Or pin to a specific version tag:

```json
{
  "name": "monty",
  "oci": "ghcr.io/pydantic/monty-plugin:v0.1.0"
}
```

You can pull the artifact directly with [ORAS](https://oras.land/):

```sh
oras pull ghcr.io/pydantic/monty-plugin:latest
```

All release artifacts are signed with [Cosign](https://docs.sigstore.dev/cosign/overview/). Verify with:

```sh
cosign verify \
  --certificate-identity-regexp "https://github.com/pydantic/monty-plugin/.github/workflows/" \
  --certificate-oidc-issuer-regexp "https://token.actions.githubusercontent.com" \
  ghcr.io/pydantic/monty-plugin:latest
```

## Docker

A minimal scratch-based Docker image is provided for packaging the `.wasm` artifact:

```sh
# First build the plugin
cargo build --release --target wasm32-wasip1
cp target/wasm32-wasip1/release/plugin.wasm .

# Then build the Docker image
docker build -t monty-plugin .
```

## Project Structure

```text
monty-plugin/
├── src/
│   ├── lib.rs                   # MCP entry-points and Monty execution loop
│   ├── types.rs                 # RunArguments, RunResponse, MontyObject wrapper
│   ├── function_calls.rs        # http_request, notify_progress implementations
│   ├── os_calls.rs              # pathlib.Path and os.* operation handlers
│   ├── python_args.rs           # Positional/keyword argument resolution helpers
│   ├── monty_object.schema.json # JSON Schema for MontyObject
│   └── pdk/                     # Plugin Development Kit glue
│       ├── mod.rs
│       ├── exports.rs           # Extism-exported MCP handler functions
│       ├── imports.rs           # Host-imported functions
│       └── types.rs             # MCP protocol types
├── Cargo.toml
├── Dockerfile
├── rust-toolchain.toml
└── LICENSE
```

## License

This project is licensed under the [Apache License 2.0](LICENSE).

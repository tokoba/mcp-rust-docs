# MCP Rust Docs Server

A Model Context Protocol (MCP) server that provides comprehensive access to Rust crate documentation and metadata. This server enables AI agents to search for crates on crates.io and retrieve detailed documentation from docs.rs.

## Usage

You can run the MCP Rust Docs Server using either Node.js or Rust:

### Using npm

```json
{
  "servers": {
    "mcp-rust-docs": {
      "command": "pnpx",
      "args": ["mcp-rust-docs@latest"]
    }
  }
}
```

### Using cargo

First, install the server with Cargo:

```bash
cargo install mpc-rust-docs
```

Then start the server:

```json
{
  "servers": {
    "mcp-rust-docs": {
      "command": "mcp-rust-docs"
    }
  }
}
```

## Features

### 🔍 Tools

The server provides 5 powerful tools for Rust documentation exploration:

1. **`search_crate`** - Search for crates on crates.io by name
2. **`retrieve_documentation_index_page`** - Get the main documentation page for a crate
3. **`retrieve_documentation_all_items`** - List all items (structs, enums, functions, etc.) in a crate
4. **`search_documentation_items`** - Fuzzy search for specific items within a crate's documentation
5. **`retrieve_documentation_page`** - Retrieve specific documentation pages by exact path

### 📚 Resources

- **Instruction Resource** (`str://mcp-rust-docs/instruction`) - Provides mandatory usage guidelines for AI agents when handling Rust documentation queries

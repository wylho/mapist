//! Standalone MCP server entry point.
//!
//! The desktop application also serves MCP, from its own binary with `--mcp`.
//! This thin binary exists so the server can run without the GUI — on a
//! headless machine, or from a script.

fn main() {
    std::process::exit(mapist_mcp::serve());
}

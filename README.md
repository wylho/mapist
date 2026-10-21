# Mapist

A mind-mapping application in Rust and Tauri, derived from
[Minder](https://github.com/phase1geo/Minder) by Trevor Williams.

Mapist keeps Minder's file format so existing maps open unchanged, and adds a
built-in Model Context Protocol server so that agents can read and edit maps
directly.

## Status

Phase 1 — core. The data model, layout engine, document I/O, undo history, and
MCP server are implemented and tested. The Tauri desktop shell is not built yet.

| Component | State |
|---|---|
| `mapist-core` — model, layout, document I/O, undo | Implemented, 15 tests |
| `mapist-mcp` — MCP server over stdio | Implemented, 19 tests |
| `mapist-app` — Tauri desktop shell | Not started |

## Layout

```
crates/
├── mapist-core/     model, layout, XML document I/O, undo — no UI
└── mapist-mcp/      Model Context Protocol server over stdio
```

`mapist-core` is deliberately UI-free. The desktop application and the MCP
server embed exactly the same semantics, so an agent editing a map through MCP
produces the same result as a person editing it in the window.

## Building

Requires Rust 1.80 or newer.

```bash
cargo build --release
cargo test
```

The MCP server binary lands at `target/release/mapist-mcp`.

## File format

Mapist reads and writes the `.minder` XML format used by Minder 2.0.9
(`<minder version="1.15.0">`). Parsing ignores unknown attributes so that files
from newer Minder releases still open.

A round trip is lossless for the fields Mapist models: node text, notes,
colours, sides, layouts, styles, theme, and viewport.

```bash
cargo run -p mapist-core --example parse_real /path/to/map.minder
```

## MCP server

`mapist-mcp` speaks JSON-RPC 2.0 over stdio, line-delimited. Register it with
any MCP client:

```json
{
  "mcpServers": {
    "mapist": {
      "command": "/absolute/path/to/mapist-mcp",
      "args": []
    }
  }
}
```

### Tools

| Tool | Purpose |
|---|---|
| `new_map` | Create an empty map with a root node |
| `open_map` | Open a `.minder` file |
| `save_map` | Save, optionally to a new path |
| `get_map` | Return the whole tree |
| `get_node` | Return one node and its child ids |
| `add_node` | Add a child under a node |
| `remove_node` | Remove a node and its subtree |
| `set_text` | Replace a node's text |
| `set_note` | Replace a node's note |
| `set_color` | Set or clear a branch colour |
| `set_side` | Place a branch left or right |
| `relayout` | Recompute positions |
| `search_nodes` | Find nodes by text or note |
| `undo` / `redo` | Step through history |
| `map_stats` | Node count, depth, dirty flag |

Every mutating tool goes through the same undo history a person would use, so
an agent's edits are reversible with `undo`.

### Example session

```bash
printf '%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' \
  '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"new_map","arguments":{"title":"Ideas"}}}' \
  '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"add_node","arguments":{"parent":0,"text":"First"}}}' \
  '{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"map_stats","arguments":{}}}' \
  | ./target/release/mapist-mcp
```

## Roadmap

- **Phase 1** — core, MCP server *(current)*
- **Phase 2** — Tauri shell: canvas rendering, keyboard editing, inspectors
- **Phase 3** — import/export: Markdown, OPML, FreeMind, Mermaid, SVG, PNG
- **Phase 4** — themes, animations, focus mode, tabs, i18n

## License

GPL-3.0-or-later, inherited from Minder. See [LICENSE](LICENSE).

Minder is copyright Trevor Williams and contributors. Mapist is a derivative
work and carries the same licence.

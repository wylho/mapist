# Mapist

A mind-mapping application in Rust and Tauri, derived from
[Minder](https://github.com/phase1geo/Minder) by Trevor Williams.

Mapist keeps Minder's file format so existing maps open unchanged, and adds a
built-in Model Context Protocol server so that agents can read and edit maps
directly.

**Full specification: [`docs/SPEC.md`](docs/SPEC.md). Project rules: [`CLAUDE.md`](CLAUDE.md).**
Design prototypes live in [`prototype/`](prototype/).

## Status

Phases 1 and 2 are done. The window opens, draws, and edits maps; the same
binary serves MCP to an agent.

| Component | State |
|---|---|
| `mapist-core` — model, layout, document I/O, undo | Implemented, 15 tests |
| `mapist-mcp` — MCP server over stdio, 16 tools | Implemented, 25 tests |
| `mapist-app` — Tauri shell, 18 commands | Implemented, 5 tests |
| `mapist-app/ui` — Svelte 5 interface | Implemented, 24 tests |

70 tests total. `cargo clippy --all-targets -- -D warnings` and `svelte-check`
are both clean.

## Layout

```
crates/
├── mapist-core/     model, layout, XML document I/O, undo — no UI
├── mapist-mcp/      Model Context Protocol server over stdio
└── mapist-app/      Tauri window, commands, and canvas UI
    └── ui/          HTML/CSS/JS renderer
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

### Preview in a browser

The interface runs in a browser over a simulated backend, with hot reload. This
is the fast path for any visual change — the Tauri build takes minutes and shows
nothing during it.

```bash
cd crates/mapist-app/ui
pnpm install
pnpm dev
```

Then open `http://localhost:1420/?preview`. Edits to `.svelte` files appear
immediately.

### Run the desktop application

```bash
cargo tauri build --config crates/mapist-app/tauri.conf.json
open target/release/bundle/macos/Mapist.app
```

> `cargo build` alone produces a binary with **no interface embedded** — the
> `beforeBuildCommand` that generates the frontend only runs under the Tauri
> CLI, and `cargo build` skips it. The window would open blank. Use
> `cargo tauri build`.

### Using the window

| Input | Effect |
|---|---|
| Click | Select a node |
| Double-click | Select and focus the text field |
| Drag background | Pan |
| Wheel | Zoom |
| <kbd>Tab</kbd> | Add a child |
| <kbd>Enter</kbd> | Add a sibling |
| <kbd>Delete</kbd> | Delete the node and its subtree |
| <kbd>Space</kbd> | Fold or unfold |
| Arrow keys | Move the selection geometrically |
| <kbd>Cmd</kbd>+<kbd>Z</kbd> / <kbd>Cmd</kbd>+<kbd>Shift</kbd>+<kbd>Z</kbd> | Undo / redo |
| <kbd>Cmd</kbd>+<kbd>S</kbd> | Save |


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

- **Phase 1** — core, MCP server *(done)*
- **Phase 2** — Tauri shell: canvas rendering, keyboard editing, inspectors *(done)*
- **Phase 3** — import/export: Markdown, OPML, FreeMind, Mermaid, SVG, PNG
- **Phase 4** — themes, animations, focus mode, tabs, i18n

### Known gaps

- Save and open use a prompt for the path; native file dialogs are not wired.
- Node widths are estimated from character counts, not measured with a real
  font, so boxes can be slightly wide or narrow.
- Connections parse and round-trip but cannot be drawn or edited yet.
- The webview has no window-control or menu integration yet.

## License

GPL-3.0-or-later, inherited from Minder. See [LICENSE](LICENSE).

Minder is copyright Trevor Williams and contributors. Mapist is a derivative
work and carries the same licence.

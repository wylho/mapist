<!-- Same content as the website home page (site/index.html), with the Portuguese copy in README.pt-BR.md and site/pt/index.html. Change one, change the others: skill /sync-site. -->

<p align="center"><b>English</b> · <a href="README.pt-BR.md">Português</a></p>

<p align="center">
  <img src="site/favicon.svg" width="88" height="88" alt="Nodo app icon">
</p>

<h1 align="center">Nodo</h1>

<p align="center"><b>Think in maps.</b><br>
A free, open-source mind mapping app that opens in an instant, works offline and keeps every map in a file you own,
readable without the app. And Claude can work on the same map you do, by the same rules. For Windows, macOS and Linux.</p>

<p align="center"><a href="https://github.com/wylho/nodo/releases/latest"><b>Version 0.4.1: in English, timeline, matrix and tree table, tabs and attachments →</b></a></p>

<p align="center">
  <b>Windows · macOS · Linux</b><br>
  <a href="#download"><b>Download</b></a> ·
  <a href="https://nodo-1c7.pages.dev">nodo-1c7.pages.dev</a> ·
  <a href="https://nodo-1c7.pages.dev/privacy/">Privacy</a> ·
  <a href="README.pt-BR.md">Português</a> ·
  <a href="https://github.com/sponsors/wylho">♥ Sponsor</a>
</p>

<picture>
  <source srcset="site/img/mapa-escuro.webp" media="(prefers-color-scheme: dark)">
  <img src="site/img/mapa.webp" alt="A mind map in Nodo: the central topic in the middle, colorful branches on both sides, topics shown as cards with checklists, tags and status, and a loose note">
</picture>

## Built for fast thinking

All from the keyboard, no waiting. The mind map lays itself out while you type.

| | |
|---|---|
| **Select and type** | Text goes straight into the topic. Tab adds a child, Enter a sibling, both ready for typing; the arrow keys move around the map. |
| **On both sides** | Branches to the left and right of the central topic. Pick each branch’s side from the right-click menu. |
| **Notes on every topic** | Short text on the map, details in a Markdown note saved right along with it. |
| **Real undo** | Two hundred steps back and forth, without losing what you collapsed or moved to the other side. |
| **Search the map** | Finds by text or note, and highlights every matching topic. |
| **Works offline** | No account, no server. Local-first: the map is a file on your computer. |

## Claude on the same map

Nodo has a built-in MCP server, so AI agents like Claude can edit your maps. Turn it on in Settings and ask Claude to
read, create and reorganize your mind maps. It uses the same core as the app: each of its edits follows the same rules
as one made by hand, and it can undo what it did.

- Claude Desktop: one button in Settings
- Claude Code: one command to paste into the terminal
- When Claude saves, the window reloads the map as soon as you return to it

<img src="site/img/claude.webp" alt="Nodo settings with appearance, line style and the Claude section, with the Connect button and the Claude Code command">

## The file is yours

Every map is a `.nodo` file: a zip with the map as readable JSON plus its attachments, which you can open without the
app. Saving never leaves a half-written file. Minder (`.minder`), Markdown and OPML files open as new maps, and the
original stays as it is.

<img src="site/img/busca.webp" alt="A map with search open in the toolbar, highlighting the topics that contain the word Design">

## Map themes

Six themes in the spirit of XMind, so it feels familiar if you are looking for an XMind alternative: colored cards,
outline, text on the line, bold colors, pills or grays. Each main branch gets a color and the ones below inherit it.
The theme is saved in the map itself.

- Curved or angled branches, with rounded corners
- A color you pick by hand still wins over the theme

<img src="site/img/temas.webp" alt="The Map theme sheet, with six sample maps: Classic, Outline, Underline, Vibrant, Pills and Monochrome">

## Looks at home on your system

Light and dark themes, and a System theme that matches the colors of Windows, macOS, GNOME and KDE. It works in a
narrow window too.

<img src="site/img/janela-estreita.webp" width="300" alt="Nodo in a narrow window, in dark mode, zoomed in on a topic card with its checklist">

## Structures and formats

Switch the whole map’s structure in one click, and move your ideas in and out in open formats.

| | |
|---|---|
| **Seven structures** | Mind map, logic chart, org chart, tree, matrix, tree table and timeline, with compact, normal or wide spacing. The structure is saved in the map and can be undone. |
| **Markdown and OPML import and export** | Open Markdown, OPML and Minder files as new maps. Export to PNG, SVG, a print-ready PDF, Markdown or OPML. |
| **Every topic is a card** | A Markdown note, tags, an emoji, a reminder, a three-color status, photos and attachments. Copy a card into Ideario, or paste one from it. |
| **Several maps in tabs** | Keep many maps open at once and switch with Ctrl+Tab. Closing one with unsaved changes asks before it goes. |

## Download

Free and open source. Installers for Windows, macOS and Linux are on the
[GitHub releases page](https://github.com/wylho/nodo/releases/latest). Once installed, Nodo tells you when a new version
is out and updates itself.

| | |
|---|---|
| **Windows** | The `-setup.exe` (or the `.msi`). |
| **macOS** | The `.dmg`, for Apple Silicon and Intel. The first time, right-click the app and choose **Open**: it isn’t signed by Apple yet. |
| **Linux** | From the apt or dnf repository, which updates along with your system (below). Or the `.AppImage`, which runs on any distribution and updates itself. |

### Linux, from the repository

Ubuntu, Debian and derivatives:

```sh
curl -fsSL https://nodo-1c7.pages.dev/linux/nodo.gpg | sudo tee /usr/share/keyrings/nodo.gpg > /dev/null
echo "deb [signed-by=/usr/share/keyrings/nodo.gpg] https://nodo-1c7.pages.dev/apt stable main" | sudo tee /etc/apt/sources.list.d/nodo.list
sudo apt update && sudo apt install nodo
```

Fedora:

```sh
sudo curl -fsSL -o /etc/yum.repos.d/nodo.repo https://nodo-1c7.pages.dev/linux/nodo.repo
sudo dnf install nodo
```

**Privacy:** no ads, no tracking, no data collection. Your maps stay on your computer. Read the
[Privacy Policy](https://nodo-1c7.pages.dev/privacy/).

---

## Development

Specification (in Portuguese): [`docs/SPEC.md`](docs/SPEC.md). Project rules: [`CLAUDE.md`](CLAUDE.md). Releasing a
version: [`docs/RELEASE.md`](docs/RELEASE.md).

```
crates/
├── nodo-core/   model, layout, .nodo (and .minder reading), Editor (history + file), no UI
├── nodo-mcp/    MCP server over stdio (24 tools)
└── nodo-app/    Tauri window; the Svelte 5 interface lives in ui/
site/              nodo-1c7.pages.dev (Cloudflare Pages, no build step; English at the root, Portuguese in site/pt/)
```

The app and the MCP server both go through the same `nodo_core::Editor`: an agent produces exactly the same result as
a person in the window. One binary, two modes: `nodo` opens the window, `nodo --mcp` speaks the protocol.

Requires Rust 1.80+, Node 22 and pnpm. On Linux, the WebKitGTK libraries
(`libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev patchelf`).

```bash
pnpm --dir crates/nodo-app/ui install

# Interface in the browser, on a simulated backend, with instant reload: the way to go for any visual change
pnpm --dir crates/nodo-app/ui dev          # open http://localhost:1420/?preview

# Full check
cargo fmt --all --check && cargo clippy --all-targets -- -D warnings && cargo test --workspace
pnpm --dir crates/nodo-app/ui run check && pnpm --dir crates/nodo-app/ui run test

# The real app (cargo build alone does not embed the interface and opens an empty window)
crates/nodo-app/ui/node_modules/.bin/tauri build
```

### MCP outside the app

```json
{ "mcpServers": { "nodo": { "command": "/path/to/nodo", "args": ["--mcp"] } } }
```

Tools: `new_map`, `open_map`, `save_map`, `get_map`, `get_node`, `add_node`, `remove_node`, `set_text`,
`set_note`, `set_color`, `set_side`, `set_map_theme`, `toggle_fold`, `relayout`, `search_nodes`, `undo`, `redo`,
`map_stats`.

## License

GPL-3.0-or-later, inherited from Minder. See [LICENSE](LICENSE). Minder is by Trevor Williams and contributors; Nodo is
a derivative work and carries the same license.

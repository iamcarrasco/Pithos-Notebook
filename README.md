<p align="center">
  <img src="data/icons/hicolor/512x512/apps/com.pithos.notebook.png" width="120" alt="Pithos Notebook icon" />
</p>

<h1 align="center">Pithos Notebook</h1>

<p align="center">
  A private, encrypted markdown notebook for Linux.
  <br />
  Your notes never leave your machine.
</p>

---

## Why Pithos Notebook?

Most note-taking apps sync your data to someone else's server. Pithos Notebook doesn't. Every note is encrypted with AES-256-GCM on your device. There are no accounts, no cloud, no telemetry — just your notes, your machine, your data.

Built from the ground up with GTK 4 and Libadwaita for a fast, lightweight experience that feels at home on GNOME.

## Screenshots

| Light Mode | Dark Mode |
|:---:|:---:|
| ![Light Mode](screenshots/light-mode.png) | ![Dark Mode](screenshots/dark-mode.png) |

| Zen Mode | Mermaid Diagrams |
|:---:|:---:|
| ![Zen Mode](screenshots/zen-mode.png) | ![Mermaid](screenshots/mermaid.png) |

---

## Features

### Encrypted Vault
Your entire notebook is encrypted with a passphrase you choose. AES-256-GCM encryption with PBKDF2 key derivation (600,000 iterations). The passphrase is never stored anywhere — lose it and your data is gone. That's the point.

### Source Editor + Live Preview
Edit raw markdown in a syntax-highlighted source editor (GtkSourceView) with a side-by-side live HTML preview (WebKitGTK). Toolbar buttons and keyboard shortcuts insert markdown syntax automatically.

### Mermaid Diagrams
Write Mermaid diagram blocks in your notes and see them rendered live in the preview pane — flowcharts, sequence diagrams, class diagrams, and more.

### Wiki Links & Backlinks
Link notes together with `[[Note Name]]` syntax. A backlinks panel shows every note that references the current one.

### Organization
- **Folders** — Nest notes in folders, create and rename from the sidebar
- **Tabs** — Open multiple notes, drag to reorder, close with `Ctrl+W`
- **Tags** — Filter by AND/OR logic with a collapsible sidebar panel
- **Search** — Search note titles with `Ctrl+Shift+F`
- **Daily notes** — One-click creation with `Ctrl+Shift+T`
- **Templates** — Create notes from built-in or custom templates
- **Sort** — By name, date created, or date modified (ascending/descending)
- **Version history** — Save named snapshots and restore earlier content

### Writing
- **Zen mode** — Distraction-free writing with `Ctrl+Shift+J`
- **Code blocks** — Fenced blocks with language selector for 14 languages
- **Encrypted image storage** — Drag images into the editor; each image is encrypted and stored as a separate vault asset
- **Markdown & HTML export** — Export individual notes from the menu
- **Import** — Import `.md` files with `Ctrl+O`
- **Command palette** — Quick access to all actions with `Ctrl+Shift+P`

### Desktop Integration
- **Dark mode** — Toggle with button or `Ctrl+Shift+D`, matched across editor and preview
- **Fullscreen** — `F11` for immersive editing
- **Adaptive layout** — Sidebar collapses to overlay on narrow windows
- **Auto-save** — Saves every 30 seconds and on close
- **Libadwaita** — Native GNOME look and feel

---

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Ctrl+N` | New note |
| `Ctrl+S` | Save |
| `Ctrl+W` | Close tab |
| `Ctrl+Shift+F` | Search notes |
| `Ctrl+Shift+T` | Daily note |
| `Ctrl+Shift+D` | Toggle dark/light mode |
| `Ctrl+Shift+J` | Zen mode |
| `Ctrl+Shift+P` | Command palette |
| `Ctrl+\` | Toggle sidebar |
| `F2` | Rename note |
| `F11` | Fullscreen |

### Formatting

| Shortcut | Action |
|----------|--------|
| `Ctrl+B` | Bold |
| `Ctrl+I` | Italic |
| `Ctrl+U` | Underline |
| `Ctrl+D` | Strikethrough |
| `Ctrl+E` | Inline code |
| `Ctrl+K` | Insert link |
| `Ctrl+1` – `Ctrl+6` | Heading 1–6 |
| `Ctrl+Shift+Q` | Block quote |
| `Ctrl+Shift+L` | Bullet list |
| `Ctrl+Space` | Toggle checkbox |
| `Ctrl+Z` | Undo |
| `Ctrl+Shift+Z` | Redo |

---

## Security

| | |
|---|---|
| **Encryption** | AES-256-GCM with PBKDF2-SHA256 (600k iterations) |
| **Storage** | Passphrase never stored; vault unlocked once per session |
| **Assets** | Images encrypted individually alongside the vault |
| **Preview** | WebView runs with restrictive CSP, ephemeral session, navigation blocked |
| **Network** | Zero outbound connections |
| **Telemetry** | None. No analytics, no tracking, no cloud sync |

---

## Build from Source

### Prerequisites

- [Rust](https://rustup.rs/) (latest stable)
- GTK 4, Libadwaita, GtkSourceView 5, and WebKitGTK 6.0 development libraries

#### Ubuntu / Debian

```bash
sudo apt install -y \
  build-essential pkg-config \
  libgtk-4-dev libadwaita-1-dev libgtksourceview-5-dev libwebkitgtk-6.0-dev
```

#### Fedora

```bash
sudo dnf install -y \
  gcc pkg-config \
  gtk4-devel libadwaita-devel gtksourceview5-devel webkit2gtk6.0-devel
```

#### Arch

```bash
sudo pacman -S base-devel gtk4 libadwaita gtksourceview5 webkit2gtk-6.0
```

### Commands

```bash
cargo run              # Development mode
cargo test             # Run tests
cargo build --release  # Production build (binary at target/release/pithos-notebook)
cargo deb              # Build .deb package (requires cargo-deb)
snapcraft              # Build .snap package (requires snapcraft)
```

---

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Toolkit | GTK 4, Libadwaita |
| Source Editor | GtkSourceView 5 |
| Preview | WebKitGTK 6.0 |
| Language | Rust (2021 edition) |
| Encryption | aes-gcm, pbkdf2, sha2 |
| Markdown | pulldown-cmark |
| Diagrams | Mermaid.js (embedded) |
| Serialization | serde, serde_json |

---

## Project Structure

```
├── Cargo.toml
├── data/
│   ├── icons/              # App icon
│   ├── mermaid.min.js      # Embedded mermaid.js for diagram rendering
│   ├── com.pithos.notebook.desktop
│   └── com.pithos.notebook.metainfo.xml
├── snap/
│   └── snapcraft.yaml      # Snap package definition
├── src/
│   ├── main.rs             # Entry point, GtkApplication setup
│   ├── crypto.rs           # AES-256-GCM encryption, PBKDF2 key derivation
│   ├── vault.rs            # Vault file format, JSON serialization, config
│   ├── state.rs            # DocState, NoteItem, FolderItem, TrashItem
│   ├── signals.rs          # Actions, event wiring, all application logic
│   ├── style.css           # Application stylesheet
│   └── ui/
│       ├── types.rs        # EditorCtx, widget struct definitions
│       ├── window.rs       # Window builder, content pane assembly
│       ├── sidebar.rs      # Sidebar pane with search, notes list, folders
│       ├── toolbar.rs      # Formatting toolbar
│       └── dialogs.rs      # Help, about, shortcuts, vault dialogs
└── LICENSE
```

## Vault Format

The vault is a single encrypted file (`vault.mdnb`) stored in a user-chosen folder. On disk it contains a base64-encoded blob: `salt (16 bytes) || IV (12 bytes) || AES-256-GCM ciphertext`. The plaintext is a JSON document holding the full note tree, folders, trash, open tabs, theme, sort order, templates, and version history. Image assets are stored as separate encrypted files in an `assets/` subdirectory alongside the vault.

---

## Changelog

### v0.1.0
- Initial release of Pithos Notebook
- Source editor (GtkSourceView) with side-by-side live HTML preview (WebKitGTK)
- Mermaid diagrams rendered via embedded mermaid.js
- AES-256-GCM encrypted vault with PBKDF2 key derivation (600k iterations)
- Encrypted image storage — drag-and-drop images encrypted as separate vault assets
- Sidebar with nested folders, tags, search, and sort
- Tabbed editing with drag-to-reorder
- Code blocks with syntax highlighting for 14 languages
- Wiki links and backlinks
- Version history with named snapshots and restore
- Daily notes, templates, and command palette
- Zen mode, dark/light theme toggle, fullscreen
- Import Markdown files, export as Markdown or HTML
- WebView security — restrictive CSP, ephemeral sessions, navigation blocking
- Built with GTK 4, Libadwaita, GtkSourceView 5, and WebKitGTK 6.0

## License

[GPL-3.0](LICENSE)

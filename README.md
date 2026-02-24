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
Edit raw markdown in a syntax-highlighted source editor (GtkSourceView) with a side-by-side live HTML preview (WebKitGTK). Toolbar buttons and keyboard shortcuts insert markdown syntax automatically. Synchronized scrolling keeps the preview aligned with your editing position.

### Mermaid Diagrams
Write Mermaid diagram blocks in your notes and see them rendered live in the preview pane — flowcharts, sequence diagrams, class diagrams, and more.

### Wiki Links & Backlinks
Link notes together with `[[Note Name]]` syntax. An autocomplete popover suggests note names as you type. A backlinks panel shows every note that references the current one.

### Organization
- **Folders** — Nest notes in folders, create and rename from the sidebar
- **Tabs** — Open multiple notes, drag to reorder, close with `Ctrl+W`
- **Tags** — Filter by AND/OR logic with a collapsible sidebar panel
- **Search** — Search note titles and content with `Ctrl+Shift+F`; supports `tag:`, `in:folder`, and quoted phrases
- **Daily notes** — One-click creation with `Ctrl+Shift+T`
- **Templates** — Create notes from built-in or custom templates
- **Sort** — By name, date created, or date modified (ascending/descending)
- **Version history** — Save named snapshots and restore earlier content
- **Trash** — Deleted notes go to trash with auto-purge after 30 days

### Writing
- **Zen mode** — Distraction-free writing with `Ctrl+Shift+J`
- **Focus mode** — Dims all text except the current paragraph
- **Hemingway mode** — Disables backspace and delete for forward-only writing
- **Typewriter scrolling** — Keeps the cursor line vertically centered
- **Smart list continuation** — Press Enter in a list to auto-continue with the matching prefix
- **Find and replace** — In-editor search with regex toggle and match count
- **Table editing** — Add rows, add columns, and auto-align from toolbar or command palette
- **Code blocks** — Fenced blocks with language selector for 14 languages
- **Encrypted image storage** — Drag or paste images into the editor; each image is encrypted and stored as a separate vault asset
- **Copy as HTML** — Copy the current note's rendered HTML to the clipboard
- **Markdown & HTML export** — Export individual notes from the menu
- **Import** — Import `.md` files with `Ctrl+O`
- **Command palette** — Quick access to all actions with `Ctrl+Shift+P`
- **Inline link tooltips** — Hover over markdown links and images to see their URL

### Desktop Integration
- **Dark mode** — Toggle with button or `Ctrl+Shift+D`, matched across editor and preview
- **Preview themes** — Cycle between Default, Sepia, and Solarized preview styles
- **Fullscreen** — `F11` for immersive editing with auto-hiding header bar
- **Adaptive layout** — Sidebar collapses to overlay on narrow windows
- **Auto-save** — Saves every 30 seconds and on close
- **Vault change monitoring** — Notified when vault.json is modified externally
- **Change passphrase** — Re-encrypt your entire vault with a new passphrase
- **Vault backup** — Automatic backup before each save
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
| `Ctrl+F` | Find in editor |
| `Ctrl+H` | Find and replace |
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
| **Encryption** | AES-256-GCM with PBKDF2-SHA256 (600k iterations); fresh salt per session |
| **Storage** | Passphrase never stored; vault unlocked once per session |
| **Assets** | Images encrypted individually alongside the vault |
| **Preview** | WebView runs with nonce-based CSP, ephemeral session, navigation blocked, script tags stripped |
| **Key material** | `ZeroizeOnDrop` on cached keys; plaintext vault JSON zeroized after encryption |
| **File I/O** | Atomic writes (write-to-tmp-then-rename); path traversal prevention on asset IDs |
| **Passphrase change** | Transactional all-or-nothing re-encryption of vault and all assets |
| **Network** | Zero outbound connections |
| **Telemetry** | None. No analytics, no tracking, no cloud sync |

> **Snap sandbox note:** The snap package disables WebKit's internal bubblewrap sandbox (`WEBKIT_DISABLE_SANDBOX_THIS_IS_DANGEROUS`) because snap's strict confinement seccomp filters block the nested bwrap calls WebKit requires. Snap's own strict confinement still sandboxes the entire application. When running from source outside the snap, WebKit retains its full internal sandbox.

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
│   ├── signals.rs          # Theme management, CSS, utility functions
│   ├── actions.rs          # Menu actions, toolbar signals, formatting
│   ├── editor.rs           # Editor signals, find/replace, table editing, snippets
│   ├── notes.rs            # Note CRUD, tabs, trash, folders, templates
│   ├── sidebar_ops.rs      # Sidebar search, drag-and-drop, context menus
│   ├── persistence.rs      # Vault save/load, auto-save, file monitoring
│   ├── preview.rs          # Markdown-to-HTML, preview rendering, CSP
│   ├── app_dialogs.rs      # Vault dialogs, passphrase change, command palette
│   ├── style.css           # Application stylesheet
│   └── ui/
│       ├── mod.rs          # Module re-exports
│       ├── types.rs        # EditorCtx, widget struct definitions
│       ├── window.rs       # Window builder, content pane assembly
│       ├── sidebar.rs      # Sidebar pane with search, notes list, folders
│       ├── toolbar.rs      # Formatting toolbar
│       └── dialogs.rs      # Help, about, shortcuts dialogs
└── LICENSE
```

## Vault Format

The vault is a single encrypted file (`vault.json`) stored in a user-chosen folder. On disk it contains a base64-encoded blob: `salt (16 bytes) || IV (12 bytes) || AES-256-GCM ciphertext`. The plaintext is a JSON document holding the full note tree, folders, trash, open tabs, theme, sort order, templates, and version history. Image assets are stored as separate encrypted files in an `assets/` subdirectory alongside the vault.

---

## Changelog

### v0.2.0

#### New Features
- **Find and replace** — In-editor search with regex toggle, case sensitivity toggle, and match count display (`Ctrl+F` / `Ctrl+H`)
- **Copy as HTML** — Copy the current note's rendered HTML to the clipboard
- **Hemingway mode** — Disables backspace and delete for forward-only writing
- **Smart list continuation** — Press Enter inside a list item to auto-continue with the appropriate prefix (bullets, ordered, task lists, blockquotes); pressing Enter on an empty item exits the list
- **Focus mode** — Dims all text except the current paragraph for distraction-free editing
- **Typewriter scrolling** — Keeps the cursor line vertically centered in the editor viewport
- **Preview themes** — Cycle between Default, Sepia, and Solarized preview color schemes
- **Auto-hiding header in fullscreen** — Header bar and toolbar hide in fullscreen and reappear when the mouse moves to the top edge
- **Synchronized preview scrolling** — Editor scroll position is mirrored in the preview WebView
- **Inline link/image tooltips** — Hover over markdown links and images in the editor to see the target URL
- **Vault file change monitoring** — Toast notification when vault.json is modified externally
- **Wiki-link autocomplete** — Note name suggestions appear as you type `[[`
- **Search operators** — `tag:name`, `in:folder`, and `"quoted phrases"` in the sidebar search
- **Change passphrase** — Re-encrypt vault and all assets with a new passphrase (transactional, all-or-nothing)
- **Table editing** — Add rows, add columns, and auto-align markdown tables from the toolbar or command palette
- **Trash auto-purge** — Notes in trash older than 30 days are automatically purged on startup
- **Vault backup** — Automatic `vault.json.bak` created before each save
- **Passphrase strength indicator** — Visual feedback during vault creation and passphrase change

#### Security Improvements
- **Nonce-based CSP** — Preview WebView uses per-render random nonces for mermaid script authorization instead of `'unsafe-inline'`; `script-src 'none'` when no mermaid content is present
- **Script tag stripping** — `<script>` tags are stripped from markdown-generated HTML as defense-in-depth before CSP enforcement
- **Memory zeroization** — `ZeroizeOnDrop` on `CachedKey`; plaintext vault JSON explicitly zeroized after encryption; passphrase strings zeroized after use in change-passphrase flow
- **Atomic file writes** — Vault and asset files are written to a `.tmp` file first, then atomically renamed to prevent corruption on crash
- **Path traversal prevention** — Asset IDs are validated against a strict allowlist (alphanumeric, hyphens, underscores, dots; no path separators, no leading dots, max 128 chars) at every filesystem access point
- **Randomized temp filenames** — Clipboard image paste uses randomized temp file names to prevent symlink/race attacks
- **WebView hardening** — Ephemeral network session (no persistent cookies/cache), developer extras disabled, file URL access disabled, all navigation actions blocked
- **Transactional passphrase change** — All assets are re-encrypted to memory before any are committed to disk; failure at any point leaves the vault unchanged

#### Architecture
- Refactored monolithic `signals.rs` into focused modules: `actions.rs`, `editor.rs`, `notes.rs`, `sidebar_ops.rs`, `persistence.rs`, `preview.rs`, `app_dialogs.rs`
- Async vault save with generation IDs to prevent stale writes from overwriting newer data
- Background-threaded PBKDF2 key derivation to keep the UI responsive during unlock and vault creation

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

# Changelog

## v0.3.0

### Editor Improvements
- **Toggle formatting** — Bold, italic, underline, strikethrough, and inline code now toggle on/off instead of double-wrapping; selection is preserved across multiple clicks
- **Smart bold/italic** — Correctly handles nested `*`/`**` markers (e.g. toggling bold off `***text***` leaves `*text*`)
- **Selection preserved** — Toolbar buttons no longer steal focus from the editor, so selected text stays highlighted
- **Scroll jump fixed** — Applying formatting to selected text no longer jumps to the top of the document
- **List continuation fixed** — Pressing Enter on bullet, numbered, and task list items now correctly continues the list (key handler moved to capture phase)

### Security
- **Passphrase fully zeroized** — Both key material and passphrase string are now zeroized on drop (previously only key was zeroized)
- **Consistent crypto errors** — All decryption errors now include the underlying error message for debugging
- **Asset encryption tests** — Added roundtrip, passthrough, and wrong-key test coverage for asset encrypt/decrypt

### Bug Fixes
- **Template deletion** — Uses name-based lookup instead of stale loop index (prevents deleting wrong template)
- **Shortcuts dialog** — No longer panics if UI builder fails; gracefully returns instead
- **Zen mode split restore** — Correctly restores split position when it was 0 (used +1 offset encoding)
- **Vault item type validation** — Unknown item types in vault data now log a warning instead of being silently treated as notes

### Improvements
- **Vault name in header** — The vault folder name is now displayed in the content header bar
- **Script stripping optimized** — `strip_script_tags` lowercases HTML once instead of per-loop iteration
- **O(1) note ID lookups** — `vault_to_doc_state` uses HashSet for validating active note and open tabs
- **Case-insensitive search optimized** — Search no longer allocates lowercased string copies of full note content
- **Passphrase strength helper** — Extracted duplicate validation logic into shared `wire_passphrase_strength()` function
- **Named constants** — Version limit (`MAX_VERSIONS`), PBKDF2 iterations, sidebar breakpoint, and trash purge days are now named constants
- **Deduplication safety** — `deduplicate_note_name` capped at 10k iterations with timestamp fallback instead of `unreachable!()`

## v0.2.5

### Bug Fixes & Security
- **Fixed double PBKDF2** — Cached key path no longer re-derives the key unnecessarily
- **Save race condition** — Async/sync save ordering secured with generation counter bump
- **Passphrase change safety** — Refused while async save is in-flight; writes vault before assets for safer failure ordering
- **Config write failures logged** — `save_config_or_log()` replaces silent `let _ =` ignoring of config write errors
- **PDF export path fix** — `--pdf-engine-opt=--root=/` resolves Typst template path correctly
- **Undo/redo fix** — Toolbar undo/redo buttons now invoke GtkSourceView undo rather than no-op
- **Table off-by-one** — Fixed column insertion placing the pipe in the wrong position
- **Checkbox toggle** — Fixed toggle logic for task lists in editor
- **Passphrase zeroization** — Passphrase strings securely zeroized after use in all dialogs

### Improvements
- **Export formats expanded** — Now supports Markdown, HTML, PDF, Word (.docx), LaTeX (.tex), and EPUB
- **Removed stale code** — Deleted pre-workspace `src/` directory (19 orphaned files)
- **Removed unused dependency** — `pulldown-cmark` dropped from `pithos-gtk`
- **Zen mode split restore** — Split pane position saved before zen mode and restored on exit
- **Error dialog styling** — Delete confirmation uses destructive action styling

## v0.2.0

### New Features
- **PDF export** — Export any note as a professionally styled PDF document
- **Multi-vault support** — Switch between vaults without restarting the app; recent vaults remembered across sessions
- **Settings dialog** — Enable or disable built-in and custom templates from a dedicated settings panel
- **Find and replace** — In-editor search with regex toggle, case sensitivity toggle, and match count display (`Ctrl+F` / `Ctrl+H`)
- **Smart list continuation** — Press Enter inside a list item to auto-continue with the appropriate prefix (bullets, ordered, task lists, blockquotes); pressing Enter on an empty item exits the list
- **Auto-hiding header in fullscreen** — Header bar and toolbar hide in fullscreen and reappear when the mouse moves to the top edge
- **Synchronized preview scrolling** — Editor scroll position is mirrored in the preview WebView
- **Vault file change monitoring** — Toast notification when vault.json is modified externally
- **Change passphrase** — Re-encrypt vault and all assets with a new passphrase (transactional, all-or-nothing)
- **Table editing** — Add rows, add columns, and auto-align markdown tables from the toolbar or command palette
- **Trash auto-purge** — Notes in trash older than 30 days are automatically purged on startup
- **Vault backup** — Automatic `vault.json.bak` created before each save
- **Passphrase strength indicator** — Visual feedback during vault creation and passphrase change
- **New templates** — Threat Model, Architecture Decision Record, IAM Blueprint, Runbook, and Security Review
- **Additional code languages** — TypeScript, HCL, PowerShell, KQL, and Dockerfile added to code block syntax highlighting

### Security Improvements
- **Nonce-based CSP** — Preview WebView uses per-render random nonces for mermaid script authorization instead of `'unsafe-inline'`; `script-src 'none'` when no mermaid content is present
- **Script tag stripping** — `<script>` tags are stripped from markdown-generated HTML as defense-in-depth before CSP enforcement
- **Memory zeroization** — `ZeroizeOnDrop` on `CachedKey`; plaintext vault JSON explicitly zeroized after encryption; passphrase strings zeroized after use in change-passphrase flow
- **Atomic file writes** — Vault and asset files are written to a `.tmp` file first, then atomically renamed to prevent corruption on crash
- **Path traversal prevention** — Asset IDs are validated against a strict allowlist (alphanumeric, hyphens, underscores, dots; no path separators, no leading dots, max 128 chars) at every filesystem access point
- **Randomized temp filenames** — Clipboard image paste uses randomized temp file names to prevent symlink/race attacks
- **WebView hardening** — Ephemeral network session (no persistent cookies/cache), developer extras disabled, file URL access disabled, all navigation actions blocked
- **Transactional passphrase change** — All assets are re-encrypted to memory before any are committed to disk; failure at any point leaves the vault unchanged

### Architecture
- Split into a Cargo workspace: `pithos-core` (GUI-independent library) and `pithos-gtk` (GTK 4 frontend)
- Refactored monolithic `signals.rs` into focused modules: `actions.rs`, `editor.rs`, `notes.rs`, `sidebar_ops.rs`, `persistence.rs`, `preview.rs`, `app_dialogs.rs`
- Async vault save with generation IDs to prevent stale writes from overwriting newer data
- Background-threaded PBKDF2 key derivation to keep the UI responsive during unlock and vault creation

## v0.1.0
- Initial release of Pithos Notebook
- Source editor (GtkSourceView) with side-by-side live HTML preview (WebKitGTK)
- Mermaid diagrams rendered via embedded mermaid.js
- AES-256-GCM encrypted vault with PBKDF2 key derivation (600k iterations)
- Encrypted image storage — drag-and-drop images encrypted as separate vault assets
- Sidebar with nested folders, tags, search, and sort
- Tabbed editing with drag-to-reorder
- Code blocks with syntax highlighting for 14 languages
- Wiki links
- Version history with named snapshots and restore
- Daily notes, templates, and command palette
- Zen mode, dark/light theme toggle, fullscreen
- Import Markdown files, export as Markdown or HTML
- WebView security — restrictive CSP, ephemeral sessions, navigation blocking
- Built with GTK 4, Libadwaita, GtkSourceView 5, and WebKitGTK 6.0

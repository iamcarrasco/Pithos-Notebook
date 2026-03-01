use crate::*;
use pithos_core::crypto;
use pithos_core::vault;
use std::fs;

/// Generate a random base64 nonce for Content-Security-Policy script-src.
fn generate_csp_nonce() -> String {
    let mut bytes = [0u8; 16];
    rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut bytes);
    base64::Engine::encode(&base64::engine::general_purpose::STANDARD, bytes)
}

/// Strip `<script>...</script>` tags (case-insensitive) from HTML to prevent
/// user-authored scripts from executing in the preview.
fn strip_script_tags(html: &mut String) {
    loop {
        let lower = html.to_lowercase();
        let Some(start) = lower.find("<script") else {
            break;
        };
        // Find matching </script>
        if let Some(rel_end) = lower[start..].find("</script>") {
            let end = start + rel_end + "</script>".len();
            html.replace_range(start..end, "");
        } else {
            // Unclosed <script> tag — remove from <script to end
            html.truncate(start);
            break;
        }
    }
}

pub fn render_preview(ctx: &EditorCtx) {
    use webkit6::prelude::WebViewExt;
    let markdown = source_buffer_text(&ctx.source_buffer);
    let dark = is_dark_active();
    // Set the WebView background to match the theme immediately,
    // preventing a flash of the wrong color while HTML loads.
    let bg = if dark {
        gtk::gdk::RGBA::new(0.118, 0.118, 0.118, 1.0)
    } else {
        gtk::gdk::RGBA::new(0.98, 0.98, 0.98, 1.0)
    };
    ctx.preview_webview.set_background_color(&bg);
    let mut html = build_preview_html(&markdown, dark);

    // Replace vault:// asset URLs with inline data: URLs so images render in preview
    resolve_vault_assets(&mut html, ctx);

    ctx.preview_webview.load_html(&html, None);
}

/// Replaces `src="vault://asset_id"` in HTML with `src="data:mime;base64,..."`.
fn resolve_vault_assets(html: &mut String, ctx: &EditorCtx) {
    let prefix = "src=\"vault://";
    while let Some(start) = html.find(prefix) {
        let attr_start = start + "src=\"".len(); // start of vault://...
        let Some(quote_end) = html[attr_start..].find('"') else {
            break;
        };
        let vault_url = html[attr_start..attr_start + quote_end].to_string();
        let Some(asset_id) = vault_url.strip_prefix("vault://") else {
            break;
        };

        let data_url = resolve_single_asset(asset_id, ctx);
        let replacement = format!("src=\"{}\"", data_url);
        html.replace_range(start..attr_start + quote_end + 1, &replacement);
    }
}

fn resolve_single_asset(asset_id: &str, ctx: &EditorCtx) -> String {
    // Reject asset IDs that could escape the assets directory (path traversal).
    if !vault::is_valid_asset_id(asset_id) {
        return String::new();
    }

    let vault_folder = ctx.vault_folder.borrow().clone();
    let mime_type = ctx
        .state
        .borrow()
        .assets
        .get(asset_id)
        .map(|m| m.mime_type.clone())
        .unwrap_or_else(|| "image/png".to_string());

    let asset_path = vault::assets_dir(&vault_folder).join(asset_id);
    let Ok(raw_data) = fs::read(&asset_path) else {
        return String::new();
    };

    let cached_key_ref = ctx.cached_key.borrow();
    let Some(cached_key) = cached_key_ref.as_ref() else {
        return String::new();
    };

    match crypto::decrypt_asset(&raw_data, cached_key) {
        Ok(decrypted) => {
            let b64 =
                base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &decrypted);
            format!("data:{mime_type};base64,{b64}")
        }
        Err(_) => String::new(),
    }
}

const MERMAID_JS: &str = include_str!("../../data/mermaid.min.js");

pub fn build_preview_html(markdown: &str, dark: bool) -> String {
    let mut body = pithos_core::export::markdown_to_html(markdown);

    // Strip any <script> tags from the markdown-generated HTML to prevent
    // user-authored scripts from executing in the preview WebView.
    strip_script_tags(&mut body);

    // Convert mermaid code blocks: <pre><code class="language-mermaid">...</code></pre>
    // → <pre class="mermaid">...</pre>
    let has_mermaid = body.contains("language-mermaid");
    // Replace only mermaid code blocks, leaving other <pre><code> blocks intact
    if has_mermaid {
        let open_tag = r#"<pre><code class="language-mermaid">"#;
        let close_tag = "</code></pre>";
        while let Some(start) = body.find(open_tag) {
            if let Some(rel_end) = body[start + open_tag.len()..].find(close_tag) {
                let content_start = start + open_tag.len();
                let content_end = content_start + rel_end;
                let content = body[content_start..content_end].to_string();
                let replacement = format!(r#"<pre class="mermaid">{content}</pre>"#);
                body.replace_range(start..content_end + close_tag.len(), &replacement);
            } else {
                break;
            }
        }
    }

    let (bg, fg, code_bg, border, link_color, heading_color) = if dark {
        (
            "#1e1e1e", "#d4d4d4", "#2a2a2a", "#3c3c3c", "#78aeed", "#e0e0e0",
        )
    } else {
        (
            "#fafafa", "#2e2e2e", "#f0f0f0", "#d5d5d5", "#1c71d8", "#1e1e1e",
        )
    };

    // Use a nonce-based CSP for mermaid scripts instead of 'unsafe-inline'.
    // This ensures only our trusted mermaid scripts can execute.
    let nonce = generate_csp_nonce();
    let script_src = if has_mermaid {
        format!("'nonce-{nonce}'")
    } else {
        "'none'".to_string()
    };

    let mermaid_theme = if dark { "dark" } else { "default" };
    let mermaid_script = if has_mermaid {
        format!(
            r#"<script nonce="{nonce}">{MERMAID_JS}</script>
<script nonce="{nonce}">mermaid.initialize({{ startOnLoad: true, theme: '{mermaid_theme}' }});</script>"#
        )
    } else {
        String::new()
    };

    format!(
        r#"<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src 'unsafe-inline'; img-src data:; script-src {script_src};">
<style>
body {{
    font-family: -apple-system, 'Cantarell', 'Segoe UI', sans-serif;
    font-size: 15px;
    line-height: 1.7;
    color: {fg};
    background: {bg};
    padding: 16px 24px;
    max-width: 800px;
    margin: 0 auto;
}}
h1, h2, h3, h4, h5, h6 {{ color: {heading_color}; margin-top: 1.4em; margin-bottom: 0.5em; }}
h1 {{ font-size: 2em; border-bottom: 1px solid {border}; padding-bottom: 0.3em; }}
h2 {{ font-size: 1.5em; border-bottom: 1px solid {border}; padding-bottom: 0.3em; }}
h3 {{ font-size: 1.25em; }}
a {{ color: {link_color}; text-decoration: none; }}
a:hover {{ text-decoration: underline; }}
code {{ background: {code_bg}; padding: 2px 6px; border-radius: 4px; font-size: 0.9em; }}
pre {{ background: {code_bg}; padding: 12px 16px; border-radius: 8px; overflow-x: auto; border: 1px solid {border}; }}
pre code {{ background: none; padding: 0; }}
pre.mermaid {{ background: transparent; border: none; padding: 8px 0; text-align: center; }}
blockquote {{ border-left: 3px solid {link_color}; margin: 1em 0; padding: 0.5em 1em; color: {fg}; opacity: 0.85; }}
table {{ border-collapse: collapse; width: 100%; margin: 1em 0; }}
th, td {{ border: 1px solid {border}; padding: 8px 12px; text-align: left; }}
th {{ background: {code_bg}; font-weight: 600; }}
hr {{ border: none; border-top: 1px solid {border}; margin: 2em 0; }}
img {{ max-width: 100%; height: auto; border-radius: 4px; }}
ul, ol {{ padding-left: 1.5em; }}
li {{ margin: 0.3em 0; }}
input[type="checkbox"] {{ margin-right: 0.5em; }}
</style>
</head>
<body>
{body}
{mermaid_script}
</body>
</html>"#
    )
}

/// Scroll the preview WebView to match the editor scroll fraction.
pub fn sync_preview_scroll(ctx: &EditorCtx, fraction: f64) {
    use webkit6::prelude::WebViewExt;
    let js = format!(
        "window.scrollTo(0, (document.body.scrollHeight - window.innerHeight) * {fraction});"
    );
    ctx.preview_webview.evaluate_javascript(
        &js,
        None,
        None,
        None::<&gtk::gio::Cancellable>,
        |_| {},
    );
}

//! Text exports: the map as an indented Markdown list and as OPML, the outline format most note and outliner apps
//! open. They live in the core so the window and the MCP server export exactly the same thing.

use crate::model::{MindMap, Node};

/// Export formats that are plain text.
pub const TEXT_FORMATS: [&str; 3] = ["markdown", "opml", "html"];

/// Where the window puts the drawing of the map in [`to_html`]; the MCP export keeps it as a comment.
pub const HTML_FIGURE: &str = "<!-- nodo:figure -->";

/// The map as Markdown: the root as a title, every other node as a nested list item, and each note as an indented
/// paragraph under its node. Folded branches are exported too: folding is a view, not content.
pub fn to_markdown(map: &MindMap) -> String {
    let mut out = format!("# {}\n", one_line(&map.root.text));
    if !map.root.note.trim().is_empty() {
        out.push('\n');
        out.push_str(map.root.note.trim());
        out.push('\n');
    }
    if !map.root.children.is_empty() {
        out.push('\n');
    }
    fn walk(node: &Node, depth: usize, out: &mut String) {
        let pad = "  ".repeat(depth);
        out.push_str(&format!("{pad}- {}\n", one_line(&node.text)));
        let note = node.note.trim();
        if !note.is_empty() {
            for line in note.lines() {
                if line.trim().is_empty() {
                    out.push('\n');
                } else {
                    out.push_str(&format!("{pad}  {line}\n"));
                }
            }
        }
        for child in &node.children {
            walk(child, depth + 1, out);
        }
    }
    for child in &map.root.children {
        walk(child, 0, &mut out);
    }
    out
}

/// The map as OPML 2.0: one `<outline>` per node, the note in `_note` (the convention OmniOutliner, Workflowy and
/// others read).
pub fn to_opml(map: &MindMap) -> String {
    let mut out =
        String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<opml version=\"2.0\">\n");
    out.push_str(&format!(
        "  <head>\n    <title>{}</title>\n  </head>\n  <body>\n",
        esc(&one_line(&map.root.text))
    ));
    fn walk(node: &Node, depth: usize, out: &mut String) {
        let pad = "  ".repeat(depth + 2);
        let note = if node.note.trim().is_empty() {
            String::new()
        } else {
            format!(" _note=\"{}\"", esc(node.note.trim()))
        };
        let text = esc(&one_line(&node.text));
        if node.children.is_empty() {
            out.push_str(&format!("{pad}<outline text=\"{text}\"{note}/>\n"));
        } else {
            out.push_str(&format!("{pad}<outline text=\"{text}\"{note}>\n"));
            for child in &node.children {
                walk(child, depth + 1, out);
            }
            out.push_str(&format!("{pad}</outline>\n"));
        }
    }
    walk(&map.root, 0, &mut out);
    out.push_str("  </body>\n</opml>\n");
    out
}

/// The map as one self-contained HTML page: the title, a slot for the drawing ([`HTML_FIGURE`]) and the whole map as
/// an outline whose branches open and close (`<details>`), with each note under its topic. No scripts, no external
/// files: it opens anywhere, offline.
pub fn to_html(map: &MindMap) -> String {
    let title = esc(&one_line(&map.root.text));
    let mut out = format!(
        "<!doctype html>\n<html lang=\"pt-BR\">\n<head>\n<meta charset=\"utf-8\">\n\
<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n<title>{title}</title>\n<style>\n{HTML_CSS}</style>\n</head>\n<body>\n<main>\n<h1>{title}</h1>\n"
    );
    if !map.root.note.trim().is_empty() {
        out.push_str(&format!(
            "<p class=\"note\">{}</p>\n",
            text_html(map.root.note.trim())
        ));
    }
    out.push_str(&format!(
        "<figure>{HTML_FIGURE}</figure>\n<ul class=\"outline\">\n"
    ));
    fn walk(node: &Node, out: &mut String) {
        let text = esc(&one_line(&node.text));
        let note = if node.note.trim().is_empty() {
            String::new()
        } else {
            format!("<p class=\"note\">{}</p>", text_html(node.note.trim()))
        };
        if node.children.is_empty() {
            out.push_str(&format!("<li><span>{text}</span>{note}</li>\n"));
        } else {
            out.push_str(&format!(
                "<li><details open><summary>{text}</summary>{note}<ul>\n"
            ));
            for child in &node.children {
                walk(child, out);
            }
            out.push_str("</ul></details></li>\n");
        }
    }
    for child in &map.root.children {
        walk(child, &mut out);
    }
    out.push_str("</ul>\n<footer>Exportado do Nodo</footer>\n</main>\n</body>\n</html>\n");
    out
}

/// A note as HTML text: escaped, with its line breaks kept.
fn text_html(s: &str) -> String {
    esc(s).replace("&#10;", "<br>")
}

const HTML_CSS: &str = "\
:root { color-scheme: light dark; --bg: #eef1ef; --fg: #16201c; --muted: #5d6a65; --line: #d6ddd9; }
@media (prefers-color-scheme: dark) { :root { --bg: #101513; --fg: #e5ebe8; --muted: #93a19b; --line: #2b3632; } }
body { margin: 0; background: var(--bg); color: var(--fg); font: 16px/1.55 system-ui, -apple-system, 'Segoe UI', Roboto, sans-serif; }
main { max-width: 960px; margin: 0 auto; padding: 40px 20px 64px; }
h1 { font-size: 2em; letter-spacing: -.02em; margin: 0 0 .4em; }
figure { margin: 24px 0 32px; overflow-x: auto; }
figure svg { max-width: 100%; height: auto; display: block; margin: 0 auto; }
.outline, .outline ul { list-style: none; padding-left: 1.2em; margin: 0; border-left: 1px solid var(--line); }
.outline { padding-left: 0; border: 0; }
li { margin: 4px 0; }
summary { cursor: pointer; font-weight: 600; }
.note { color: var(--muted); margin: 2px 0 6px; font-size: .94em; }
footer { margin-top: 48px; color: var(--muted); font-size: 13px; }
";

/// Export in one of [`TEXT_FORMATS`].
pub fn to_text(map: &MindMap, format: &str) -> crate::Result<String> {
    match format {
        "markdown" => Ok(to_markdown(map)),
        "opml" => Ok(to_opml(map)),
        "html" => Ok(to_html(map)),
        other => Err(crate::Error::UnknownFormat(other.to_string())),
    }
}

/// A node's text on one line: line breaks inside a node become spaces in a list item or an attribute.
fn one_line(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Escape for XML attributes and text, including line breaks inside attributes.
fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\n', "&#10;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{undo::Action, Editor};

    fn sample() -> MindMap {
        let mut ed = Editor::new("Viagem <2027>");
        let a = ed
            .apply(Action::AddChild {
                parent: 0,
                text: "Roteiro".into(),
            })
            .unwrap()
            .unwrap();
        let a1 = ed
            .apply(Action::AddChild {
                parent: a,
                text: "Dia 1\nchegada".into(),
            })
            .unwrap()
            .unwrap();
        ed.apply(Action::SetNote {
            id: a1,
            note: "Hotel perto da praia & \"barato\"".into(),
        })
        .unwrap();
        ed.apply(Action::AddChild {
            parent: 0,
            text: "Orçamento".into(),
        })
        .unwrap();
        ed.apply(Action::ToggleFold { id: a }).unwrap();
        ed.map().clone()
    }

    #[test]
    fn markdown_is_a_nested_list_with_notes_and_folded_branches() {
        let md = to_markdown(&sample());
        assert_eq!(
            md,
            "# Viagem <2027>\n\n- Roteiro\n  - Dia 1 chegada\n    Hotel perto da praia & \"barato\"\n- Orçamento\n"
        );
    }

    #[test]
    fn opml_nests_outlines_escapes_and_keeps_notes() {
        let opml = to_opml(&sample());
        assert!(opml.contains("<title>Viagem &lt;2027&gt;</title>"));
        assert!(opml.contains("<outline text=\"Roteiro\">"));
        assert!(opml.contains(
            "<outline text=\"Dia 1 chegada\" _note=\"Hotel perto da praia &amp; &quot;barato&quot;\"/>"
        ));
        // Bem formado: cada <outline> aberto é fechado.
        assert_eq!(
            opml.matches("<outline").count(),
            opml.matches("/>").count() + opml.matches("</outline>").count()
        );
    }

    #[test]
    fn html_is_one_page_with_an_outline_and_a_slot_for_the_drawing() {
        let html = to_html(&sample());
        assert!(html.starts_with("<!doctype html>"));
        assert!(html.contains("<title>Viagem &lt;2027&gt;</title>"));
        assert!(html.contains(HTML_FIGURE));
        assert!(html.contains("<details open><summary>Roteiro</summary>"));
        assert!(html.contains("<li><span>Dia 1 chegada</span><p class=\"note\">Hotel perto da praia &amp; &quot;barato&quot;</p></li>"));
        assert!(!html.contains("<script"));
        assert_eq!(
            html.matches("<details").count(),
            html.matches("</details>").count()
        );
    }

    #[test]
    fn unknown_text_format_is_refused() {
        assert!(to_text(&sample(), "docx").is_err());
        assert!(to_text(&sample(), "markdown").is_ok());
    }
}

//! Import: a Markdown document becomes a map. Headings and list items are topics, nested by heading level and by
//! list indentation; any other text (paragraphs, quotes, code) becomes the note of the topic above it. The Markdown
//! that [`crate::export::to_markdown`] writes comes back as the same tree.

use crate::model::{MindMap, Node};

/// One topic as it is read, before it becomes a node.
struct Item {
    /// Heading depth (1 for `#`) or, for list items, a level below the heading they sit under.
    depth: usize,
    /// A `#` heading rather than a list item: only a heading can become the root.
    heading: bool,
    text: String,
    note: Vec<String>,
}

/// Turn Markdown into a map. A single top `# Title` is the root; otherwise the root is `fallback_title` and the top
/// headings or items hang from it. Empty documents give a map with just the root.
pub fn from_markdown(markdown: &str, fallback_title: &str) -> MindMap {
    let items = read_items(markdown);

    // Uma única linha "# Título" no topo, sem outro título do mesmo nível, vira a raiz.
    let h1s = items.iter().filter(|i| i.heading && i.depth == 1).count();
    let mut items = items.into_iter().peekable();
    let (title, root_note, base) = match items.peek() {
        Some(first) if first.heading && first.depth == 1 && h1s == 1 => {
            let first = items.next().unwrap_or_else(unreachable_item);
            (first.text, first.note, 1)
        }
        _ => (fallback_title.trim().to_string(), Vec::new(), 0),
    };
    let mut map = MindMap::new(if title.is_empty() { "Mapa" } else { &title });
    map.root.note = join_note(&root_note);

    // Pilha de (profundidade, id) dos ancestrais abertos: cada item fica sob o último mais raso que ele.
    let mut stack: Vec<(usize, u64)> = vec![(base, map.root.id)];
    for item in items {
        while stack.len() > 1 && stack.last().is_some_and(|(d, _)| *d >= item.depth) {
            stack.pop();
        }
        let parent = stack.last().map(|(_, id)| *id).unwrap_or(map.root.id);
        let Some(id) = map.add_child(parent, item.text) else {
            continue;
        };
        if let Some(n) = map.find_mut(id) {
            n.note = join_note(&item.note);
        }
        stack.push((item.depth, id));
    }
    // Ramos da raiz se dividem entre os dois lados, como num mapa novo.
    balance_sides(&mut map.root);
    map
}

/// Turn an OPML outline into a map. A single top `<outline>` is the root (that is what [`crate::export::to_opml`]
/// writes); otherwise the root is the `<title>` of the file, or `fallback_title`, with the outlines under it. The note
/// comes from `_note`, the attribute OmniOutliner, Workflowy and others use.
pub fn from_opml(xml: &str, fallback_title: &str) -> crate::Result<MindMap> {
    use crate::document::{attr, local_name};
    use quick_xml::events::Event;
    use quick_xml::Reader;

    struct Raw {
        text: String,
        note: String,
        children: Vec<Raw>,
    }

    let mut reader = Reader::from_str(xml);
    let mut buf = Vec::new();
    let mut title = String::new();
    let mut in_title = false;
    let mut saw_opml = false;
    // A pilha começa com um nó de mentira que junta as raízes do <body>.
    let mut stack: Vec<Raw> = vec![Raw {
        text: String::new(),
        note: String::new(),
        children: Vec::new(),
    }];
    loop {
        let event = reader
            .read_event_into(&mut buf)
            .map_err(|e| crate::Error::Xml(e.to_string()))?;
        match event {
            Event::Eof => break,
            Event::Start(ref e) | Event::Empty(ref e) => {
                let empty = matches!(event, Event::Empty(_));
                match local_name(e.name().as_ref()).as_str() {
                    "opml" => saw_opml = true,
                    "title" => in_title = !empty,
                    "outline" => {
                        let item = Raw {
                            text: attr(e, "text")
                                .or_else(|| attr(e, "title"))
                                .unwrap_or_default(),
                            note: attr(e, "_note").unwrap_or_default(),
                            children: Vec::new(),
                        };
                        if empty {
                            if let Some(top) = stack.last_mut() {
                                top.children.push(item);
                            }
                        } else {
                            stack.push(item);
                        }
                    }
                    _ => {}
                }
            }
            Event::Text(t) if in_title => {
                title.push_str(&t.unescape().map(|s| s.into_owned()).unwrap_or_default());
            }
            Event::End(ref e) => match local_name(e.name().as_ref()).as_str() {
                "title" => in_title = false,
                "outline" if stack.len() > 1 => {
                    if let Some(done) = stack.pop() {
                        if let Some(top) = stack.last_mut() {
                            top.children.push(done);
                        }
                    }
                }
                _ => {}
            },
            _ => {}
        }
        buf.clear();
    }
    if !saw_opml {
        return Err(crate::Error::Xml(
            "not an OPML file (no <opml> element)".into(),
        ));
    }
    let body = stack
        .into_iter()
        .next()
        .map(|r| r.children)
        .unwrap_or_default();

    let (root, tops) = match <[Raw; 1]>::try_from(body) {
        Ok([only]) => ((only.text, only.note), only.children),
        Err(many) => {
            let name = if title.trim().is_empty() {
                fallback_title
            } else {
                title.trim()
            };
            ((name.to_string(), String::new()), many)
        }
    };
    let mut map = MindMap::new(if root.0.trim().is_empty() {
        "Mapa"
    } else {
        root.0.trim()
    });
    map.root.note = root.1.trim().to_string();
    fn add(map: &mut MindMap, parent: u64, raw: Raw) {
        if let Some(id) = map.add_child(parent, raw.text.trim().to_string()) {
            if let Some(n) = map.find_mut(id) {
                n.note = raw.note.trim().to_string();
            }
            for c in raw.children {
                add(map, id, c);
            }
        }
    }
    let root_id = map.root.id;
    for t in tops {
        add(&mut map, root_id, t);
    }
    balance_sides(&mut map.root);
    Ok(map)
}

/// Never reached: `peek` just said there is a first item. Kept so the code has no `unwrap` on a path that panics.
fn unreachable_item() -> Item {
    Item {
        depth: 1,
        heading: true,
        text: String::new(),
        note: Vec::new(),
    }
}

fn join_note(lines: &[String]) -> String {
    let text = lines.join("\n");
    text.trim_matches('\n').to_string()
}

/// Read headings and list items in order, with every other line attached as note text.
fn read_items(markdown: &str) -> Vec<Item> {
    let mut items: Vec<Item> = Vec::new();
    // Profundidade do último título: as listas abaixo dele começam um nível mais fundo.
    let mut heading = 0usize;
    // Recuos das listas abertas sob o título atual, para saber o nível de cada item.
    let mut indents: Vec<usize> = Vec::new();
    let mut in_code = false;

    for raw in markdown.lines() {
        let line = raw.trim_end();
        let trimmed = line.trim_start();

        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_code = !in_code;
            attach(&mut items, line);
            continue;
        }
        if in_code {
            attach(&mut items, line);
            continue;
        }

        if let Some((level, text)) = heading_of(trimmed) {
            heading = level;
            indents.clear();
            items.push(Item {
                depth: level,
                heading: true,
                text: plain_title(&text),
                note: Vec::new(),
            });
            continue;
        }

        if let Some(text) = list_item(trimmed) {
            let indent = expand_tabs(&line[..line.len() - trimmed.len()]);
            while indents.last().is_some_and(|&i| i > indent) {
                indents.pop();
            }
            if indents.last() != Some(&indent) {
                indents.push(indent);
            }
            items.push(Item {
                depth: heading + indents.len(),
                heading: false,
                text: plain_title(&text),
                note: Vec::new(),
            });
            continue;
        }

        // Linha horizontal (`---`, `***`, `___`) separa seções do documento; não é texto de nenhum tópico.
        if is_rule(trimmed) {
            continue;
        }

        // Texto solto: nota do tópico acima, sem o recuo da lista. Linha em branco separa parágrafos.
        if trimmed.is_empty() {
            if items.last().is_some_and(|i| !i.note.is_empty()) {
                attach(&mut items, "");
            }
        } else {
            attach(&mut items, trimmed);
        }
    }
    items
}

/// O título do tópico é texto puro: negrito, itálico, código, riscado e links do Markdown saem, ficando o texto. A
/// leitura é a do Atrio (a mesma das notas). Se a linha não for um parágrafo comum (`2024. Ano` seria uma lista), fica
/// como veio.
fn plain_title(line: &str) -> String {
    fn collect(v: &serde_json::Value, out: &mut String) {
        match v.get("type").and_then(|t| t.as_str()) {
            Some("text") => out.push_str(v.get("text").and_then(|t| t.as_str()).unwrap_or("")),
            Some("hardBreak") => out.push(' '),
            _ => {}
        }
        for c in v
            .get("content")
            .and_then(|c| c.as_array())
            .into_iter()
            .flatten()
        {
            collect(c, out);
        }
    }
    let doc = atrio::markdown::from_markdown(line);
    let blocks = doc.get("content").and_then(|c| c.as_array());
    let [only] = blocks.map(Vec::as_slice).unwrap_or(&[]) else {
        return line.to_string();
    };
    if !matches!(
        only.get("type").and_then(|t| t.as_str()),
        Some("paragraph" | "heading")
    ) {
        return line.to_string();
    }
    let mut text = String::new();
    collect(only, &mut text);
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if text.is_empty() {
        line.to_string()
    } else {
        text
    }
}

/// `---`, `***` ou `___` (três ou mais, espaços entre eles valem): a linha horizontal do Markdown.
fn is_rule(line: &str) -> bool {
    let marks: Vec<char> = line.chars().filter(|c| !c.is_whitespace()).collect();
    marks.len() >= 3 && ['-', '*', '_'].iter().any(|m| marks.iter().all(|c| c == m))
}

fn attach(items: &mut [Item], line: &str) {
    if let Some(last) = items.last_mut() {
        last.note.push(line.to_string());
    }
}

/// `## Texto` → (2, "Texto"). Até seis `#`, seguidos de espaço.
fn heading_of(line: &str) -> Option<(usize, String)> {
    let hashes = line.chars().take_while(|c| *c == '#').count();
    if hashes == 0 || hashes > 6 {
        return None;
    }
    let rest = &line[hashes..];
    if !rest.is_empty() && !rest.starts_with(' ') {
        return None;
    }
    let text = rest.trim().trim_end_matches('#').trim();
    Some((hashes, text.to_string()))
}

/// `- texto`, `* texto`, `+ texto`, `1. texto` ou `1) texto`; tarefas `[ ]`/`[x]` perdem a caixinha.
fn list_item(line: &str) -> Option<String> {
    let rest = if let Some(r) = line
        .strip_prefix("- ")
        .or_else(|| line.strip_prefix("* "))
        .or_else(|| line.strip_prefix("+ "))
    {
        r
    } else {
        let digits = line.chars().take_while(|c| c.is_ascii_digit()).count();
        if digits == 0 {
            return None;
        }
        let after = &line[digits..];
        after
            .strip_prefix(". ")
            .or_else(|| after.strip_prefix(") "))?
    };
    let rest = rest.trim();
    let rest = ["[ ] ", "[x] ", "[X] "]
        .iter()
        .find_map(|b| rest.strip_prefix(b))
        .unwrap_or(rest);
    Some(rest.trim().to_string())
}

fn expand_tabs(s: &str) -> usize {
    s.chars().map(|c| if c == '\t' { 4 } else { 1 }).sum()
}

/// Alternate the root's branches between right and left, starting on the right.
fn balance_sides(root: &mut Node) {
    use crate::model::NodeSide;
    for (i, child) in root.children.iter_mut().enumerate() {
        let side = if i % 2 == 0 {
            NodeSide::Right
        } else {
            NodeSide::Left
        };
        crate::layout::for_each_mut(child, &mut |n| n.side = side);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texts(n: &Node) -> Vec<&str> {
        n.children.iter().map(|c| c.text.as_str()).collect()
    }

    #[test]
    fn headings_and_lists_nest_and_text_becomes_notes() {
        let md = "# Viagem\n\nPlanejamento de julho.\n\n## Roteiro\n- Dia 1\n  - Praia\n    Levar protetor.\n  - Jantar\n- Dia 2\n\n## Orçamento\n1. Hotel\n2. [x] Passagens\n\nTotal em aberto.\n";
        let map = from_markdown(md, "ignorado");
        assert_eq!(map.root.text, "Viagem");
        assert_eq!(map.root.note, "Planejamento de julho.");
        assert_eq!(texts(&map.root), ["Roteiro", "Orçamento"]);
        let roteiro = &map.root.children[0];
        assert_eq!(texts(roteiro), ["Dia 1", "Dia 2"]);
        assert_eq!(texts(&roteiro.children[0]), ["Praia", "Jantar"]);
        assert_eq!(roteiro.children[0].children[0].note, "Levar protetor.");
        let orc = &map.root.children[1];
        assert_eq!(texts(orc), ["Hotel", "Passagens"]);
        assert_eq!(orc.children[1].note, "Total em aberto.");
    }

    #[test]
    fn titles_lose_the_inline_markdown() {
        let md = "# **Plano** da `feira`\n\n- **Banca:** perto da *entrada*.\n- Ver o [mapa](https://exemplo.org) e ~~o atalho~~\n- 2024. Ano da primeira feira\n- 5 * 3 caixas\n";
        let map = from_markdown(md, "x");
        assert_eq!(map.root.text, "Plano da feira");
        assert_eq!(
            texts(&map.root),
            [
                "Banca: perto da entrada.",
                "Ver o mapa (https://exemplo.org) e o atalho",
                "2024. Ano da primeira feira",
                "5 * 3 caixas"
            ]
        );
    }

    #[test]
    fn a_rule_between_sections_is_not_a_note() {
        let md = "## Vídeos\n- Narração calma.\n- Música da plataforma.\n\n---\n\n## Fotos\n- Luz natural.\n\n***\n";
        let map = from_markdown(md, "x");
        assert_eq!(texts(&map.root), ["Vídeos", "Fotos"]);
        assert_eq!(map.root.children[0].children[1].note, "");
        assert_eq!(map.root.children[1].children[0].note, "");
    }

    #[test]
    fn without_a_single_title_the_file_name_is_the_root() {
        let map = from_markdown("- Leite\n- Pão\n  - Integral\n", "Compras");
        assert_eq!(map.root.text, "Compras");
        assert_eq!(texts(&map.root), ["Leite", "Pão"]);
        assert_eq!(texts(&map.root.children[1]), ["Integral"]);

        let two = from_markdown("# A\n# B\n", "Arquivo");
        assert_eq!(two.root.text, "Arquivo");
        assert_eq!(texts(&two.root), ["A", "B"]);
    }

    #[test]
    fn code_blocks_stay_whole_in_the_note() {
        let md = "- Script\n\n```sh\n# não é título\n- não é item\n```\n";
        let map = from_markdown(md, "x");
        assert_eq!(texts(&map.root), ["Script"]);
        assert!(map.root.children[0].note.contains("# não é título"));
        assert!(map.root.children[0].note.contains("- não é item"));
    }

    #[test]
    fn exported_markdown_comes_back_as_the_same_tree() {
        let md = "# Mapa\n\n- A\n  - A1\n    nota de A1\n  - A2\n- B\n";
        let map = from_markdown(md, "x");
        assert_eq!(crate::export::to_markdown(&map), md);
    }

    #[test]
    fn opml_from_other_apps_and_from_nodo_becomes_the_same_tree() {
        let other = r#"<?xml version="1.0"?><opml version="2.0"><head><title>Livros</title></head><body>
            <outline text="Ficção"><outline text="Duna" _note="Reler &amp; anotar"/></outline>
            <outline title="Ensaios"/></body></opml>"#;
        let map = from_opml(other, "x").unwrap();
        assert_eq!(map.root.text, "Livros");
        assert_eq!(texts(&map.root), ["Ficção", "Ensaios"]);
        assert_eq!(map.root.children[0].children[0].note, "Reler & anotar");

        // Ida e volta pelo OPML do próprio Nodo.
        let md = "# Mapa\n\n- A\n  - A1\n    nota\n- B\n";
        let original = from_markdown(md, "x");
        let back = from_opml(&crate::export::to_opml(&original), "x").unwrap();
        assert_eq!(crate::export::to_markdown(&back), md);

        assert!(from_opml("<minder/>", "x").is_err());
    }

    #[test]
    fn branches_of_the_root_alternate_sides() {
        use crate::model::NodeSide;
        let map = from_markdown("- A\n  - A1\n- B\n- C\n", "x");
        let sides: Vec<_> = map.root.children.iter().map(|c| c.side).collect();
        assert_eq!(sides, [NodeSide::Right, NodeSide::Left, NodeSide::Right]);
        assert_eq!(map.root.children[0].children[0].side, NodeSide::Right);
    }
}

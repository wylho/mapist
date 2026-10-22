//! Propriedades do layout e do arquivo, com mapas gerados ao acaso.

use nodo_core::{document, undo::Action, Editor, MindMap, NodeSide, SPACINGS, STRUCTURES};
use proptest::prelude::*;

/// Uma sequência de edições: (pai como índice entre os nós já criados, lado, largura, altura, recolher).
fn edits() -> impl Strategy<Value = Vec<(usize, bool, f64, f64, bool)>> {
    prop::collection::vec(
        (
            0usize..64,
            any::<bool>(),
            40.0f64..320.0,
            30.0f64..120.0,
            prop::bool::weighted(0.1),
        ),
        1..40,
    )
}

fn build(steps: &[(usize, bool, f64, f64, bool)]) -> Editor {
    let mut ed = Editor::new("Raiz");
    let mut ids = vec![0u64];
    for (i, (parent, left, w, h, fold)) in steps.iter().enumerate() {
        let parent = ids[parent % ids.len()];
        let Ok(Some(id)) = ed.apply(Action::AddChild {
            parent,
            text: format!("nó {i}"),
        }) else {
            continue;
        };
        ids.push(id);
        ed.set_sizes([(id, *w, *h)]);
        if parent == 0 && *left {
            let _ = ed.apply(Action::SetSide {
                id,
                side: NodeSide::Left,
            });
        }
        if *fold {
            let _ = ed.apply(Action::ToggleFold { id: parent });
        }
    }
    ed
}

/// Nós desenhados: os descendentes de um nó recolhido não contam.
fn visible(map: &MindMap) -> Vec<&nodo_core::Node> {
    fn walk<'a>(n: &'a nodo_core::Node, out: &mut Vec<&'a nodo_core::Node>) {
        out.push(n);
        if !n.folded {
            for c in &n.children {
                walk(c, out);
            }
        }
    }
    let mut out = Vec::new();
    walk(&map.root, &mut out);
    out
}

proptest! {
    #[test]
    fn visible_nodes_never_overlap(steps in edits()) {
        let ed = build(&steps);
        let nodes = visible(ed.map());
        for (i, a) in nodes.iter().enumerate() {
            for b in &nodes[i + 1..] {
                let overlap = a.pos.x < b.pos.x + b.width - 1e-6
                    && b.pos.x < a.pos.x + a.width - 1e-6
                    && a.pos.y < b.pos.y + b.height - 1e-6
                    && b.pos.y < a.pos.y + a.height - 1e-6;
                prop_assert!(!overlap, "{} sobrepõe {}", a.text, b.text);
            }
        }
    }

    #[test]
    fn no_structure_or_spacing_makes_nodes_overlap(steps in edits(), st in 0usize..STRUCTURES.len(), sp in 0usize..3) {
        let mut ed = build(&steps);
        ed.apply(Action::SetStructure { structure: Some(STRUCTURES[st].to_string()) }).unwrap();
        ed.apply(Action::SetSpacing { spacing: Some(SPACINGS[sp].to_string()) }).unwrap();
        let nodes = visible(ed.map());
        for (i, a) in nodes.iter().enumerate() {
            for b in &nodes[i + 1..] {
                let overlap = a.pos.x < b.pos.x + b.width - 1e-6
                    && b.pos.x < a.pos.x + a.width - 1e-6
                    && a.pos.y < b.pos.y + b.height - 1e-6
                    && b.pos.y < a.pos.y + a.height - 1e-6;
                prop_assert!(!overlap, "{}: {} sobrepõe {}", STRUCTURES[st], a.text, b.text);
            }
        }
        // E a estrutura vai e volta pelo arquivo.
        let back = document::from_xml(&document::to_xml(ed.map()).unwrap()).unwrap();
        prop_assert_eq!(back.theme.structure.as_deref(), Some(STRUCTURES[st]));
        prop_assert_eq!(back.theme.spacing.as_deref(), Some(SPACINGS[sp]));
    }

    #[test]
    fn tables_tile_every_topic_into_its_own_cell(steps in edits(), matrix in any::<bool>(), sp in 0usize..3) {
        let mut ed = build(&steps);
        let st = if matrix { "matriz" } else { "tabela" };
        ed.apply(Action::SetStructure { structure: Some(st.into()) }).unwrap();
        ed.apply(Action::SetSpacing { spacing: Some(SPACINGS[sp].to_string()) }).unwrap();
        let map = ed.map();
        // Células: (x, y, largura, altura), o tópico dentro da dele.
        let mut cells = Vec::new();
        for n in visible(map) {
            if let Some((_, _, w, h)) = n.cell {
                let (x, y, _, _) = n.cell.unwrap();
                prop_assert!(
                    n.pos.x >= x - 1e-6 && n.pos.y >= y - 1e-6 && n.pos.x + n.width <= x + w + 1e-6 && n.pos.y + n.height <= y + h + 1e-6,
                    "{st}: {} fora da própria célula", n.text
                );
                cells.push((n.text.clone(), n.pos, w, h));
            }
        }
        prop_assert!(map.root.cell.is_some(), "a raiz é a primeira célula");
        // Células não se sobrepõem (encostar pode), e cada tópico está dentro da sua.
        for (i, a) in cells.iter().enumerate() {
            for b in &cells[i + 1..] {
                let (ax, ay) = cell_origin(map, &a.0);
                let (bx, by) = cell_origin(map, &b.0);
                let _ = (a.1, b.1);
                let overlap = ax < bx + b.2 - 1e-6 && bx < ax + a.2 - 1e-6 && ay < by + b.3 - 1e-6 && by < ay + a.3 - 1e-6;
                prop_assert!(!overlap, "{st}: célula de {} sobrepõe a de {}", a.0, b.0);
            }
        }
        if matrix {
            // A n-ésima célula de cada linha começa na mesma coluna.
            let mut col_x: Vec<Option<f64>> = Vec::new();
            for row in map.root.children.iter().filter(|_| !map.root.folded) {
                if row.folded { continue }
                for (j, c) in row.children.iter().enumerate() {
                    if col_x.len() <= j { col_x.push(None) }
                    let x = c.pos.x;
                    match col_x[j] {
                        Some(x0) => prop_assert!((x - x0).abs() < 1e-6, "coluna {j} desalinhada"),
                        None => col_x[j] = Some(x),
                    }
                }
            }
        } else {
            // A célula do pai cobre a largura das células dos filhos.
            for n in visible(map) {
                if n.folded || n.children.is_empty() { continue }
                let (px, _) = cell_origin(map, &n.text);
                let (_, _, pw, _) = n.cell.unwrap();
                for c in &n.children {
                    let (cx, _) = cell_origin(map, &c.text);
                    let (_, _, cw, _) = c.cell.unwrap();
                    prop_assert!(cx >= px - 1e-6 && cx + cw <= px + pw + 1e-6, "{} fora da célula de {}", c.text, n.text);
                }
            }
        }
    }

    #[test]
    fn branches_grow_on_their_side(steps in edits()) {
        let ed = build(&steps);
        let root = &ed.map().root;
        for n in visible(ed.map()).into_iter().skip(1) {
            match n.side {
                NodeSide::Left => prop_assert!(n.pos.x + n.width <= root.pos.x + 1e-6),
                _ => prop_assert!(n.pos.x >= root.pos.x + root.width - 1e-6),
            }
        }
    }

    #[test]
    fn xml_round_trip_keeps_the_tree(steps in edits(), text in "[ -~áéíóúçãõ&<>\"']{0,24}") {
        let mut ed = build(&steps);
        let _ = ed.apply(Action::SetText { id: 0, text: text.clone() });
        let map = ed.map();
        let back = document::from_xml(&document::to_xml(map).map_err(|e| TestCaseError::fail(e.to_string()))?)
            .map_err(|e| TestCaseError::fail(e.to_string()))?;
        prop_assert_eq!(back.node_count(), map.node_count());
        prop_assert_eq!(&back.root.text, &text);
        for (a, b) in map.nodes().zip(back.nodes()) {
            prop_assert_eq!(a.id, b.id);
            prop_assert_eq!(a.side, b.side);
            prop_assert_eq!(a.folded, b.folded);
            prop_assert_eq!(&a.text, &b.text);
        }
    }

    #[test]
    fn nodo_round_trip_keeps_everything_modelled(steps in edits(), text in "\\PC{0,24}", note in "\\PC{0,40}") {
        let mut ed = build(&steps);
        let _ = ed.apply(Action::SetText { id: 0, text: text.clone() });
        let _ = ed.apply(Action::SetNote { id: 0, note: note.clone() });
        let map = ed.map();
        let back = nodo_core::nodofile::from_nodo(&nodo_core::nodofile::to_nodo(map).map_err(|e| TestCaseError::fail(e.to_string()))?)
            .map_err(|e| TestCaseError::fail(e.to_string()))?;
        prop_assert_eq!(back.node_count(), map.node_count());
        prop_assert_eq!(&back.root.note, &note);
        prop_assert_eq!(&back.theme.structure, &map.theme.structure);
        prop_assert_eq!(&back.theme.spacing, &map.theme.spacing);
        for (a, b) in map.nodes().zip(back.nodes()) {
            prop_assert_eq!((a.id, a.side, a.folded, &a.text, &a.note, &a.color), (b.id, b.side, b.folded, &b.text, &b.note, &b.color));
        }
    }

    #[test]
    fn undo_everything_returns_to_the_start(steps in edits()) {
        let mut ed = build(&steps);
        while ed.undo() {}
        prop_assert_eq!(ed.map().node_count(), 1);
        prop_assert!(!ed.is_dirty());
    }
}

/// Canto da célula de um tópico.
fn cell_origin(map: &MindMap, text: &str) -> (f64, f64) {
    let n = map.nodes().find(|n| n.text == text).expect("tópico");
    let (x, y, _, _) = n.cell.expect("célula");
    (x, y)
}

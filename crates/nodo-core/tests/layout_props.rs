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
    fn no_structure_or_spacing_makes_nodes_overlap(steps in edits(), st in 0usize..4, sp in 0usize..3) {
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
    fn undo_everything_returns_to_the_start(steps in edits()) {
        let mut ed = build(&steps);
        while ed.undo() {}
        prop_assert_eq!(ed.map().node_count(), 1);
        prop_assert!(!ed.is_dirty());
    }
}

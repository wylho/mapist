//! Parse a real Minder file and report what was recovered.
use mapist_core::{document, layout};

fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("usage: parse_real <file.minder>");
    let xml = std::fs::read_to_string(&path).expect("read");
    let mut map = document::from_xml(&xml).expect("parse");
    println!("version      : {}", map.version);
    println!("theme        : {} ({})", map.theme.label, map.theme.name);
    println!(
        "drawarea     : x={} y={} scale={}",
        map.draw_area.x, map.draw_area.y, map.draw_area.scale
    );
    println!("nodes        : {}", map.node_count());
    println!(
        "max depth    : {}",
        map.nodes()
            .map(|n| map.depth_of(n.id).unwrap_or(0))
            .max()
            .unwrap_or(0)
    );
    println!("styles       : {}", map.styles.len());
    println!(
        "colored nodes: {}",
        map.nodes().filter(|n| n.color.is_some()).count()
    );
    println!(
        "nodes w/ note: {}",
        map.nodes().filter(|n| !n.note.is_empty()).count()
    );
    layout::layout(&mut map);
    println!(
        "layout ok    : root at ({:.1}, {:.1})",
        map.root.pos.x, map.root.pos.y
    );
    let xml2 = document::to_xml(&map).expect("serialize");
    let back = document::from_xml(&xml2).expect("reparse");
    println!(
        "round-trip   : {} -> {} nodes ({})",
        map.node_count(),
        back.node_count(),
        if map.node_count() == back.node_count() {
            "OK"
        } else {
            "MISMATCH"
        }
    );
}

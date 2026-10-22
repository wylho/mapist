//! Document I/O for the native format.
//!
//! The on-disk shape is the XML structure observed in Minder 2.0.9. Import
//! reads that shape; export writes it. Unknown attributes are ignored rather
//! than rejected so that files from newer Minder releases still open.

use crate::model::*;
use crate::Error;
use quick_xml::events::Event;
use quick_xml::Reader;

/// Read a map from XML text.
pub fn from_xml(xml: &str) -> Result<MindMap, Error> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(false);

    let mut buf = Vec::new();
    let mut map: Option<MindMap> = None;
    let mut styles: Vec<Style> = Vec::new();
    let mut node_stack: Vec<Node> = Vec::new();
    let mut cur_text = String::new();
    let mut in_text = false;
    let mut free_note: Option<FreeNote> = None;

    loop {
        match reader.read_event_into(&mut buf) {
            Err(e) => return Err(Error::Xml(e.to_string())),
            Ok(Event::Eof) => break,
            // Nota solta sem texto: `<nodo-note .../>` não tem fim, entra já.
            Ok(Event::Empty(ref e)) if local_name(e.name().as_ref()) == "nodo-note" => {
                if let Some(m) = map.as_mut() {
                    m.notes.push(parse_free_note(e));
                }
            }
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                let name = local_name(e.name().as_ref());
                match name.as_str() {
                    "minder" => {
                        let version = attr(e, "version").unwrap_or_else(|| "1.15.0".into());
                        map = Some(MindMap {
                            version,
                            theme: ThemeRef::default(),
                            styles: Vec::new(),
                            draw_area: DrawArea::default(),
                            root: Node::new(0, ""),
                            connections: Vec::new(),
                            notes: Vec::new(),
                        });
                    }
                    "theme" => {
                        if let Some(m) = map.as_mut() {
                            m.theme = ThemeRef {
                                name: attr(e, "name").unwrap_or_else(|| "default".into()),
                                label: attr(e, "label").unwrap_or_else(|| "Light".into()),
                                index: attr(e, "index").and_then(|v| v.parse().ok()).unwrap_or(1),
                                // `mapist-style`: o mesmo atributo, gravado pela 0.1.0, que saiu com o nome Mapist.
                                style: attr(e, "nodo-style")
                                    .or_else(|| attr(e, "mapist-style"))
                                    .filter(|s| !s.is_empty()),
                                // Só nomes conhecidos: um valor estranho no arquivo vale como o padrão.
                                structure: attr(e, "nodo-structure")
                                    .filter(|s| crate::STRUCTURES.contains(&s.as_str())),
                                spacing: attr(e, "nodo-spacing")
                                    .filter(|s| crate::SPACINGS.contains(&s.as_str())),
                            };
                        }
                    }
                    "style" => {
                        let s = parse_style(e);
                        if node_stack.is_empty() {
                            styles.push(s);
                        } else if let Some(n) = node_stack.last_mut() {
                            n.style = s;
                        }
                    }
                    "drawarea" => {
                        if let Some(m) = map.as_mut() {
                            m.draw_area = DrawArea {
                                x: attr_f64(e, "x", 0.0),
                                y: attr_f64(e, "y", 0.0),
                                scale: attr_f64(e, "scale", 1.0),
                            };
                        }
                    }
                    "node" => {
                        let mut n =
                            Node::new(attr(e, "id").and_then(|v| v.parse().ok()).unwrap_or(0), "");
                        n.pos = Point {
                            x: attr_f64(e, "posx", 0.0),
                            y: attr_f64(e, "posy", 0.0),
                        };
                        n.width = attr_f64(e, "width", 63.0);
                        n.height = attr_f64(e, "height", 46.0);
                        n.side = NodeSide::parse(&attr(e, "side").unwrap_or_default());
                        n.folded = attr_bool(e, "fold", false);
                        n.tree_size = attr_f64(e, "treesize", 46.0);
                        n.layout = LayoutKind::parse(&attr(e, "layout").unwrap_or_default());
                        n.grouped = attr_bool(e, "group", false);
                        n.color = attr(e, "color");
                        n.color_root = attr_bool(e, "colorroot", false);
                        node_stack.push(n);
                    }
                    "text" => {
                        if let Some(v) = attr(e, "data") {
                            if let Some(n) = node_stack.last_mut() {
                                n.text = v;
                            }
                        }
                    }
                    "nodo-note" => {
                        free_note = Some(parse_free_note(e));
                        cur_text.clear();
                        in_text = true;
                    }
                    "nodenote" => {
                        // Note content arrives as character data inside the
                        // element, not as an attribute.
                        cur_text.clear();
                        in_text = true;
                    }
                    _ => {}
                }
            }
            Ok(Event::Text(ref t)) => {
                if in_text {
                    cur_text.push_str(&t.unescape().unwrap_or_default());
                }
            }
            Ok(Event::End(ref e)) => {
                let name = local_name(e.name().as_ref());
                match name.as_str() {
                    "node" => {
                        if let Some(done) = node_stack.pop() {
                            match node_stack.last_mut() {
                                Some(parent) => parent.children.push(done),
                                None => {
                                    if let Some(m) = map.as_mut() {
                                        m.root = done;
                                    }
                                }
                            }
                        }
                    }
                    "nodename" => in_text = false,
                    "nodo-note" => {
                        in_text = false;
                        if let (Some(m), Some(mut n)) = (map.as_mut(), free_note.take()) {
                            n.text = std::mem::take(&mut cur_text);
                            m.notes.push(n);
                        }
                    }
                    "nodenote" => {
                        in_text = false;
                        if let Some(n) = node_stack.last_mut() {
                            n.note = std::mem::take(&mut cur_text);
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::CData(ref t)) if in_text => {
                cur_text.push_str(&String::from_utf8_lossy(t));
            }
            _ => {}
        }
        buf.clear();
    }

    let mut m = map.ok_or(Error::MissingRoot)?;
    if !styles.is_empty() {
        m.styles = styles;
    }
    if m.styles.is_empty() {
        m.styles.push(Style::default());
    }
    Ok(m)
}

/// Write a map as XML text.
pub fn to_xml(map: &MindMap) -> Result<String, Error> {
    let mut out = String::with_capacity(4096);
    out.push_str("<?xml version=\"1.0\"?>\n");
    out.push_str(&format!("<minder version=\"{}\">\n", esc(&map.version)));
    // The map theme rides on an extra attribute, absent when unset, so a map
    // without one stays byte-for-byte what Minder writes.
    let extra = |name: &str, value: &Option<String>| {
        value
            .as_deref()
            .map(|s| format!(" {name}=\"{}\"", esc(s)))
            .unwrap_or_default()
    };
    let style = extra("nodo-style", &map.theme.style)
        + &extra("nodo-structure", &map.theme.structure)
        + &extra("nodo-spacing", &map.theme.spacing);
    out.push_str(&format!(
        "  <theme name=\"{}\" label=\"{}\" index=\"{}\"{}/>\n",
        esc(&map.theme.name),
        esc(&map.theme.label),
        map.theme.index,
        style
    ));
    out.push_str("  <styles>\n");
    for (i, s) in map.styles.iter().enumerate() {
        out.push_str(&format!("    <style level=\"{}\" {}", i, style_attrs(s)));
        out.push_str("/>\n");
    }
    out.push_str("  </styles>\n");
    out.push_str(&format!(
        "  <drawarea x=\"{}\" y=\"{}\" scale=\"{}\"/>\n",
        map.draw_area.x, map.draw_area.y, map.draw_area.scale
    ));
    out.push_str("  <images/>\n");
    out.push_str("  <nodes>\n");
    write_node(&mut out, &map.root, 2);
    out.push_str("  </nodes>\n");
    // Notas soltas no mapa: elemento só do Nodo, ausente quando não há nenhuma (o arquivo fica como o do Minder).
    if !map.notes.is_empty() {
        out.push_str("  <nodo-notes>\n");
        for n in &map.notes {
            out.push_str(&format!(
                "    <nodo-note id=\"{}\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\">{}</nodo-note>\n",
                n.id,
                n.pos.x,
                n.pos.y,
                n.width,
                n.height,
                esc(&n.text)
            ));
        }
        out.push_str("  </nodo-notes>\n");
    }
    out.push_str("</minder>\n");
    Ok(out)
}

fn write_node(out: &mut String, n: &Node, indent: usize) {
    let pad = "  ".repeat(indent);
    let mut attrs = format!(
        "id=\"{}\" posx=\"{}\" posy=\"{}\" width=\"{}\" height=\"{}\" side=\"{}\" fold=\"{}\" treesize=\"{}\" layout=\"{}\" group=\"{}\"",
        n.id,
        n.pos.x,
        n.pos.y,
        n.width,
        n.height,
        n.side.as_str(),
        n.folded,
        n.tree_size,
        n.layout.as_str(),
        n.grouped
    );
    if let Some(c) = &n.color {
        attrs.push_str(&format!(" color=\"{}\"", esc(c)));
    }
    if n.color_root {
        attrs.push_str(" colorroot=\"true\"");
    }
    out.push_str(&format!("{}<node {}>\n", pad, attrs));
    out.push_str(&format!("{}  <style {}/>\n", pad, style_attrs(&n.style)));
    out.push_str(&format!(
        "{}  <nodename posx=\"{}\" posy=\"{}\" maxwidth=\"{}\">\n",
        pad,
        n.pos.x + 14.0,
        n.pos.y + 14.0,
        n.style.node_width
    ));
    out.push_str(&format!("{}    <text data=\"{}\"/>\n", pad, esc(&n.text)));
    out.push_str(&format!("{}  </nodename>\n", pad));
    out.push_str(&format!("{}  <nodenote>{}</nodenote>\n", pad, esc(&n.note)));
    if !n.children.is_empty() {
        out.push_str(&format!("{}  <nodes>\n", pad));
        for c in &n.children {
            write_node(out, c, indent + 2);
        }
        out.push_str(&format!("{}  </nodes>\n", pad));
    }
    out.push_str(&format!("{}</node>\n", pad));
}

fn parse_free_note(e: &quick_xml::events::BytesStart) -> FreeNote {
    FreeNote {
        id: attr(e, "id").and_then(|v| v.parse().ok()).unwrap_or(0),
        pos: Point {
            x: attr_f64(e, "x", 0.0),
            y: attr_f64(e, "y", 0.0),
        },
        width: attr_f64(e, "width", crate::model::FREE_NOTE_WIDTH),
        height: attr_f64(e, "height", crate::model::FREE_NOTE_HEIGHT),
        text: String::new(),
        ..Default::default()
    }
}

fn style_attrs(s: &Style) -> String {
    format!(
        "branchmargin=\"{}\" branchradius=\"{}\" linktype=\"{}\" linkwidth=\"{}\" linkarrow=\"{}\" linkdash=\"{}\" nodeborder=\"{}\" nodewidth=\"{}\" nodeborderwidth=\"{}\" nodefill=\"{}\" nodemargin=\"{}\" nodepadding=\"{}\" nodefont=\"{}\" nodemarkup=\"{}\" connectiondash=\"{}\" connectionlwidth=\"{}\" connectionarrow=\"{}\" connectionpadding=\"{}\" connectionfont=\"{}\" connectiontwidth=\"{}\"",
        s.branch_margin,
        s.branch_radius,
        s.link_type.as_str(),
        s.link_width,
        s.link_arrow,
        s.link_dash.as_str(),
        s.node_border.as_str(),
        s.node_width,
        s.node_border_width,
        s.node_fill,
        s.node_margin,
        s.node_padding,
        esc(&s.node_font),
        s.node_markup,
        s.connection_dash.as_str(),
        s.connection_line_width,
        s.connection_arrow.as_str(),
        s.connection_padding,
        esc(&s.connection_font),
        s.connection_text_width
    )
}

fn parse_style(e: &quick_xml::events::BytesStart) -> Style {
    let d = Style::default();
    Style {
        branch_margin: attr_f64(e, "branchmargin", d.branch_margin),
        branch_radius: attr_f64(e, "branchradius", d.branch_radius),
        link_type: BorderStyle::parse(&attr(e, "linktype").unwrap_or_default()),
        link_width: attr_f64(e, "linkwidth", d.link_width),
        link_arrow: attr_bool(e, "linkarrow", d.link_arrow),
        link_dash: DashStyle::parse(&attr(e, "linkdash").unwrap_or_default()),
        node_border: BorderStyle::parse(&attr(e, "nodeborder").unwrap_or_default()),
        node_width: attr_f64(e, "nodewidth", d.node_width),
        node_border_width: attr_f64(e, "nodeborderwidth", d.node_border_width),
        node_fill: attr_bool(e, "nodefill", d.node_fill),
        node_margin: attr_f64(e, "nodemargin", d.node_margin),
        node_padding: attr_f64(e, "nodepadding", d.node_padding),
        node_font: attr(e, "nodefont").unwrap_or(d.node_font),
        node_markup: attr_bool(e, "nodemarkup", d.node_markup),
        connection_dash: DashStyle::parse(&attr(e, "connectiondash").unwrap_or_default()),
        connection_line_width: attr_f64(e, "connectionlwidth", d.connection_line_width),
        connection_arrow: ArrowStyle::parse(&attr(e, "connectionarrow").unwrap_or_default()),
        connection_padding: attr_f64(e, "connectionpadding", d.connection_padding),
        connection_font: attr(e, "connectionfont").unwrap_or(d.connection_font),
        connection_text_width: attr_f64(e, "connectiontwidth", d.connection_text_width),
    }
}

pub(crate) fn local_name(raw: &[u8]) -> String {
    let s = String::from_utf8_lossy(raw);
    match s.rsplit(':').next() {
        Some(n) => n.to_string(),
        None => s.to_string(),
    }
}

pub(crate) fn attr(e: &quick_xml::events::BytesStart, key: &str) -> Option<String> {
    e.attributes().flatten().find_map(|a| {
        if local_name(a.key.as_ref()) == key {
            a.unescape_value().ok().map(|v| v.into_owned())
        } else {
            None
        }
    })
}

fn attr_f64(e: &quick_xml::events::BytesStart, key: &str, default: f64) -> f64 {
    attr(e, key).and_then(|v| v.parse().ok()).unwrap_or(default)
}

fn attr_bool(e: &quick_xml::events::BytesStart, key: &str, default: bool) -> bool {
    attr(e, key)
        .map(|v| v == "true" || v == "1")
        .unwrap_or(default)
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

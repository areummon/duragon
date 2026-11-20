//! Allow-list SVG sanitizer for model-generated drawings.
use roxmltree::{Document, Node, NodeType};

const ELEMENTS: &[&str] = &[
    "svg", "g", "path", "line", "polyline", "polygon", "rect", "circle", "ellipse", "text", "tspan",
];

const ATTRIBUTES: &[&str] = &[
    "viewBox", "x", "y", "x1", "y1", "x2", "y2", "cx", "cy", "r", "rx", "ry", "width", "height",
    "d", "points", "transform", "text-anchor", "dominant-baseline", "style", "data-step",
    "data-caption", "font-size", "stroke-width", "stroke-dasharray", "opacity", "fill", "stroke",
    "stroke-linecap", "stroke-linejoin", "font-family", "font-weight", "font-style",
];

const SVG_NS: &str = "http://www.w3.org/2000/svg";

fn escape(s: &str, out: &mut String) {
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            c => out.push(c),
        }
    }
}

fn unsafe_value(v: &str) -> bool {
    let v = v.to_ascii_lowercase();
    ["url(", "expression", "javascript:", "@import", "-moz-binding", "behavior"]
        .iter()
        .any(|bad| v.contains(bad))
}

fn clean_style(style: &str) -> String {
    style
        .split(';')
        .map(str::trim)
        .filter(|d| !d.is_empty() && !unsafe_value(d))
        .collect::<Vec<_>>()
        .join(";")
}

fn write(node: Node, out: &mut String, root: bool) {
    let name = node.tag_name().name();
    if !ELEMENTS.contains(&name) {
        return;
    }
    out.push('<');
    out.push_str(name);
    if root {
        out.push_str(" xmlns=\"");
        out.push_str(SVG_NS);
        out.push('"');
    }
    for a in node.attributes() {
        if a.namespace().is_some() || !ATTRIBUTES.contains(&a.name()) {
            continue;
        }
        let value = if a.name() == "style" {
            clean_style(a.value())
        } else if unsafe_value(a.value()) {
            continue;
        } else {
            a.value().to_string()
        };
        out.push(' ');
        out.push_str(a.name());
        out.push_str("=\"");
        escape(&value, out);
        out.push('"');
    }
    out.push('>');
    for child in node.children() {
        match child.node_type() {
            NodeType::Element => write(child, out, false),
            NodeType::Text if matches!(name, "text" | "tspan") => {
                escape(child.text().unwrap_or(""), out);
            }
            _ => {}
        }
    }
    out.push_str("</");
    out.push_str(name);
    out.push('>');
}

/// Returns a re-serialized, allow-listed SVG, or an error when the input is not an SVG document.
pub fn sanitize(input: &str) -> Result<String, String> {
    let doc = Document::parse(input.trim()).map_err(|e| format!("invalid SVG: {e}"))?;
    let root = doc.root_element();
    if root.tag_name().name() != "svg" {
        return Err("root element is not <svg>".into());
    }
    let mut out = String::new();
    write(root, &mut out, true);
    Ok(out)
}

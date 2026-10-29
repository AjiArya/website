use std::collections::BTreeSet;
use std::fmt::Write as _;

use anyhow::Result;

use crate::diagram::icons;
use crate::diagram::layout::{DiagramLayout, LayoutEdge, LayoutGroup, LayoutNode, LayoutPort};
use crate::diagram::model::{Arrow, EdgeAnimation, EdgeStyle, NodeKind, Shape};

pub fn render(layout: &DiagramLayout, standalone: bool) -> Result<String> {
    let mut svg = String::new();
    let prefix = &layout.prefix;
    let title_id = format!("{prefix}-title");
    let desc_id = format!("{prefix}-desc");

    let static_diagram = layout
        .edges
        .iter()
        .all(|edge| edge.animation == EdgeAnimation::None);
    let data_attrs = format!(
        r#"data-dg-draw-ms="{}" data-dg-flow-ms="{}" data-dg-stagger-ms="{}" data-dg-direction="{}" data-dg-autoplay="{}" data-dg-static="{}" data-dg-no-drag="{}" data-dg-fit-mobile="{}""#,
        layout.draw_ms,
        layout.flow_ms,
        layout.stagger_ms,
        layout.direction.as_str(),
        layout.autoplay,
        static_diagram,
        layout.no_drag,
        layout.fit_mobile
    );
    let style_vars = format!(
        "--dg-draw-duration:{}ms;--dg-flow-duration:{}ms;--dg-pulse-duration:2000ms",
        layout.draw_ms, layout.flow_ms
    );

    if standalone {
        write!(
            svg,
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {:.0} {:.0}" width="{:.0}" height="{:.0}" role="img" aria-labelledby="{title_id} {desc_id}" class="dg-svg" {} style="{};max-width:100%;height:auto">"#,
            layout.width, layout.height, layout.width, layout.height, data_attrs, style_vars
        )?;
    } else {
        write!(
            svg,
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {:.0} {:.0}" role="img" aria-labelledby="{title_id} {desc_id}" class="dg-svg" {} style="{}">"#,
            layout.width, layout.height, data_attrs, style_vars
        )?;
    }

    if standalone {
        write!(svg, "<style>{STANDALONE_CSS}</style>")?;
    }

    write!(
        svg,
        "<title id=\"{}\">{}</title><desc id=\"{}\">{}</desc>",
        escape_attr(&title_id),
        escape_xml(&layout.title),
        escape_attr(&desc_id),
        escape_xml(&layout.description)
    )?;

    let mut marker_tones = BTreeSet::new();
    for edge in &layout.edges {
        if edge.arrow != Arrow::None {
            marker_tones.insert(edge.tone.as_str());
        }
    }

    if !marker_tones.is_empty() {
        svg.push_str("<defs>");
        for tone in marker_tones {
            write!(
                svg,
                r#"<marker id="{}-arrow-{}" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse"><path d="M 0 1 L 9 5 L 0 9 z" class="dg-arrow dg-tone-{}"/></marker>"#,
                escape_attr(prefix),
                escape_attr(tone),
                escape_attr(tone)
            )?;
        }
        svg.push_str("</defs>");
    }

    if !layout.groups.is_empty() {
        svg.push_str(r#"<g class="dg-groups">"#);
        for group in &layout.groups {
            render_group(&mut svg, group);
        }
        svg.push_str("</g>");
    }

    if !layout.networks.is_empty() {
        svg.push_str(r#"<g class="dg-network-legend">"#);
        for (index, network) in layout.networks.iter().enumerate() {
            let y = layout.padding + 10.0 + index as f64 * 16.0;
            write!(
                svg,
                r#"<line class="network-legend-swatch dg-tone-{}" x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}"/><text class="network-legend-label dg-tone-{}" x="{:.1}" y="{:.1}">{}</text>"#,
                escape_attr(&network.tone),
                layout.padding,
                y,
                layout.padding + 18.0,
                y,
                escape_attr(&network.tone),
                layout.padding + 26.0,
                y + 3.0,
                escape_xml(&network.label)
            )?;
        }
        svg.push_str("</g>");
        svg.push_str(r#"<g class="dg-networks">"#);
        for network in &layout.networks {
            render_network(&mut svg, layout, network)?;
        }
        svg.push_str("</g>");
    }

    svg.push_str(r#"<g class="dg-edges">"#);
    for (index, edge) in layout.edges.iter().enumerate() {
        render_edge(&mut svg, prefix, index, edge);
    }
    svg.push_str("</g>");

    if !layout.bonds.is_empty() {
        svg.push_str(r#"<g class="dg-bonds">"#);
        for bond in &layout.bonds {
            write!(
                svg,
                r#"<ellipse class="dg-bond-mark dg-tone-{}" cx="{:.1}" cy="{:.1}" rx="{:.1}" ry="{:.1}"/><text class="dg-bond-label dg-tone-{}" x="{:.1}" y="{:.1}" text-anchor="middle">{}</text>"#,
                escape_attr(&bond.tone),
                bond.x,
                bond.y,
                bond.rx,
                bond.ry,
                escape_attr(&bond.tone),
                bond.x,
                bond.y - bond.ry - 5.0,
                escape_xml(&bond.label)
            )?;
        }
        svg.push_str("</g>");
    }

    if !standalone {
        let packets: Vec<(usize, &LayoutEdge)> = layout
            .edges
            .iter()
            .enumerate()
            .filter(|(_, edge)| edge.animation == EdgeAnimation::Flow)
            .collect();
        if !packets.is_empty() {
            svg.push_str(r#"<g class="dg-packets" aria-hidden="true">"#);
            for (index, edge) in packets {
                write!(
                    svg,
                    r#"<circle class="dg-packet dg-tone-{}" data-dg-edge="{}" r="4.5" cx="0" cy="0" opacity="0"/>"#,
                    escape_attr(&edge.tone),
                    index
                )?;
            }
            svg.push_str("</g>");
        }
    }

    svg.push_str(r#"<g class="dg-nodes">"#);
    for node in &layout.nodes {
        render_node(&mut svg, node);
    }
    svg.push_str("</g>");

    if layout.edges.iter().any(|edge| edge.label.is_some()) {
        svg.push_str(r#"<g class="dg-edge-labels">"#);
        for (index, edge) in layout.edges.iter().enumerate() {
            if let Some(label) = &edge.label {
                write!(
                    svg,
                    r#"<text class="dg-edge-label" data-dg-edge="{}" x="{:.1}" y="{:.1}" text-anchor="middle">{}</text>"#,
                    index,
                    edge.label_pos.x,
                    edge.label_pos.y - 7.0,
                    escape_xml(label)
                )?;
            }
        }
        svg.push_str("</g>");
    }

    svg.push_str("</svg>");
    Ok(svg)
}

fn render_group(svg: &mut String, group: &LayoutGroup) {
    let rect = group.rect;
    write!(
        svg,
        r#"<g class="dg-group dg-tone-{}" data-group="{}"><rect class="dg-group-box" x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" rx="10"/><text class="dg-group-label" x="{:.1}" y="{:.1}">{}</text></g>"#,
        escape_attr(&group.tone),
        escape_attr(&group.id),
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        rect.x + 14.0,
        rect.y + 21.0,
        escape_xml(&group.label)
    )
    .expect("writing to String cannot fail");
}

fn render_edge(svg: &mut String, prefix: &str, index: usize, edge: &LayoutEdge) {
    let mut classes = format!(
        "dg-edge dg-tone-{} dg-style-{}",
        edge.tone,
        style_class(edge.style)
    );
    if let Some(animation) = animation_class(edge.animation) {
        classes.push(' ');
        classes.push_str(animation);
    }

    let mut markers = String::new();
    if edge.arrow != Arrow::None {
        let marker = format!(
            "url(#{}-arrow-{})",
            escape_attr(prefix),
            escape_attr(&edge.tone)
        );
        if edge.arrow == Arrow::Both {
            write!(markers, " marker-start=\"{marker}\"").expect("string write");
        }
        write!(markers, " marker-end=\"{marker}\"").expect("string write");
    }

    let animation = animation_name(edge.animation);
    let direction = edge
        .direction
        .map(|direction| direction.as_str())
        .unwrap_or("");

    write!(
        svg,
        r#"<path class="{}" d="{}" pathLength="1" data-from="{}" data-to="{}" data-dg-edge="{}" data-dg-route="{}" data-dg-animation="{}" data-dg-direction="{}" style="--dg-delay:{}ms"{} />"#,
        classes,
        escape_attr(&edge.path),
        escape_attr(&edge.from),
        escape_attr(&edge.to),
        index,
        route_name(edge.route),
        animation,
        direction,
        edge.delay_ms,
        markers
    )
    .expect("writing to String cannot fail");
}

fn render_node(svg: &mut String, node: &LayoutNode) {
    if node.arch.is_some() {
        render_architecture_node(svg, node);
        return;
    }
    let rect = node.rect;
    let radius = match node.shape {
        Shape::Box => 6.0,
        Shape::Pill => rect.h / 2.0,
    };
    write!(
        svg,
        r#"<g class="dg-node dg-tone-{}" data-node="{}" data-group="{}" data-dg-lb="{}" data-dg-lb-algorithm="{}"><rect class="dg-node-box" x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" rx="{:.1}"/><rect class="dg-node-accent" x="{:.1}" y="{:.1}" width="3" height="{:.1}" rx="1.5"/>"#,
        escape_attr(&node.tone),
        escape_attr(&node.id),
        escape_attr(node.group.as_deref().unwrap_or("")),
        node.load_balancer,
        node.lb_algorithm.as_str(),
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        radius,
        rect.x,
        rect.y,
        rect.h
    )
    .expect("writing to String cannot fail");

    let text_x;
    let text_anchor;
    if let Some(icon) = &node.icon {
        let ix = rect.x + 16.0;
        let iy = rect.y + rect.h / 2.0 - 11.0;
        write!(
            svg,
            r#"<g class="dg-node-icon" transform="translate({ix:.1},{iy:.1}) scale(0.8333)">{}</g>"#,
            icons::markup(icon).unwrap_or_default()
        )
        .expect("writing to String cannot fail");
        text_x = rect.x + 44.0;
        text_anchor = "start";
    } else {
        text_x = rect.x + rect.w / 2.0;
        text_anchor = "middle";
    }

    let line_count = node.lines.len() as f64;
    let first_y = rect.y + rect.h / 2.0 - (line_count - 1.0) * 9.0 + 5.0;
    write!(
        svg,
        r#"<text class="dg-node-label" x="{:.1}" text-anchor="{}">"#,
        text_x, text_anchor
    )
    .expect("writing to String cannot fail");
    for (index, line) in node.lines.iter().enumerate() {
        write!(
            svg,
            r#"<tspan x="{:.1}" y="{:.1}">{}</tspan>"#,
            text_x,
            first_y + index as f64 * 18.0,
            escape_xml(line)
        )
        .expect("writing to String cannot fail");
    }
    svg.push_str("</text></g>");
}

fn render_architecture_node(svg: &mut String, node: &LayoutNode) {
    let arch = node.arch.as_ref().unwrap();
    let rect = node.rect;
    let radius = match node.shape {
        Shape::Box => 6.0,
        Shape::Pill => rect.h / 2.0,
    };
    let kind_class = match arch.kind {
        NodeKind::Switch => "dg-node--switch",
        NodeKind::Server => "dg-node--server",
        NodeKind::Vm => "dg-node--vm",
        NodeKind::Generic => "dg-node--generic",
    };
    write!(
        svg,
        r#"<g class="dg-node dg-tone-{} {}" data-node="{}" data-group="{}" data-dg-lb="{}" data-dg-lb-algorithm="{}" data-dg-kind="{}"><rect class="dg-node-box" x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" rx="{:.1}"/><rect class="dg-node-accent" x="{:.1}" y="{:.1}" width="3" height="{:.1}" rx="1.5"/>"#,
        escape_attr(&node.tone),
        kind_class,
        escape_attr(&node.id),
        escape_attr(node.group.as_deref().unwrap_or("")),
        node.load_balancer,
        node.lb_algorithm.as_str(),
        escape_attr(match arch.kind {
            NodeKind::Switch => "switch",
            NodeKind::Server => "server",
            NodeKind::Vm => "vm",
            NodeKind::Generic => "generic",
        }),
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        radius,
        rect.x,
        rect.y,
        rect.h
    )
    .expect("writing to String cannot fail");

    match arch.kind {
        NodeKind::Switch => render_architecture_switch(svg, node),
        NodeKind::Server | NodeKind::Vm => render_architecture_host(svg, node),
        _ => {}
    }

    svg.push_str("</g>");
}

fn render_architecture_switch(svg: &mut String, node: &LayoutNode) {
    let arch = node.arch.as_ref().unwrap();
    let rect = node.rect;
    let pad = 10.0;
    let icon = node.icon.as_deref().unwrap_or("switch");
    if let Some(markup) = icons::markup(icon) {
        write!(
            svg,
            r#"<g class="dg-node-icon" transform="translate({:.1},{:.1}) scale(0.7)">{}</g>"#,
            rect.x + pad,
            rect.y + pad + 2.0,
            markup
        )
        .expect("writing to String cannot fail");
    }
    let title = node.lines.join(" ");
    write!(
        svg,
        r#"<text class="arch-node-title" x="{:.1}" y="{:.1}">{}</text>"#,
        rect.x + pad + 24.0,
        rect.y + pad + 16.0,
        escape_xml(&title)
    )
    .expect("writing to String cannot fail");
    write!(
        svg,
        r#"<text class="arch-node-meta" x="{:.1}" y="{:.1}" text-anchor="end">{} ports</text>"#,
        rect.x + rect.w - pad,
        rect.y + pad + 16.0,
        arch.ports.len()
    )
    .expect("writing to String cannot fail");

    for port in &arch.ports {
        render_architecture_port(svg, port);
    }
}

fn render_architecture_host(svg: &mut String, node: &LayoutNode) {
    let arch = node.arch.as_ref().unwrap();
    let rect = node.rect;

    for nic in &arch.nics {
        write!(
            svg,
            r#"<rect class="arch-nic dg-tone-{}" x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" rx="4"/>"#,
            escape_attr(&nic.tone),
            nic.rect.x,
            nic.rect.y,
            nic.rect.w,
            nic.rect.h
        )
        .expect("writing to String cannot fail");
        write!(
            svg,
            r#"<text class="arch-nic-label" x="{:.1}" y="{:.1}" text-anchor="middle">{}</text>"#,
            nic.rect.center().x,
            nic.rect.bottom() - 3.5,
            escape_xml(&nic.id)
        )
        .expect("writing to String cannot fail");
    }

    for port in &arch.ports {
        render_architecture_port(svg, port);
    }

    let icon = node.icon.as_deref().unwrap_or(match arch.kind {
        NodeKind::Vm => "vm",
        _ => "server",
    });
    let text_x;
    let text_anchor;
    if let Some(markup) = icons::markup(icon) {
        let ix = rect.x + 16.0;
        let iy = rect.y + rect.h - 31.0;
        write!(
            svg,
            r#"<g class="dg-node-icon" transform="translate({ix:.1},{iy:.1}) scale(0.7)">{markup}</g>"#
        )
        .expect("writing to String cannot fail");
        text_x = rect.x + 42.0;
        text_anchor = "start";
    } else {
        text_x = rect.x + rect.w / 2.0;
        text_anchor = "middle";
    }

    let line_count = node.lines.len() as f64;
    let first_y = rect.y + rect.h - 22.0 - (line_count - 1.0) * 8.0;
    write!(
        svg,
        r#"<text class="arch-node-title" x="{:.1}" y="{:.1}" text-anchor="{}">"#,
        text_x, first_y, text_anchor
    )
    .expect("writing to String cannot fail");
    for (index, line) in node.lines.iter().enumerate() {
        write!(
            svg,
            r#"<tspan x="{:.1}" y="{:.1}">{}</tspan>"#,
            text_x,
            first_y + index as f64 * 16.0,
            escape_xml(line)
        )
        .expect("writing to String cannot fail");
    }
    svg.push_str("</text>");
}

fn render_architecture_port(svg: &mut String, port: &LayoutPort) {
    let used = if port.used { " is-used" } else { "" };
    write!(
        svg,
        r#"<rect class="arch-port dg-tone-{} {}" x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" rx="1"/>"#,
        escape_attr(&port.tone),
        used.trim(),
        port.rect.x,
        port.rect.y,
        port.rect.w,
        port.rect.h
    )
    .expect("writing to String cannot fail");
    write!(
        svg,
        r#"<text class="arch-port-label{}" x="{:.1}" y="{:.1}" text-anchor="middle">{}</text>"#,
        used,
        port.rect.center().x,
        port.rect.center().y + 2.0,
        port.index
    )
    .expect("writing to String cannot fail");
}

fn render_network(
    svg: &mut String,
    layout: &DiagramLayout,
    network: &crate::diagram::layout::LayoutNetwork,
) -> Result<()> {
    write!(
        svg,
        r#"<line class="network-rail dg-tone-{}" x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}"/>"#,
        escape_attr(&network.tone),
        network.x1,
        network.y1,
        network.x2,
        network.y2
    )?;
    for member in &network.members {
        if let Some(node) = layout.nodes.iter().find(|node| &node.id == member) {
            let (x1, y1, x2, y2) = if network.horizontal {
                let x = node.rect.center().x;
                if network.y1 < node.rect.center().y {
                    (x, node.rect.y, x, network.y1)
                } else {
                    (x, node.rect.bottom(), x, network.y1)
                }
            } else if network.x1 < node.rect.center().x {
                (
                    node.rect.x,
                    node.rect.center().y,
                    network.x1,
                    node.rect.center().y,
                )
            } else {
                (
                    node.rect.right(),
                    node.rect.center().y,
                    network.x1,
                    node.rect.center().y,
                )
            };
            write!(
                svg,
                r#"<line class="network-stub dg-tone-{}" x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}"/>"#,
                escape_attr(&network.tone),
                x1,
                y1,
                x2,
                y2
            )?;
        }
    }
    Ok(())
}

fn style_class(style: EdgeStyle) -> &'static str {
    match style {
        EdgeStyle::Solid => "solid",
        EdgeStyle::Dashed => "dashed",
        EdgeStyle::Dotted => "dotted",
    }
}

fn route_name(route: crate::diagram::model::Route) -> &'static str {
    match route {
        crate::diagram::model::Route::Elbow => "elbow",
        crate::diagram::model::Route::Straight => "straight",
        crate::diagram::model::Route::Curve => "curve",
    }
}

fn animation_class(animation: EdgeAnimation) -> Option<&'static str> {
    match animation {
        EdgeAnimation::Draw => Some("dg-edge--draw"),
        EdgeAnimation::Flow => Some("dg-edge--flow"),
        EdgeAnimation::Pulse => Some("dg-edge--pulse"),
        EdgeAnimation::None | EdgeAnimation::Inherit => None,
    }
}

fn animation_name(animation: EdgeAnimation) -> &'static str {
    match animation {
        EdgeAnimation::Draw => "draw",
        EdgeAnimation::Flow => "flow",
        EdgeAnimation::Pulse => "pulse",
        EdgeAnimation::None | EdgeAnimation::Inherit => "none",
    }
}

pub fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

pub fn escape_attr(value: &str) -> String {
    escape_xml(value)
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// Styles embedded into standalone SVG exports. The site uses the same rules in
/// `sass/_diagram.scss`; keep the class names in sync when changing either one.
const STANDALONE_CSS: &str = r#"
:root{color-scheme:dark;--dg-bg:#0b0d0c;--dg-node-bg:#101411;--dg-node-border:#2c3a30;--dg-text:#d8e3d8;--dg-muted:#7f9182;--dg-edge:#7f9182;--dg-label-halo:#0b0d0c;--dg-group-bg:#101411;--dg-group-border:#243027;--dg-neutral:#8b9c8d;--dg-blue:#79b8ff;--dg-violet:#b7a1ff;--dg-green:#1cb900;--dg-amber:#e6c26e;--dg-red:#ff8b7a;--dg-cyan:#7fe0e0;--dg-pink:#f3a0c8}
.dg-svg{background:var(--dg-bg);font-family:ui-sans-serif,-apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif}
.dg-tone-neutral{--dg-tone:var(--dg-neutral)}.dg-tone-blue{--dg-tone:var(--dg-blue)}.dg-tone-violet{--dg-tone:var(--dg-violet)}.dg-tone-green{--dg-tone:var(--dg-green)}.dg-tone-amber{--dg-tone:var(--dg-amber)}.dg-tone-red{--dg-tone:var(--dg-red)}.dg-tone-cyan{--dg-tone:var(--dg-cyan)}.dg-tone-pink{--dg-tone:var(--dg-pink)}
.dg-group-box{fill:var(--dg-group-bg);stroke:var(--dg-group-border);stroke-width:1}
.dg-group-label{fill:var(--dg-muted);font-size:10.5px;font-weight:650;letter-spacing:.12em;text-transform:uppercase;font-family:ui-monospace,SFMono-Regular,Menlo,Consolas,monospace}
.dg-node-box{fill:var(--dg-node-bg);stroke:var(--dg-node-border);stroke-width:1.2}
.dg-node-accent{fill:var(--dg-tone)}
.dg-node-label{fill:var(--dg-text);font-size:13px;font-weight:560;font-family:ui-monospace,SFMono-Regular,Menlo,Consolas,monospace}
.dg-node-icon{fill:none;stroke:var(--dg-tone);stroke-width:1.6;stroke-linecap:round;stroke-linejoin:round}
.dg-node--vm .dg-node-box{stroke-dasharray:4 3}
.arch-node-title{fill:var(--dg-text);font-size:11px;font-weight:600;font-family:ui-monospace,SFMono-Regular,Menlo,Consolas,monospace}
.arch-node-meta{fill:var(--dg-muted);font-size:7px;letter-spacing:.04em;font-family:ui-monospace,SFMono-Regular,Menlo,Consolas,monospace}
.arch-nic{fill:rgba(127,145,130,.10);stroke:var(--dg-tone);stroke-width:.8;stroke-opacity:.55}
.arch-nic-label{fill:var(--dg-tone);font-size:6.5px;font-weight:650;letter-spacing:.04em;font-family:ui-monospace,SFMono-Regular,Menlo,Consolas,monospace}
.arch-port{fill:var(--dg-bg);stroke:#2c3a30;stroke-width:.7}
.arch-port.is-used{fill:var(--dg-tone);stroke:var(--dg-tone)}
.arch-port-label{fill:var(--dg-muted);font-size:4.8px;font-family:ui-monospace,SFMono-Regular,Menlo,Consolas,monospace}
.arch-port-label.is-used{fill:var(--dg-bg);font-weight:700}
.network-rail{fill:none;stroke:var(--dg-tone);stroke-width:3;stroke-linecap:round}
.network-stub{fill:none;stroke:var(--dg-tone);stroke-width:1.4;stroke-opacity:.8}
.network-label{fill:var(--dg-tone);font-size:9px;font-weight:650;letter-spacing:.12em;text-transform:uppercase;font-family:ui-monospace,SFMono-Regular,Menlo,Consolas,monospace}
.network-legend-swatch{stroke:var(--dg-tone);stroke-width:3;stroke-linecap:round}
.network-legend-label{fill:var(--dg-tone);font-size:9px;font-weight:650;letter-spacing:.1em;text-transform:uppercase;font-family:ui-monospace,SFMono-Regular,Menlo,Consolas,monospace}
.dg-bond-mark{fill:color-mix(in srgb,var(--dg-tone) 7%,transparent);stroke:var(--dg-tone);stroke-width:1.2;stroke-dasharray:3 3}
.dg-bond-label{fill:var(--dg-tone);font-size:7px;font-weight:650;letter-spacing:.04em;font-family:ui-monospace,SFMono-Regular,Menlo,Consolas,monospace}
.dg-edge{fill:none;stroke:var(--dg-tone);stroke-width:1.4;stroke-linecap:round;stroke-linejoin:round}
.dg-edge--draw{stroke-dasharray:1;stroke-dashoffset:1;animation:dg-draw var(--dg-draw-duration,1200ms) ease forwards;animation-delay:var(--dg-delay,0ms)}
.dg-style-dashed{stroke-dasharray:.025 .018}
.dg-style-dotted{stroke-dasharray:.002 .016;stroke-linecap:round}
.dg-edge--flow{stroke-dasharray:none}
.dg-edge--pulse{animation:dg-pulse var(--dg-pulse-duration,2000ms) ease-in-out infinite;animation-delay:var(--dg-delay,0ms)}
.dg-arrow{fill:var(--dg-tone)}
.dg-edge-label{fill:var(--dg-muted);font-size:11.5px;font-weight:500;font-family:ui-monospace,SFMono-Regular,Menlo,Consolas,monospace;paint-order:stroke;stroke:var(--dg-label-halo);stroke-width:5px;stroke-linejoin:round}
@keyframes dg-draw{to{stroke-dashoffset:0}}
@keyframes dg-pulse{0%,100%{stroke-opacity:1}50%{stroke-opacity:.3}}
@media (prefers-reduced-motion:reduce){.dg-edge--draw,.dg-edge--pulse{animation:none;stroke-dashoffset:0}}
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_text_and_attributes() {
        assert_eq!(escape_xml("a & <b>"), "a &amp; &lt;b&gt;");
        assert_eq!(escape_attr("\"x\" 'y'"), "&quot;x&quot; &#39;y&#39;");
    }
}

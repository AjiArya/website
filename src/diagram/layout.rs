use std::collections::{HashMap, HashSet, VecDeque};

use anyhow::Result;
use unicode_width::UnicodeWidthStr;

use crate::diagram::model::{
    AnimationDirection, Arrow, Diagram, DiagramStyle, Direction, EdgeAnimation, EdgeStyle,
    LayoutMode, LbAlgorithm, NodeKind, Route, Shape,
};
use crate::diagram::router;

type ArchitectureGroup = ((String, String), Vec<usize>);

const BOND_TONES: [&str; 8] = [
    "blue", "amber", "violet", "green", "cyan", "pink", "red", "neutral",
];

#[derive(Debug, Clone, Copy, Default)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl Rect {
    pub fn right(self) -> f64 {
        self.x + self.w
    }

    pub fn bottom(self) -> f64 {
        self.y + self.h
    }

    pub fn center(self) -> Point {
        Point {
            x: self.x + self.w / 2.0,
            y: self.y + self.h / 2.0,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Side {
    Left,
    Right,
    Top,
    Bottom,
}

impl Side {
    pub fn is_horizontal(self) -> bool {
        matches!(self, Self::Left | Self::Right)
    }
}

#[derive(Debug, Clone)]
pub struct LayoutNode {
    pub id: String,
    pub lines: Vec<String>,
    pub icon: Option<String>,
    pub tone: String,
    pub shape: Shape,
    pub rect: Rect,
    pub group: Option<String>,
    pub load_balancer: bool,
    pub lb_algorithm: LbAlgorithm,
    pub arch: Option<LayoutArchitecture>,
}

#[derive(Debug, Clone)]
pub struct LayoutNic {
    pub id: String,
    pub tone: String,
    pub rect: Rect,
}

#[derive(Debug, Clone)]
pub struct LayoutPort {
    pub key: String,
    pub index: usize,
    pub rect: Rect,
    pub used: bool,
    pub tone: String,
}

#[derive(Debug, Clone)]
pub struct LayoutArchitecture {
    pub kind: NodeKind,
    pub ports: Vec<LayoutPort>,
    pub nics: Vec<LayoutNic>,
}

#[derive(Debug, Clone)]
pub struct LayoutBond {
    pub node: String,
    pub label: String,
    pub tone: String,
    pub x: f64,
    pub y: f64,
    pub rx: f64,
    pub ry: f64,
}

#[derive(Debug, Clone)]
pub struct LayoutNetwork {
    pub id: String,
    pub label: String,
    pub tone: String,
    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,
    pub horizontal: bool,
    pub members: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct LayoutGroup {
    pub id: String,
    pub label: String,
    pub tone: String,
    pub rect: Rect,
}

#[derive(Debug, Clone)]
pub struct LayoutEdge {
    pub from: String,
    pub to: String,
    pub path: String,
    pub route: Route,
    pub label: Option<String>,
    pub label_pos: Point,
    pub tone: String,
    pub style: EdgeStyle,
    pub animation: EdgeAnimation,
    pub direction: Option<AnimationDirection>,
    pub arrow: Arrow,
    pub delay_ms: u32,
}

#[derive(Debug, Clone)]
pub struct DiagramLayout {
    pub title: String,
    pub description: String,
    pub width: f64,
    pub height: f64,
    pub padding: f64,
    pub nodes: Vec<LayoutNode>,
    pub groups: Vec<LayoutGroup>,
    pub edges: Vec<LayoutEdge>,
    pub bonds: Vec<LayoutBond>,
    pub networks: Vec<LayoutNetwork>,
    pub no_drag: bool,
    pub fit_mobile: bool,
    pub prefix: String,
    pub draw_ms: u32,
    pub flow_ms: u32,
    pub stagger_ms: u32,
    pub direction: AnimationDirection,
    pub autoplay: bool,
}

pub fn layout(diagram: &Diagram, prefix: &str) -> Result<DiagramLayout> {
    diagram.validate()?;

    let architecture_detailed = diagram.style == DiagramStyle::ArchitectureDetailed;
    let architecture_simple = diagram.style == DiagramStyle::ArchitectureSimple;
    let mut nodes: Vec<LayoutNode> = diagram
        .nodes
        .iter()
        .map(|node| {
            let (lines, w, h, arch) = if architecture_detailed {
                architecture_geometry(node)
            } else {
                let (lines, w, h) = size_node(node);
                (lines, w, h, None)
            };
            LayoutNode {
                id: node.id.clone(),
                lines,
                icon: node.icon.clone(),
                tone: node.tone.clone().unwrap_or_else(|| "neutral".to_string()),
                shape: node.shape,
                rect: Rect {
                    x: 0.0,
                    y: 0.0,
                    w,
                    h,
                },
                group: node.group.clone(),
                load_balancer: node.load_balancer,
                lb_algorithm: node.lb_algorithm,
                arch,
            }
        })
        .collect();

    let mut groups = Vec::new();
    let mut networks = Vec::new();
    let (width, height);
    if architecture_simple {
        let result = simple_positions(diagram, &mut nodes);
        width = result.0;
        height = result.1;
        networks = result.2;
    } else {
        if architecture_detailed {
            auto_positions_architecture(diagram, &mut nodes);
        } else {
            match diagram.layout {
                LayoutMode::Manual => {
                    for (node, layout_node) in diagram.nodes.iter().zip(nodes.iter_mut()) {
                        let [x, y] = node.at.expect("validated manual layout");
                        layout_node.rect.x = x;
                        layout_node.rect.y = y;
                    }
                }
                LayoutMode::Auto => auto_positions(diagram, &mut nodes)?,
            }
        }

        groups = if architecture_detailed {
            Vec::new()
        } else {
            build_groups(diagram, &nodes)
        };
        shift_to_origin(&mut nodes, &mut groups, diagram.padding);
        if architecture_detailed {
            offset_architecture_rects(&mut nodes);
        }

        let size = canvas_size(&nodes, &groups, diagram.padding);
        width = size.0;
        height = size.1;
    }

    let (edges, bonds) = if architecture_detailed {
        route_architecture_edges(diagram, &mut nodes)
    } else if architecture_simple {
        (Vec::new(), Vec::new())
    } else {
        (route_edges(diagram, &nodes, &groups), Vec::new())
    };

    Ok(DiagramLayout {
        title: diagram.title.clone(),
        description: diagram
            .description
            .clone()
            .unwrap_or_else(|| format!("Architecture diagram: {}", diagram.title)),
        width,
        height,
        padding: diagram.padding,
        nodes,
        groups,
        edges,
        bonds,
        networks,
        no_drag: architecture_simple,
        fit_mobile: architecture_detailed || architecture_simple,
        prefix: prefix.to_string(),
        draw_ms: diagram.animation.draw_ms,
        flow_ms: diagram.animation.flow_ms,
        stagger_ms: diagram.animation.stagger_ms,
        direction: diagram.animation.direction,
        autoplay: diagram.animation.autoplay,
    })
}

fn size_node(node: &crate::diagram::model::Node) -> (Vec<String>, f64, f64) {
    let lines = wrap_label(&node.label, 26);
    let text_width = lines
        .iter()
        .map(|line| UnicodeWidthStr::width(line.as_str()) as f64 * 7.8)
        .fold(0.0_f64, f64::max);
    let icon_width = if node.icon.is_some() { 34.0 } else { 0.0 };
    let width = node
        .width
        .unwrap_or((text_width + icon_width + 30.0).clamp(120.0, 270.0));
    let height = node
        .height
        .unwrap_or((22.0 + lines.len() as f64 * 18.0).max(52.0));
    (lines, width, height)
}

fn simple_positions(diagram: &Diagram, nodes: &mut [LayoutNode]) -> (f64, f64, Vec<LayoutNetwork>) {
    let padding = diagram.padding;
    let node_gap = 22.0;
    let node_width: f64 = nodes.iter().map(|node| node.rect.w).sum::<f64>()
        + node_gap * nodes.len().saturating_sub(1) as f64;
    let legend_height = diagram.networks.len() as f64 * 16.0;
    let top_count = diagram
        .networks
        .iter()
        .enumerate()
        .filter(|(index, _)| index % 2 == 0)
        .count();
    let bottom_count = diagram.networks.len().saturating_sub(top_count);
    let top_start = padding + 22.0 + legend_height;
    let node_top = top_start + top_count as f64 * 24.0 + 30.0;

    let mut x = padding;
    for node in nodes.iter_mut() {
        node.rect.x = x;
        node.rect.y = node_top + 0.0;
        x += node.rect.w + node_gap;
    }
    let width = node_width + padding * 2.0;
    let node_bottom = nodes
        .iter()
        .map(|node| node.rect.bottom())
        .fold(node_top, f64::max);
    let height = node_bottom + 30.0 + bottom_count as f64 * 24.0 + padding;

    let by_id: HashMap<&str, Rect> = nodes
        .iter()
        .map(|node| (node.id.as_str(), node.rect))
        .collect();
    let mut networks = Vec::with_capacity(diagram.networks.len());
    for (index, network) in diagram.networks.iter().enumerate() {
        let top = index % 2 == 0;
        let lane = index / 2;
        let y = if top {
            top_start + lane as f64 * 24.0
        } else {
            node_bottom + 30.0 + lane as f64 * 24.0
        };
        let member_x: Vec<f64> = network
            .members
            .iter()
            .filter_map(|member| by_id.get(member.as_str()).map(|rect| rect.center().x))
            .collect();
        let x1 = member_x.first().copied().unwrap_or(padding);
        let x2 = member_x.last().copied().unwrap_or(width - padding);
        networks.push(LayoutNetwork {
            id: network.id.clone(),
            label: network.label.clone(),
            tone: network
                .tone
                .clone()
                .unwrap_or_else(|| "neutral".to_string()),
            x1,
            y1: y,
            x2,
            y2: y,
            horizontal: true,
            members: network.members.clone(),
        });
    }

    (width, height, networks)
}

fn architecture_geometry(
    node: &crate::diagram::model::Node,
) -> (Vec<String>, f64, f64, Option<LayoutArchitecture>) {
    let lines = wrap_label(&node.label, 26);
    let text_width = lines
        .iter()
        .map(|line| UnicodeWidthStr::width(line.as_str()) as f64 * 7.8)
        .fold(0.0_f64, f64::max);
    let icon_width = 24.0;

    match node.kind {
        NodeKind::Switch => {
            let pad = 10.0;
            let header = 34.0;
            let port_w = 7.5;
            let port_h = 9.0;
            let gap = 1.5;
            let ports = node.ports.unwrap_or(24);
            let columns = node
                .port_columns
                .unwrap_or(24)
                .clamp(1, 48)
                .min(ports.max(1));
            let rows = ports.div_ceil(columns).max(1);
            let grid_w = columns as f64 * port_w + (columns.saturating_sub(1)) as f64 * gap;
            let grid_h = rows as f64 * port_h + (rows.saturating_sub(1)) as f64 * gap;
            let width = (grid_w + pad * 2.0)
                .max(text_width + icon_width + 86.0)
                .max(180.0);
            let height = pad + header + grid_h + pad;
            let grid_x = (width - grid_w) / 2.0;
            let grid_y = pad + header;
            let tone = node.tone.clone().unwrap_or_else(|| "neutral".to_string());
            let mut port_cells = Vec::with_capacity(ports);
            for index in 1..=ports {
                let position = index - 1;
                let column = position % columns;
                let from_bottom = position / columns;
                let row = rows - 1 - from_bottom;
                port_cells.push(LayoutPort {
                    key: index.to_string(),
                    index,
                    rect: Rect {
                        x: grid_x + column as f64 * (port_w + gap),
                        y: grid_y + row as f64 * (port_h + gap),
                        w: port_w,
                        h: port_h,
                    },
                    used: false,
                    tone: tone.clone(),
                });
            }
            (
                lines,
                width,
                height,
                Some(LayoutArchitecture {
                    kind: NodeKind::Switch,
                    ports: port_cells,
                    nics: Vec::new(),
                }),
            )
        }
        NodeKind::Server | NodeKind::Vm if !node.nics.is_empty() => {
            let pad = 10.0;
            let block_gap = 8.0;
            let port_w = 10.0;
            let port_h = 12.0;
            let port_gap = 4.0;
            let nic_label_h = 11.0;
            let block_h = port_h + 5.0 + nic_label_h;
            let block_widths: Vec<f64> = node
                .nics
                .iter()
                .map(|nic| {
                    (nic.ports as f64 * port_w
                        + nic.ports.saturating_sub(1) as f64 * port_gap
                        + 12.0)
                        .max(46.0)
                })
                .collect();
            let nics_width = block_widths.iter().sum::<f64>()
                + block_gap * node.nics.len().saturating_sub(1) as f64;
            let width = (nics_width + pad * 2.0)
                .max(text_width + icon_width + 30.0)
                .max(150.0);
            let header_y = pad + block_h + 8.0;
            let height = header_y + (lines.len() as f64 * 16.0).max(22.0) + pad;
            let mut x = (width - nics_width) / 2.0;
            let mut nics = Vec::with_capacity(node.nics.len());
            let mut port_cells = Vec::new();
            for (nic, block_w) in node.nics.iter().zip(block_widths.iter()) {
                let tone = nic
                    .tone
                    .clone()
                    .or_else(|| node.tone.clone())
                    .unwrap_or_else(|| "neutral".to_string());
                nics.push(LayoutNic {
                    id: nic.id.clone(),
                    tone: tone.clone(),
                    rect: Rect {
                        x,
                        y: pad,
                        w: *block_w,
                        h: block_h,
                    },
                });
                let ports_w =
                    nic.ports as f64 * port_w + nic.ports.saturating_sub(1) as f64 * port_gap;
                let port_x = x + (*block_w - ports_w) / 2.0;
                for index in 1..=nic.ports {
                    port_cells.push(LayoutPort {
                        key: format!("{}/{}", nic.id, index),
                        index,
                        rect: Rect {
                            x: port_x + (index - 1) as f64 * (port_w + port_gap),
                            y: pad + 2.0,
                            w: port_w,
                            h: port_h,
                        },
                        used: false,
                        tone: tone.clone(),
                    });
                }
                x += *block_w + block_gap;
            }
            (
                lines,
                width,
                height,
                Some(LayoutArchitecture {
                    kind: node.kind,
                    ports: port_cells,
                    nics,
                }),
            )
        }
        _ => {
            let (lines, w, h) = size_node(node);
            (lines, w, h, None)
        }
    }
}

fn auto_positions_architecture(diagram: &Diagram, nodes: &mut [LayoutNode]) {
    let max_rank = diagram
        .nodes
        .iter()
        .map(|node| {
            node.rank.unwrap_or(match node.kind {
                NodeKind::Switch => 0,
                _ => 1,
            })
        })
        .max()
        .unwrap_or(0);
    let mut ranks: Vec<Vec<usize>> = vec![Vec::new(); max_rank + 1];
    for (index, node) in diagram.nodes.iter().enumerate() {
        let rank = node.rank.unwrap_or(match node.kind {
            NodeKind::Switch => 0,
            _ => 1,
        });
        ranks[rank].push(index);
    }

    let node_gap = match diagram.direction {
        Direction::Tb => 48.0,
        Direction::Lr => 56.0,
    };
    let rank_gap = match diagram.direction {
        Direction::Tb => 220.0,
        Direction::Lr => 180.0,
    };

    match diagram.direction {
        Direction::Tb => {
            let rank_widths: Vec<f64> = ranks
                .iter()
                .map(|rank| {
                    rank.iter().map(|&index| nodes[index].rect.w).sum::<f64>()
                        + node_gap * rank.len().saturating_sub(1) as f64
                })
                .collect();
            let max_width = rank_widths.iter().copied().fold(0.0_f64, f64::max);
            let mut y = 0.0;
            for (rank_index, rank) in ranks.iter().enumerate() {
                let rank_height = rank
                    .iter()
                    .map(|&index| nodes[index].rect.h)
                    .fold(0.0_f64, f64::max);
                let mut x = (max_width - rank_widths[rank_index]) / 2.0;
                for &index in rank {
                    nodes[index].rect.x = x;
                    nodes[index].rect.y = y + (rank_height - nodes[index].rect.h) / 2.0;
                    x += nodes[index].rect.w + node_gap;
                }
                y += rank_height + rank_gap;
            }
        }
        Direction::Lr => {
            let rank_heights: Vec<f64> = ranks
                .iter()
                .map(|rank| {
                    rank.iter().map(|&index| nodes[index].rect.h).sum::<f64>()
                        + node_gap * rank.len().saturating_sub(1) as f64
                })
                .collect();
            let max_height = rank_heights.iter().copied().fold(0.0_f64, f64::max);
            let mut x = 0.0;
            for (rank_index, rank) in ranks.iter().enumerate() {
                let rank_width = rank
                    .iter()
                    .map(|&index| nodes[index].rect.w)
                    .fold(0.0_f64, f64::max);
                let mut y = (max_height - rank_heights[rank_index]) / 2.0;
                for &index in rank {
                    nodes[index].rect.x = x + (rank_width - nodes[index].rect.w) / 2.0;
                    nodes[index].rect.y = y;
                    y += nodes[index].rect.h + node_gap;
                }
                x += rank_width + rank_gap;
            }
        }
    }
}

fn offset_architecture_rects(nodes: &mut [LayoutNode]) {
    for node in nodes {
        let (ox, oy) = (node.rect.x, node.rect.y);
        if let Some(arch) = &mut node.arch {
            for nic in &mut arch.nics {
                nic.rect.x += ox;
                nic.rect.y += oy;
            }
            for port in &mut arch.ports {
                port.rect.x += ox;
                port.rect.y += oy;
            }
        }
    }
}

fn wrap_label(label: &str, max_width: usize) -> Vec<String> {
    let mut output = Vec::new();

    for raw in label.split('\n') {
        let raw = raw.trim();
        if raw.is_empty() {
            continue;
        }
        if UnicodeWidthStr::width(raw) <= max_width {
            output.push(raw.to_string());
            continue;
        }

        let mut line = String::new();
        for word in raw.split_whitespace() {
            let extra = if line.is_empty() { 0 } else { 1 };
            let next_width =
                UnicodeWidthStr::width(line.as_str()) + extra + UnicodeWidthStr::width(word);
            if !line.is_empty() && next_width > max_width {
                output.push(std::mem::take(&mut line));
            }
            if !line.is_empty() {
                line.push(' ');
            }
            if UnicodeWidthStr::width(word) > max_width {
                // Hard-break a pathological single word rather than overflowing the node.
                for ch in word.chars() {
                    if UnicodeWidthStr::width(line.as_str()) >= max_width {
                        output.push(std::mem::take(&mut line));
                    }
                    line.push(ch);
                }
            } else {
                line.push_str(word);
            }
        }
        if !line.is_empty() {
            output.push(line);
        }
    }

    if output.is_empty() {
        output.push(String::new());
    }
    if output.len() > 3 {
        output.truncate(3);
        if let Some(last) = output.last_mut() {
            last.push('…');
        }
    }
    output
}

fn auto_positions(diagram: &Diagram, nodes: &mut [LayoutNode]) -> Result<()> {
    let n = nodes.len();
    let index: HashMap<&str, usize> = diagram
        .nodes
        .iter()
        .enumerate()
        .map(|(i, node)| (node.id.as_str(), i))
        .collect();

    let mut adjacency = vec![Vec::new(); n];
    let mut reverse = vec![Vec::new(); n];
    for edge in &diagram.edges {
        let from = index[edge.from.as_str()];
        let to = index[edge.to.as_str()];
        adjacency[from].push(to);
        reverse[to].push(from);
    }

    let back_edges = find_back_edges(&adjacency);
    let mut indegree = vec![0usize; n];
    let mut dag = vec![Vec::new(); n];
    for (from, targets) in adjacency.iter().enumerate() {
        for &to in targets {
            if !back_edges.contains(&(from, to)) {
                dag[from].push(to);
                indegree[to] += 1;
            }
        }
    }

    let mut rank = vec![0usize; n];
    let mut queue: VecDeque<usize> = (0..n).filter(|&i| indegree[i] == 0).collect();
    let mut visited = 0usize;
    while let Some(node) = queue.pop_front() {
        visited += 1;
        for &next in &dag[node] {
            rank[next] = rank[next].max(rank[node] + 1);
            indegree[next] -= 1;
            if indegree[next] == 0 {
                queue.push_back(next);
            }
        }
    }
    if visited != n {
        // A cycle that the DFS pass could not break should not make the build fail;
        // keep the nodes in a stable fallback rank instead.
        for value in &mut rank {
            if *value == 0 {
                *value = 0;
            }
        }
    }
    for (i, node) in diagram.nodes.iter().enumerate() {
        if let Some(explicit) = node.rank {
            rank[i] = explicit;
        }
    }

    let max_rank = rank.iter().copied().max().unwrap_or(0);
    let mut ranks: Vec<Vec<usize>> = vec![Vec::new(); max_rank + 1];
    for (i, value) in rank.iter().enumerate() {
        ranks[*value].push(i);
    }

    let group_position: HashMap<&str, usize> = diagram
        .groups
        .iter()
        .enumerate()
        .map(|(i, group)| (group.id.as_str(), i))
        .collect();
    let group_key = |node: usize| -> usize {
        diagram.nodes[node]
            .group
            .as_deref()
            .and_then(|g| group_position.get(g).copied())
            .unwrap_or(diagram.groups.len())
    };

    for rank_nodes in &mut ranks {
        rank_nodes.sort_by_key(|&node| (group_key(node), node));
    }

    let mut position = vec![0.0_f64; n];
    update_positions(&ranks, &mut position);

    for _ in 0..3 {
        for rank_index in 1..ranks.len() {
            let previous = position.clone();
            let mut scored: Vec<(usize, f64, usize)> = ranks[rank_index]
                .iter()
                .map(|&node| {
                    let values: Vec<f64> = reverse[node]
                        .iter()
                        .filter_map(|&neighbor| {
                            if rank[neighbor] < rank[node] {
                                Some(previous[neighbor])
                            } else {
                                None
                            }
                        })
                        .collect();
                    let barycenter = if values.is_empty() {
                        previous[node]
                    } else {
                        values.iter().sum::<f64>() / values.len() as f64
                    };
                    (node, barycenter, group_key(node))
                })
                .collect();
            scored.sort_by(|a, b| {
                a.2.cmp(&b.2)
                    .then_with(|| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
                    .then_with(|| a.0.cmp(&b.0))
            });
            ranks[rank_index] = scored.into_iter().map(|item| item.0).collect();
            update_positions(&ranks, &mut position);
        }

        for rank_index in (0..ranks.len().saturating_sub(1)).rev() {
            let next = position.clone();
            let mut scored: Vec<(usize, f64, usize)> = ranks[rank_index]
                .iter()
                .map(|&node| {
                    let values: Vec<f64> = dag[node]
                        .iter()
                        .filter_map(|&neighbor| {
                            if rank[neighbor] > rank[node] {
                                Some(next[neighbor])
                            } else {
                                None
                            }
                        })
                        .collect();
                    let barycenter = if values.is_empty() {
                        next[node]
                    } else {
                        values.iter().sum::<f64>() / values.len() as f64
                    };
                    (node, barycenter, group_key(node))
                })
                .collect();
            scored.sort_by(|a, b| {
                a.2.cmp(&b.2)
                    .then_with(|| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
                    .then_with(|| a.0.cmp(&b.0))
            });
            ranks[rank_index] = scored.into_iter().map(|item| item.0).collect();
            update_positions(&ranks, &mut position);
        }
    }

    let (rank_gap, node_gap) = match diagram.direction {
        Direction::Lr => (100.0, 36.0),
        Direction::Tb => (76.0, 44.0),
    };

    match diagram.direction {
        Direction::Lr => {
            let mut x = 0.0;
            for rank_nodes in &ranks {
                let rank_width = rank_nodes
                    .iter()
                    .map(|&node| nodes[node].rect.w)
                    .fold(0.0_f64, f64::max);
                let mut y = 0.0;
                for &node in rank_nodes {
                    nodes[node].rect.x = x + (rank_width - nodes[node].rect.w) / 2.0;
                    nodes[node].rect.y = y;
                    y += nodes[node].rect.h + node_gap;
                }
                x += rank_width + rank_gap;
            }
        }
        Direction::Tb => {
            let mut y = 0.0;
            for rank_nodes in &ranks {
                let rank_height = rank_nodes
                    .iter()
                    .map(|&node| nodes[node].rect.h)
                    .fold(0.0_f64, f64::max);
                let mut x = 0.0;
                for &node in rank_nodes {
                    nodes[node].rect.x = x;
                    nodes[node].rect.y = y + (rank_height - nodes[node].rect.h) / 2.0;
                    x += nodes[node].rect.w + node_gap;
                }
                y += rank_height + rank_gap;
            }
        }
    }

    Ok(())
}

fn update_positions(ranks: &[Vec<usize>], position: &mut [f64]) {
    for rank_nodes in ranks {
        for (i, &node) in rank_nodes.iter().enumerate() {
            position[node] = i as f64;
        }
    }
}

fn find_back_edges(adjacency: &[Vec<usize>]) -> HashSet<(usize, usize)> {
    let mut color = vec![0u8; adjacency.len()];
    let mut back = HashSet::new();
    for start in 0..adjacency.len() {
        if color[start] == 0 {
            dfs(start, adjacency, &mut color, &mut back);
        }
    }
    back
}

fn dfs(
    node: usize,
    adjacency: &[Vec<usize>],
    color: &mut [u8],
    back: &mut HashSet<(usize, usize)>,
) {
    color[node] = 1;
    for &next in &adjacency[node] {
        if color[next] == 0 {
            dfs(next, adjacency, color, back);
        } else if color[next] == 1 {
            back.insert((node, next));
        }
    }
    color[node] = 2;
}

fn build_groups(diagram: &Diagram, nodes: &[LayoutNode]) -> Vec<LayoutGroup> {
    let mut groups = Vec::new();

    for group in &diagram.groups {
        let members: Vec<&LayoutNode> = nodes
            .iter()
            .filter(|node| node.group.as_deref() == Some(group.id.as_str()))
            .collect();
        if members.is_empty() {
            continue;
        }
        let min_x = members
            .iter()
            .map(|node| node.rect.x)
            .fold(f64::INFINITY, f64::min);
        let min_y = members
            .iter()
            .map(|node| node.rect.y)
            .fold(f64::INFINITY, f64::min);
        let max_x = members
            .iter()
            .map(|node| node.rect.right())
            .fold(f64::NEG_INFINITY, f64::max);
        let max_y = members
            .iter()
            .map(|node| node.rect.bottom())
            .fold(f64::NEG_INFINITY, f64::max);
        let label_width = UnicodeWidthStr::width(group.label.as_str()) as f64 * 7.2 + 32.0;
        let width = (max_x - min_x + 32.0).max(label_width);
        groups.push(LayoutGroup {
            id: group.id.clone(),
            label: group.label.clone(),
            tone: group.tone.clone().unwrap_or_else(|| "neutral".to_string()),
            rect: Rect {
                x: min_x - 16.0,
                y: min_y - 42.0,
                w: width,
                h: max_y - min_y + 58.0,
            },
        });
    }

    groups
}

fn shift_to_origin(nodes: &mut [LayoutNode], groups: &mut [LayoutGroup], padding: f64) {
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    for node in nodes.iter() {
        min_x = min_x.min(node.rect.x);
        min_y = min_y.min(node.rect.y);
    }
    for group in groups.iter() {
        min_x = min_x.min(group.rect.x);
        min_y = min_y.min(group.rect.y);
    }
    if !min_x.is_finite() || !min_y.is_finite() {
        return;
    }
    let dx = padding - min_x;
    let dy = padding - min_y;
    for node in nodes.iter_mut() {
        node.rect.x += dx;
        node.rect.y += dy;
    }
    for group in groups.iter_mut() {
        group.rect.x += dx;
        group.rect.y += dy;
    }
}

fn canvas_size(nodes: &[LayoutNode], groups: &[LayoutGroup], padding: f64) -> (f64, f64) {
    let mut max_x: f64 = 320.0;
    let mut max_y: f64 = 180.0;
    for node in nodes {
        max_x = max_x.max(node.rect.right() + padding);
        max_y = max_y.max(node.rect.bottom() + padding);
    }
    for group in groups {
        max_x = max_x.max(group.rect.right() + padding);
        max_y = max_y.max(group.rect.bottom() + padding);
    }
    (max_x, max_y)
}

type EdgeEndpoint = (usize, bool, f64);
type EndpointMap = HashMap<(usize, Side), Vec<EdgeEndpoint>>;

fn route_edges(diagram: &Diagram, nodes: &[LayoutNode], groups: &[LayoutGroup]) -> Vec<LayoutEdge> {
    let by_id: HashMap<&str, &LayoutNode> =
        nodes.iter().map(|node| (node.id.as_str(), node)).collect();
    let node_index: HashMap<&str, usize> = nodes
        .iter()
        .enumerate()
        .map(|(index, node)| (node.id.as_str(), index))
        .collect();

    let node_rects: Vec<Rect> = nodes.iter().map(|node| node.rect).collect();
    let group_label_rects: Vec<Rect> = groups
        .iter()
        .map(|group| Rect {
            x: group.rect.x + 14.0,
            y: group.rect.y + 7.0,
            w: UnicodeWidthStr::width(group.label.as_str()) as f64 * 7.0 + 8.0,
            h: 16.0,
        })
        .collect();
    let bounds = union_bounds(
        node_rects
            .iter()
            .copied()
            .chain(groups.iter().map(|group| group.rect)),
        24.0,
    );

    // Assign edges to distinct points along the side of each node so parallel
    // edges do not leave from exactly the same coordinate.
    let mut sides = Vec::with_capacity(diagram.edges.len());
    let mut endpoints: EndpointMap = HashMap::new();
    for (index, edge) in diagram.edges.iter().enumerate() {
        let from_index = node_index[edge.from.as_str()];
        let to_index = node_index[edge.to.as_str()];
        let from = nodes[from_index].rect;
        let to = nodes[to_index].rect;
        let start_side = choose_side(from, to.center(), diagram.direction);
        let end_side = choose_side(to, from.center(), diagram.direction);
        sides.push((start_side, end_side));

        let start_other = if start_side.is_horizontal() {
            to.center().y
        } else {
            to.center().x
        };
        let end_other = if end_side.is_horizontal() {
            from.center().y
        } else {
            from.center().x
        };
        endpoints
            .entry((from_index, start_side))
            .or_default()
            .push((index, true, start_other));
        endpoints
            .entry((to_index, end_side))
            .or_default()
            .push((index, false, end_other));
    }

    let mut start_ports = vec![Point::default(); diagram.edges.len()];
    let mut end_ports = vec![Point::default(); diagram.edges.len()];
    let mut start_lanes = vec![0.0_f64; diagram.edges.len()];
    let mut end_lanes = vec![0.0_f64; diagram.edges.len()];
    for ((node_index, side), mut list) in endpoints {
        list.sort_by(|a, b| {
            a.2.partial_cmp(&b.2)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.0.cmp(&b.0))
        });
        let rect = nodes[node_index].rect;
        let count = list.len();
        for (position, (edge_index, is_start, _)) in list.iter().enumerate() {
            let fraction = if count == 1 {
                0.5
            } else {
                (position + 1) as f64 / (count + 1) as f64
            };
            let point = side_point(rect, side, fraction);
            let lane = if count == 1 {
                0.0
            } else {
                (position as f64 - (count.saturating_sub(1) as f64 / 2.0)) * 12.0
            };
            if *is_start {
                start_ports[*edge_index] = point;
                start_lanes[*edge_index] = lane;
            } else {
                end_ports[*edge_index] = point;
                end_lanes[*edge_index] = lane;
            }
        }
    }

    let mut result = Vec::with_capacity(diagram.edges.len());
    for (index, edge) in diagram.edges.iter().enumerate() {
        let from_index = node_index[edge.from.as_str()];
        let to_index = node_index[edge.to.as_str()];
        let from = nodes[from_index].rect;
        let to = nodes[to_index].rect;

        let mut simple_obstacles = Vec::with_capacity(node_rects.len() + group_label_rects.len());
        for (node_index, rect) in node_rects.iter().enumerate() {
            if node_index != from_index && node_index != to_index {
                simple_obstacles.push(*rect);
            }
        }
        simple_obstacles.extend_from_slice(&group_label_rects);

        let mut router_obstacles = Vec::with_capacity(node_rects.len() + group_label_rects.len());
        router_obstacles.extend_from_slice(&node_rects);
        router_obstacles.extend_from_slice(&group_label_rects);

        let lane = if start_lanes[index].abs() >= end_lanes[index].abs() {
            start_lanes[index]
        } else {
            end_lanes[index]
        };
        let simple = simple_elbow_points(
            start_ports[index],
            sides[index].0,
            end_ports[index],
            sides[index].1,
            lane,
        );

        let routed = if edge.route == Route::Elbow {
            if path_clear(&simple, &simple_obstacles, 2.0) {
                router::path_from_points(&simple)
            } else {
                router::route(
                    start_ports[index],
                    sides[index].0,
                    end_ports[index],
                    sides[index].1,
                    &router_obstacles,
                    bounds,
                )
                .unwrap_or_else(|| route(from, to, edge.route))
            }
        } else {
            route(from, to, edge.route)
        };

        let (path, label_pos) = routed;
        let tone = edge
            .tone
            .clone()
            .unwrap_or_else(|| by_id[edge.from.as_str()].tone.clone());
        let animation = diagram.animation.for_edge(edge);
        result.push(LayoutEdge {
            from: edge.from.clone(),
            to: edge.to.clone(),
            path,
            route: edge.route,
            label: edge.label.clone(),
            label_pos,
            tone,
            style: edge.style,
            animation,
            direction: edge.direction,
            arrow: edge.arrow,
            delay_ms: index as u32 * diagram.animation.stagger_ms,
        });
    }

    result
}

fn architecture_port_key(
    node: &LayoutNode,
    nic: Option<&str>,
    port: Option<usize>,
) -> Option<String> {
    let port = port?;
    match nic {
        Some(nic) => Some(format!("{nic}/{port}")),
        None => {
            if node
                .arch
                .as_ref()
                .is_some_and(|arch| arch.kind == NodeKind::Switch)
            {
                Some(port.to_string())
            } else {
                None
            }
        }
    }
}

fn architecture_port_index(node: &LayoutNode, key: &str) -> Option<usize> {
    node.arch
        .as_ref()?
        .ports
        .iter()
        .position(|port| port.key == key)
}

fn route_architecture_edges(
    diagram: &Diagram,
    nodes: &mut [LayoutNode],
) -> (Vec<LayoutEdge>, Vec<LayoutBond>) {
    let node_index: HashMap<&str, usize> = diagram
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| (node.id.as_str(), index))
        .collect();

    let mut bond_names: Vec<&str> = diagram
        .edges
        .iter()
        .filter_map(|edge| edge.bond.as_deref())
        .collect();
    bond_names.sort_unstable();
    bond_names.dedup();
    let bond_tones: HashMap<&str, &str> = bond_names
        .iter()
        .enumerate()
        .map(|(index, bond)| (*bond, BOND_TONES[index % BOND_TONES.len()]))
        .collect();

    let mut starts: Vec<Option<Point>> = vec![None; diagram.edges.len()];
    let mut ends: Vec<Option<Point>> = vec![None; diagram.edges.len()];
    let mut tones: Vec<String> = Vec::with_capacity(diagram.edges.len());

    for (index, edge) in diagram.edges.iter().enumerate() {
        let from_index = node_index[edge.from.as_str()];
        let to_index = node_index[edge.to.as_str()];

        let from_key =
            architecture_port_key(&nodes[from_index], edge.from_nic.as_deref(), edge.from_port);
        let to_key = architecture_port_key(&nodes[to_index], edge.to_nic.as_deref(), edge.to_port);

        let from_port_index = from_key
            .as_deref()
            .and_then(|key| architecture_port_index(&nodes[from_index], key));
        let to_port_index = to_key
            .as_deref()
            .and_then(|key| architecture_port_index(&nodes[to_index], key));

        let tone = edge
            .bond
            .as_deref()
            .and_then(|bond| bond_tones.get(bond).copied())
            .map(str::to_string)
            .or_else(|| edge.tone.clone())
            .unwrap_or_else(|| nodes[from_index].tone.clone());

        if let Some(port_index) = from_port_index {
            let center = nodes[from_index].arch.as_ref().unwrap().ports[port_index]
                .rect
                .center();
            starts[index] = Some(center);
            let port = &mut nodes[from_index].arch.as_mut().unwrap().ports[port_index];
            port.used = true;
            if edge.bond.is_some() {
                port.tone = tone.clone();
            }
        }
        if let Some(port_index) = to_port_index {
            let center = nodes[to_index].arch.as_ref().unwrap().ports[port_index]
                .rect
                .center();
            ends[index] = Some(center);
            let port = &mut nodes[to_index].arch.as_mut().unwrap().ports[port_index];
            port.used = true;
            port.tone = tone.clone();
        }
        tones.push(tone);
    }

    let lane_edges: Vec<usize> = (0..diagram.edges.len())
        .filter(|&index| starts[index].is_some() && ends[index].is_some())
        .collect();

    let min_y = lane_edges
        .iter()
        .map(|&index| starts[index].unwrap().y.min(ends[index].unwrap().y))
        .fold(f64::INFINITY, f64::min);
    let max_y = lane_edges
        .iter()
        .map(|&index| starts[index].unwrap().y.max(ends[index].unwrap().y))
        .fold(f64::NEG_INFINITY, f64::max);
    let lane_top = min_y + 30.0;
    let lane_bottom = (max_y - 30.0).max(lane_top);

    let mut groups: HashMap<(String, String), Vec<usize>> = HashMap::new();
    for index in lane_edges {
        let key = (
            diagram.edges[index].from.clone(),
            diagram.edges[index].to.clone(),
        );
        groups.entry(key).or_default().push(index);
    }
    let mut group_list: Vec<((String, String), Vec<usize>)> = groups.into_iter().collect();
    for (_, members) in &mut group_list {
        members.sort_by(|&a, &b| {
            starts[a]
                .unwrap()
                .x
                .partial_cmp(&starts[b].unwrap().x)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }
    let target_order: HashMap<&str, usize> = diagram
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| (node.id.as_str(), index))
        .collect();
    group_list.sort_by(|(a, a_members), (b, b_members)| {
        let a_target = target_order.get(a.1.as_str()).copied().unwrap_or(0);
        let b_target = target_order.get(b.1.as_str()).copied().unwrap_or(0);
        let a_start = a_members
            .iter()
            .map(|&i| starts[i].unwrap().x)
            .fold(f64::INFINITY, f64::min);
        let b_start = b_members
            .iter()
            .map(|&i| starts[i].unwrap().x)
            .fold(f64::INFINITY, f64::min);
        a_target.cmp(&b_target).then_with(|| {
            a_start
                .partial_cmp(&b_start)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    });

    let mut lane_y: Vec<f64> = (0..diagram.edges.len())
        .map(|_| (min_y + max_y) / 2.0)
        .collect();

    let mut by_target: HashMap<String, Vec<ArchitectureGroup>> = HashMap::new();
    for group in group_list {
        by_target.entry(group.0.1.clone()).or_default().push(group);
    }
    let mut targets: Vec<String> = by_target.keys().cloned().collect();
    targets.sort_by_key(|target| target_order.get(target.as_str()).copied().unwrap_or(0));
    let target_count = targets.len();
    let band_gap = 16.0;
    let band_height = if target_count > 0 {
        ((lane_bottom - lane_top) - band_gap * target_count.saturating_sub(1) as f64)
            / target_count as f64
    } else {
        0.0
    }
    .max(1.0);

    for (target_position, target) in targets.iter().enumerate() {
        let groups = by_target.remove(target).unwrap_or_default();
        let band_top = lane_top + target_position as f64 * (band_height + band_gap);
        let group_count = groups.len();
        for (order, (_, members)) in groups.iter().enumerate() {
            let center = if group_count <= 1 {
                band_top + band_height / 2.0
            } else {
                band_top + band_height * (order + 1) as f64 / (group_count + 1) as f64
            };
            let member_count = members.len();
            for (position, &index) in members.iter().enumerate() {
                let offset = (position as f64 - member_count.saturating_sub(1) as f64 / 2.0) * 3.0;
                lane_y[index] = center + offset;
            }
        }
    }

    let mut bond_groups: HashMap<(String, String), Vec<usize>> = HashMap::new();
    for (index, edge) in diagram.edges.iter().enumerate() {
        if let Some(bond) = &edge.bond {
            if starts[index].is_some() && ends[index].is_some() {
                bond_groups
                    .entry((edge.from.clone(), bond.clone()))
                    .or_default()
                    .push(index);
            }
        }
    }
    let mut bonds = Vec::with_capacity(bond_groups.len());
    for ((from, bond), members) in bond_groups {
        let from_index = node_index[from.as_str()];
        let host = nodes[from_index].rect;
        let source_x = members
            .iter()
            .map(|&index| starts[index].unwrap().x)
            .sum::<f64>()
            / members.len() as f64;
        let tone = bond_tones
            .get(bond.as_str())
            .copied()
            .unwrap_or("neutral")
            .to_string();
        bonds.push(LayoutBond {
            node: from,
            label: bond,
            tone,
            x: source_x,
            y: host.bottom() + 18.0,
            rx: 22.0,
            ry: 10.0,
        });
    }

    bonds.sort_by(|a, b| a.node.cmp(&b.node).then_with(|| a.label.cmp(&b.label)));
    let mut start = 0;
    while start < bonds.len() {
        let mut end = start + 1;
        while end < bonds.len() && bonds[end].node == bonds[start].node {
            end += 1;
        }
        let count = end - start;
        for (position, bond) in bonds[start..end].iter_mut().enumerate() {
            let offset = (position as f64 - (count.saturating_sub(1) as f64 / 2.0)) * 48.0;
            bond.x += offset;
        }
        start = end;
    }

    let mut result = Vec::with_capacity(diagram.edges.len());
    for (index, edge) in diagram.edges.iter().enumerate() {
        let from_index = node_index[edge.from.as_str()];
        let to_index = node_index[edge.to.as_str()];
        let start = starts[index].unwrap_or_else(|| nodes[from_index].rect.center());
        let end = ends[index].unwrap_or_else(|| nodes[to_index].rect.center());
        let lane = lane_y[index];

        let mut points = vec![start];
        if (start.x - end.x).abs() > 0.1 {
            points.push(Point {
                x: start.x,
                y: lane,
            });
            points.push(Point { x: end.x, y: lane });
        }
        points.push(end);
        let (path, _) = router::path_from_points(&points);

        result.push(LayoutEdge {
            from: edge.from.clone(),
            to: edge.to.clone(),
            path,
            route: Route::Elbow,
            label: edge.label.clone(),
            label_pos: Point {
                x: (start.x + end.x) / 2.0,
                y: lane,
            },
            tone: tones[index].clone(),
            style: EdgeStyle::Solid,
            animation: EdgeAnimation::None,
            direction: None,
            arrow: Arrow::None,
            delay_ms: 0,
        });
    }

    (result, bonds)
}

fn simple_elbow_points(
    start: Point,
    start_side: Side,
    end: Point,
    end_side: Side,
    lane: f64,
) -> Vec<Point> {
    if start_side.is_horizontal() && end_side.is_horizontal() {
        let mid_x = (start.x + end.x) / 2.0 + lane;
        vec![
            start,
            Point {
                x: mid_x,
                y: start.y,
            },
            Point { x: mid_x, y: end.y },
            end,
        ]
    } else if !start_side.is_horizontal() && !end_side.is_horizontal() {
        let mid_y = (start.y + end.y) / 2.0 + lane;
        vec![
            start,
            Point {
                x: start.x,
                y: mid_y,
            },
            Point { x: end.x, y: mid_y },
            end,
        ]
    } else if start_side.is_horizontal() {
        vec![
            start,
            Point {
                x: end.x,
                y: start.y,
            },
            end,
        ]
    } else {
        vec![
            start,
            Point {
                x: start.x,
                y: end.y,
            },
            end,
        ]
    }
}

fn path_clear(points: &[Point], obstacles: &[Rect], clearance: f64) -> bool {
    for pair in points.windows(2) {
        let a = pair[0];
        let b = pair[1];
        for obstacle in obstacles {
            let rect = Rect {
                x: obstacle.x - clearance,
                y: obstacle.y - clearance,
                w: obstacle.w + clearance * 2.0,
                h: obstacle.h + clearance * 2.0,
            };
            let horizontal = (a.y - b.y).abs() < 0.1;
            let vertical = (a.x - b.x).abs() < 0.1;
            if horizontal {
                let y = a.y;
                let x0 = a.x.min(b.x);
                let x1 = a.x.max(b.x);
                if y >= rect.y && y <= rect.bottom() && x1 >= rect.x && x0 <= rect.right() {
                    return false;
                }
            } else if vertical {
                let x = a.x;
                let y0 = a.y.min(b.y);
                let y1 = a.y.max(b.y);
                if x >= rect.x && x <= rect.right() && y1 >= rect.y && y0 <= rect.bottom() {
                    return false;
                }
            } else {
                let x0 = a.x.min(b.x);
                let x1 = a.x.max(b.x);
                let y0 = a.y.min(b.y);
                let y1 = a.y.max(b.y);
                if x1 >= rect.x && x0 <= rect.right() && y1 >= rect.y && y0 <= rect.bottom() {
                    return false;
                }
            }
        }
    }
    true
}

fn choose_side(rect: Rect, toward: Point, direction: Direction) -> Side {
    let origin = rect.center();
    let dx = toward.x - origin.x;
    let dy = toward.y - origin.y;

    match direction {
        Direction::Tb => {
            if dy.abs() >= dx.abs() * 0.25 {
                if dy >= 0.0 { Side::Bottom } else { Side::Top }
            } else if dx >= 0.0 {
                Side::Right
            } else {
                Side::Left
            }
        }
        Direction::Lr => {
            if dx.abs() >= dy.abs() * 0.25 {
                if dx >= 0.0 { Side::Right } else { Side::Left }
            } else if dy >= 0.0 {
                Side::Bottom
            } else {
                Side::Top
            }
        }
    }
}

fn side_point(rect: Rect, side: Side, fraction: f64) -> Point {
    let inner = match side {
        Side::Left | Side::Right => (rect.h - 20.0).max(1.0),
        Side::Top | Side::Bottom => (rect.w - 20.0).max(1.0),
    };
    let offset = 10.0 + fraction * inner;
    match side {
        Side::Left => Point {
            x: rect.x,
            y: rect.y + offset,
        },
        Side::Right => Point {
            x: rect.right(),
            y: rect.y + offset,
        },
        Side::Top => Point {
            x: rect.x + offset,
            y: rect.y,
        },
        Side::Bottom => Point {
            x: rect.x + offset,
            y: rect.bottom(),
        },
    }
}

fn union_bounds(rects: impl Iterator<Item = Rect>, padding: f64) -> Rect {
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for rect in rects {
        min_x = min_x.min(rect.x);
        min_y = min_y.min(rect.y);
        max_x = max_x.max(rect.right());
        max_y = max_y.max(rect.bottom());
    }
    if !min_x.is_finite() {
        return Rect {
            x: 0.0,
            y: 0.0,
            w: 320.0,
            h: 200.0,
        };
    }
    Rect {
        x: min_x - padding,
        y: min_y - padding,
        w: max_x - min_x + padding * 2.0,
        h: max_y - min_y + padding * 2.0,
    }
}

fn route(from: Rect, to: Rect, route: Route) -> (String, Point) {
    let (start, start_side) = anchor(from, to.center());
    let (end, end_side) = anchor(to, from.center());
    let label_pos = Point {
        x: (start.x + end.x) / 2.0,
        y: (start.y + end.y) / 2.0,
    };

    let path = match route {
        Route::Straight => router::path_from_points(&[start, end]).0,
        Route::Curve => {
            let dx = end.x - start.x;
            let dy = end.y - start.y;
            let (c1, c2) = match (start_side.is_horizontal(), end_side.is_horizontal()) {
                (true, true) => (
                    Point {
                        x: start.x + dx * 0.5,
                        y: start.y,
                    },
                    Point {
                        x: end.x - dx * 0.5,
                        y: end.y,
                    },
                ),
                (false, false) => (
                    Point {
                        x: start.x,
                        y: start.y + dy * 0.5,
                    },
                    Point {
                        x: end.x,
                        y: end.y - dy * 0.5,
                    },
                ),
                (true, false) => (
                    Point {
                        x: start.x + dx * 0.5,
                        y: start.y,
                    },
                    Point {
                        x: end.x,
                        y: end.y - dy * 0.5,
                    },
                ),
                (false, true) => (
                    Point {
                        x: start.x,
                        y: start.y + dy * 0.5,
                    },
                    Point {
                        x: end.x - dx * 0.5,
                        y: end.y,
                    },
                ),
            };
            format!(
                "M {:.1} {:.1} C {:.1} {:.1}, {:.1} {:.1}, {:.1} {:.1}",
                start.x, start.y, c1.x, c1.y, c2.x, c2.y, end.x, end.y
            )
        }
        Route::Elbow => {
            router::path_from_points(&simple_elbow_points(start, start_side, end, end_side, 0.0)).0
        }
    };

    (path, label_pos)
}

fn anchor(rect: Rect, toward: Point) -> (Point, Side) {
    let center = rect.center();
    let dx = toward.x - center.x;
    let dy = toward.y - center.y;
    if dx.abs() >= dy.abs() {
        if dx >= 0.0 {
            (
                Point {
                    x: rect.right(),
                    y: center.y,
                },
                Side::Right,
            )
        } else {
            (
                Point {
                    x: rect.x,
                    y: center.y,
                },
                Side::Left,
            )
        }
    } else if dy >= 0.0 {
        (
            Point {
                x: center.x,
                y: rect.bottom(),
            },
            Side::Bottom,
        )
    } else {
        (
            Point {
                x: center.x,
                y: rect.y,
            },
            Side::Top,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wraps_long_labels() {
        let lines = wrap_label("a fairly long node label that needs wrapping", 16);
        assert!(lines.len() >= 2);
        assert!(lines.len() <= 3);
    }

    #[test]
    fn routes_elbow_with_four_points() {
        let from = Rect {
            x: 0.0,
            y: 10.0,
            w: 100.0,
            h: 40.0,
        };
        let to = Rect {
            x: 240.0,
            y: 120.0,
            w: 100.0,
            h: 40.0,
        };
        let (path, _) = route(from, to, Route::Elbow);
        assert!(path.starts_with('M'));
        assert_eq!(path.matches(" L ").count(), 3);
    }
}

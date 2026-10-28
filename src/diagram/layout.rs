use std::collections::{HashMap, HashSet, VecDeque};

use anyhow::Result;
use unicode_width::UnicodeWidthStr;

use crate::diagram::model::{
    AnimationDirection, Arrow, Diagram, Direction, EdgeAnimation, EdgeStyle, LayoutMode,
    LbAlgorithm, Route, Shape,
};
use crate::diagram::router;

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
    pub nodes: Vec<LayoutNode>,
    pub groups: Vec<LayoutGroup>,
    pub edges: Vec<LayoutEdge>,
    pub prefix: String,
    pub draw_ms: u32,
    pub flow_ms: u32,
    pub stagger_ms: u32,
    pub direction: AnimationDirection,
    pub autoplay: bool,
}

pub fn layout(diagram: &Diagram, prefix: &str) -> Result<DiagramLayout> {
    diagram.validate()?;

    let mut nodes: Vec<LayoutNode> = diagram
        .nodes
        .iter()
        .map(|node| {
            let (lines, w, h) = size_node(node);
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
            }
        })
        .collect();

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

    let mut groups = build_groups(diagram, &nodes);
    shift_to_origin(&mut nodes, &mut groups, diagram.padding);

    let (width, height) = canvas_size(&nodes, &groups, diagram.padding);
    let edges = route_edges(diagram, &nodes, &groups);

    Ok(DiagramLayout {
        title: diagram.title.clone(),
        description: diagram
            .description
            .clone()
            .unwrap_or_else(|| format!("Architecture diagram: {}", diagram.title)),
        width,
        height,
        nodes,
        groups,
        edges,
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
        Route::Straight => format!(
            "M {:.1} {:.1} L {:.1} {:.1}",
            start.x, start.y, end.x, end.y
        ),
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
            let points = if start_side.is_horizontal() && end_side.is_horizontal() {
                let mid = (start.x + end.x) / 2.0;
                vec![
                    start,
                    Point { x: mid, y: start.y },
                    Point { x: mid, y: end.y },
                    end,
                ]
            } else if !start_side.is_horizontal() && !end_side.is_horizontal() {
                let mid = (start.y + end.y) / 2.0;
                vec![
                    start,
                    Point { x: start.x, y: mid },
                    Point { x: end.x, y: mid },
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
            };
            let mut path = format!("M {:.1} {:.1}", points[0].x, points[0].y);
            for point in &points[1..] {
                path.push_str(&format!(" L {:.1} {:.1}", point.x, point.y));
            }
            path
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

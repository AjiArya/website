use anyhow::{Result, bail};
use serde::Deserialize;

use crate::diagram::icons;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Diagram {
    #[serde(default = "default_version")]
    pub version: u32,
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub direction: Direction,
    #[serde(default)]
    pub layout: LayoutMode,
    #[serde(default = "default_padding")]
    pub padding: f64,
    #[serde(default)]
    pub nodes: Vec<Node>,
    #[serde(default)]
    pub groups: Vec<Group>,
    #[serde(default)]
    pub edges: Vec<Edge>,
    #[serde(default)]
    pub animation: Animation,
    #[serde(default)]
    pub style: DiagramStyle,
}

#[derive(Debug, Clone, Copy, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DiagramStyle {
    #[default]
    Detailed,
    Architecture,
}

#[derive(Debug, Clone, Copy, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum NodeKind {
    #[default]
    Generic,
    Switch,
    Server,
    Vm,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Nic {
    pub id: String,
    #[serde(default = "default_nic_ports")]
    pub ports: usize,
    #[serde(default)]
    pub tone: Option<String>,
}

#[derive(Debug, Clone, Copy, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum Direction {
    #[default]
    Lr,
    Tb,
}

#[derive(Debug, Clone, Copy, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LayoutMode {
    #[default]
    Auto,
    Manual,
}

#[derive(Debug, Clone, Copy, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Shape {
    #[default]
    Box,
    Pill,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Node {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub tone: Option<String>,
    #[serde(default)]
    pub shape: Shape,
    #[serde(default)]
    pub group: Option<String>,
    #[serde(default)]
    pub at: Option<[f64; 2]>,
    #[serde(default)]
    pub rank: Option<usize>,
    #[serde(default)]
    pub width: Option<f64>,
    #[serde(default)]
    pub height: Option<f64>,
    #[serde(default)]
    pub load_balancer: bool,
    #[serde(default)]
    pub lb_algorithm: LbAlgorithm,
    #[serde(default)]
    pub kind: NodeKind,
    #[serde(default)]
    pub ports: Option<usize>,
    #[serde(default)]
    pub port_columns: Option<usize>,
    #[serde(default)]
    pub nics: Vec<Nic>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Group {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub tone: Option<String>,
}

#[derive(Debug, Clone, Copy, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum EdgeStyle {
    #[default]
    Solid,
    Dashed,
    Dotted,
}

#[derive(Debug, Clone, Copy, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Arrow {
    #[default]
    End,
    Both,
    None,
}

#[derive(Debug, Clone, Copy, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Route {
    #[default]
    Elbow,
    Straight,
    Curve,
}

#[derive(Debug, Clone, Copy, Deserialize, Default, PartialEq, Eq)]
pub enum EdgeAnimation {
    #[default]
    #[serde(rename = "inherit")]
    Inherit,
    #[serde(rename = "none")]
    None,
    #[serde(rename = "draw")]
    Draw,
    #[serde(rename = "flow")]
    Flow,
    #[serde(rename = "pulse")]
    Pulse,
}

#[derive(Debug, Clone, Copy, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum LbAlgorithm {
    #[default]
    RoundRobin,
    Random,
    LeastConnections,
}

impl LbAlgorithm {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::RoundRobin => "round-robin",
            Self::Random => "random",
            Self::LeastConnections => "least-connections",
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AnimationDirection {
    Forward,
    Reverse,
    #[default]
    Alternate,
}

impl AnimationDirection {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Forward => "forward",
            Self::Reverse => "reverse",
            Self::Alternate => "alternate",
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Edge {
    pub from: String,
    pub to: String,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub style: EdgeStyle,
    #[serde(default)]
    pub arrow: Arrow,
    #[serde(default)]
    pub route: Route,
    #[serde(default)]
    pub tone: Option<String>,
    #[serde(default)]
    pub animation: EdgeAnimation,
    #[serde(default)]
    pub direction: Option<AnimationDirection>,
    #[serde(default)]
    pub from_nic: Option<String>,
    #[serde(default)]
    pub from_port: Option<usize>,
    #[serde(default)]
    pub to_nic: Option<String>,
    #[serde(default)]
    pub to_port: Option<usize>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Animation {
    #[serde(default)]
    pub preset: Preset,
    #[serde(default = "default_true")]
    pub draw: bool,
    #[serde(default)]
    pub flow: bool,
    #[serde(default = "default_draw_ms", alias = "duration_ms")]
    pub draw_ms: u32,
    #[serde(default = "default_flow_ms")]
    pub flow_ms: u32,
    #[serde(default = "default_stagger")]
    pub stagger_ms: u32,
    #[serde(default)]
    pub direction: AnimationDirection,
    #[serde(default = "default_true")]
    pub autoplay: bool,
}

#[derive(Debug, Clone, Copy, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Preset {
    #[default]
    Subtle,
    #[serde(rename = "none")]
    Off,
    Showcase,
}

impl Default for Animation {
    fn default() -> Self {
        Self {
            preset: Preset::Subtle,
            draw: false,
            flow: false,
            draw_ms: 1200,
            flow_ms: 2600,
            stagger_ms: 80,
            direction: AnimationDirection::Alternate,
            autoplay: true,
        }
    }
}

impl Animation {
    /// Resolve the animation for one edge, honouring per-edge overrides first.
    pub fn for_edge(&self, edge: &Edge) -> EdgeAnimation {
        if edge.animation != EdgeAnimation::Inherit {
            return edge.animation;
        }
        match self.preset {
            Preset::Off => EdgeAnimation::None,
            Preset::Showcase if self.flow => EdgeAnimation::Flow,
            Preset::Subtle | Preset::Showcase if self.draw => EdgeAnimation::Draw,
            _ => EdgeAnimation::None,
        }
    }
}

pub const TONES: &[&str] = &[
    "neutral", "blue", "violet", "green", "amber", "red", "cyan", "pink",
];

pub fn known_tone(tone: &str) -> bool {
    TONES.contains(&tone)
}

pub fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

impl Diagram {
    pub fn validate(&self) -> Result<()> {
        let mut errors = Vec::new();

        if self.version != 1 {
            errors.push(format!("version must be 1 (found {})", self.version));
        }
        if self.title.trim().is_empty() {
            errors.push("title must not be empty".to_string());
        }
        if !self.padding.is_finite() || !(0.0..=200.0).contains(&self.padding) {
            errors.push("padding must be between 0 and 200".to_string());
        }
        if self.nodes.is_empty() {
            errors.push("at least one node is required".to_string());
        }
        if self.nodes.len() > 200 {
            errors.push("at most 200 nodes are supported".to_string());
        }
        if self.edges.len() > 500 {
            errors.push("at most 500 edges are supported".to_string());
        }
        if !(100..=10_000).contains(&self.animation.draw_ms) {
            errors.push("animation.draw_ms must be between 100 and 10000".to_string());
        }
        if !(200..=10_000).contains(&self.animation.flow_ms) {
            errors.push("animation.flow_ms must be between 200 and 10000".to_string());
        }
        if self.animation.stagger_ms > 2_000 {
            errors.push("animation.stagger_ms must be at most 2000".to_string());
        }
        if self.style == DiagramStyle::Architecture {
            if self.layout == LayoutMode::Manual {
                errors.push("architecture diagrams currently require layout: auto".to_string());
            }
            if self.direction != Direction::Tb {
                errors.push("architecture diagrams currently require direction: TB".to_string());
            }
            if !self.groups.is_empty() {
                errors.push("groups are not supported in architecture style".to_string());
            }
        }

        let mut node_ids = std::collections::HashSet::new();
        for node in &self.nodes {
            if !valid_id(&node.id) {
                errors.push(format!(
                    "node id {:?} must use only letters, numbers, '-' and '_' (max 64)",
                    node.id
                ));
            }
            if !node_ids.insert(node.id.as_str()) {
                errors.push(format!("duplicate node id {:?}", node.id));
            }
            if node.label.trim().is_empty() {
                errors.push(format!("node {:?} has an empty label", node.id));
            }
            if node.label.chars().count() > 140 {
                errors.push(format!("node {:?} label is too long (max 140)", node.id));
            }
            if let Some(tone) = &node.tone
                && !known_tone(tone)
            {
                errors.push(format!(
                    "node {:?} has unknown tone {:?} (use one of: {})",
                    node.id,
                    tone,
                    TONES.join(", ")
                ));
            }
            if let Some(icon) = &node.icon
                && !icons::exists(icon)
            {
                errors.push(format!(
                    "node {:?} has unknown icon {:?} (see docs/diagram-dsl.md)",
                    node.id, icon
                ));
            }
            if self.style == DiagramStyle::Detailed {
                if node.kind != NodeKind::Generic
                    || node.ports.is_some()
                    || node.port_columns.is_some()
                    || !node.nics.is_empty()
                {
                    errors.push(format!(
                        "node {:?} uses architecture-only fields but style is detailed",
                        node.id
                    ));
                }
            } else {
                match node.kind {
                    NodeKind::Switch => {
                        let ports = node.ports.unwrap_or(0);
                        if !(1..=128).contains(&ports) {
                            errors.push(format!(
                                "switch node {:?} needs ports between 1 and 128",
                                node.id
                            ));
                        }
                        if let Some(columns) = node.port_columns
                            && !(1..=48).contains(&columns)
                        {
                            errors.push(format!(
                                "node {:?} port_columns must be between 1 and 48",
                                node.id
                            ));
                        }
                    }
                    NodeKind::Server | NodeKind::Vm if node.nics.is_empty() => {
                        errors.push(format!(
                            "node {:?} needs at least one nic in architecture style",
                            node.id
                        ));
                    }
                    _ => {}
                }

                if node.ports.is_some() && node.kind != NodeKind::Switch {
                    errors.push(format!("node {:?} sets ports but is not a switch", node.id));
                }
                if !node.nics.is_empty() && !matches!(node.kind, NodeKind::Server | NodeKind::Vm) {
                    errors.push(format!(
                        "node {:?} sets nics but is not a server or vm",
                        node.id
                    ));
                }

                let mut nic_ids = std::collections::HashSet::new();
                for nic in &node.nics {
                    if !valid_id(&nic.id) {
                        errors.push(format!(
                            "node {:?} nic id {:?} must use only letters, numbers, '-' and '_'",
                            node.id, nic.id
                        ));
                    }
                    if !nic_ids.insert(nic.id.as_str()) {
                        errors.push(format!(
                            "node {:?} has duplicate nic id {:?}",
                            node.id, nic.id
                        ));
                    }
                    if !(1..=16).contains(&nic.ports) {
                        errors.push(format!(
                            "node {:?} nic {:?} ports must be between 1 and 16",
                            node.id, nic.id
                        ));
                    }
                    if let Some(tone) = &nic.tone
                        && !known_tone(tone)
                    {
                        errors.push(format!(
                            "node {:?} nic {:?} has unknown tone {:?}",
                            node.id, nic.id, tone
                        ));
                    }
                }
            }
            if let Some([x, y]) = node.at
                && (!x.is_finite() || !y.is_finite() || x < 0.0 || y < 0.0)
            {
                errors.push(format!(
                    "node {:?} has an invalid at value; expected [x, y] >= 0",
                    node.id
                ));
            }
            if let Some(w) = node.width
                && (!w.is_finite() || w < 40.0)
            {
                errors.push(format!("node {:?} width must be at least 40", node.id));
            }
            if let Some(h) = node.height
                && (!h.is_finite() || h < 30.0)
            {
                errors.push(format!("node {:?} height must be at least 30", node.id));
            }
            if self.layout == LayoutMode::Manual && node.at.is_none() {
                errors.push(format!(
                    "node {:?} is missing 'at' but layout is manual",
                    node.id
                ));
            }
            if self.layout == LayoutMode::Auto && node.at.is_some() {
                errors.push(format!(
                    "node {:?} sets 'at' but layout is auto; set layout: manual or remove it",
                    node.id
                ));
            }
        }

        let mut group_ids = std::collections::HashSet::new();
        let mut group_members: std::collections::HashMap<&str, usize> =
            std::collections::HashMap::new();
        for group in &self.groups {
            if !valid_id(&group.id) {
                errors.push(format!(
                    "group id {:?} must use only letters, numbers, '-' and '_' (max 64)",
                    group.id
                ));
            }
            if !group_ids.insert(group.id.as_str()) {
                errors.push(format!("duplicate group id {:?}", group.id));
            }
            if group.label.trim().is_empty() {
                errors.push(format!("group {:?} has an empty label", group.id));
            }
            if let Some(tone) = &group.tone
                && !known_tone(tone)
            {
                errors.push(format!(
                    "group {:?} has unknown tone {:?} (use one of: {})",
                    group.id,
                    tone,
                    TONES.join(", ")
                ));
            }
        }

        for node in &self.nodes {
            if let Some(group) = &node.group {
                if !group_ids.contains(group.as_str()) {
                    errors.push(format!(
                        "node {:?} references unknown group {:?}",
                        node.id, group
                    ));
                } else {
                    *group_members.entry(group.as_str()).or_insert(0) += 1;
                }
            }
        }
        for group in &self.groups {
            if group_members.get(group.id.as_str()).copied().unwrap_or(0) == 0 {
                errors.push(format!("group {:?} has no nodes", group.id));
            }
        }

        for (index, edge) in self.edges.iter().enumerate() {
            if edge.from == edge.to {
                errors.push(format!("edge #{} is a self-loop", index + 1));
            }
            if !node_ids.contains(edge.from.as_str()) {
                errors.push(format!(
                    "edge #{} starts at unknown node {:?}",
                    index + 1,
                    edge.from
                ));
            }
            if !node_ids.contains(edge.to.as_str()) {
                errors.push(format!(
                    "edge #{} ends at unknown node {:?}",
                    index + 1,
                    edge.to
                ));
            }
            if self.style == DiagramStyle::Detailed {
                if edge.from_nic.is_some()
                    || edge.from_port.is_some()
                    || edge.to_nic.is_some()
                    || edge.to_port.is_some()
                {
                    errors.push(format!(
                        "edge #{} uses architecture port fields but style is detailed",
                        index + 1
                    ));
                }
            } else {
                let from_node = self.nodes.iter().find(|node| node.id == edge.from);
                let to_node = self.nodes.iter().find(|node| node.id == edge.to);

                if edge.from_port.is_none() {
                    errors.push(format!(
                        "edge #{} needs from_port in architecture style",
                        index + 1
                    ));
                }
                if edge.to_port.is_none() {
                    errors.push(format!(
                        "edge #{} needs to_port in architecture style",
                        index + 1
                    ));
                }
                if from_node.is_some_and(|node| {
                    matches!(node.kind, NodeKind::Server | NodeKind::Vm) && edge.from_nic.is_none()
                }) {
                    errors.push(format!(
                        "edge #{} needs from_nic when the source is a server or vm",
                        index + 1
                    ));
                }
                if to_node.is_some_and(|node| {
                    matches!(node.kind, NodeKind::Server | NodeKind::Vm) && edge.to_nic.is_none()
                }) {
                    errors.push(format!(
                        "edge #{} needs to_nic when the target is a server or vm",
                        index + 1
                    ));
                }

                if let Some(node) = from_node {
                    if let Some(nic_id) = &edge.from_nic {
                        if let Some(nic) = node.nics.iter().find(|nic| &nic.id == nic_id) {
                            if let Some(port) = edge.from_port {
                                if port == 0 || port > nic.ports {
                                    errors.push(format!(
                                        "edge #{} from_port {} is outside nic {:?} on node {:?}",
                                        index + 1,
                                        port,
                                        nic_id,
                                        node.id
                                    ));
                                }
                            } else {
                                errors.push(format!(
                                    "edge #{} sets from_nic without from_port",
                                    index + 1
                                ));
                            }
                        } else {
                            errors.push(format!(
                                "edge #{} references unknown nic {:?} on node {:?}",
                                index + 1,
                                nic_id,
                                node.id
                            ));
                        }
                    } else if let Some(port) = edge.from_port
                        && node.kind == NodeKind::Switch
                    {
                        let ports = node.ports.unwrap_or(0);
                        if port == 0 || port > ports {
                            errors.push(format!(
                                "edge #{} from_port {} is outside switch {:?}",
                                index + 1,
                                port,
                                node.id
                            ));
                        }
                    }
                }

                if let Some(node) = to_node {
                    if let Some(nic_id) = &edge.to_nic {
                        if let Some(nic) = node.nics.iter().find(|nic| &nic.id == nic_id) {
                            if let Some(port) = edge.to_port {
                                if port == 0 || port > nic.ports {
                                    errors.push(format!(
                                        "edge #{} to_port {} is outside nic {:?} on node {:?}",
                                        index + 1,
                                        port,
                                        nic_id,
                                        node.id
                                    ));
                                }
                            } else {
                                errors.push(format!(
                                    "edge #{} sets to_nic without to_port",
                                    index + 1
                                ));
                            }
                        } else {
                            errors.push(format!(
                                "edge #{} references unknown nic {:?} on node {:?}",
                                index + 1,
                                nic_id,
                                node.id
                            ));
                        }
                    } else if let Some(port) = edge.to_port
                        && node.kind == NodeKind::Switch
                    {
                        let ports = node.ports.unwrap_or(0);
                        if port == 0 || port > ports {
                            errors.push(format!(
                                "edge #{} to_port {} is outside switch {:?}",
                                index + 1,
                                port,
                                node.id
                            ));
                        }
                    }
                }
            }

            if let Some(label) = &edge.label
                && label.chars().count() > 100
            {
                errors.push(format!("edge #{} label is too long (max 100)", index + 1));
            }
            if let Some(tone) = &edge.tone
                && !known_tone(tone)
            {
                errors.push(format!(
                    "edge #{} has unknown tone {:?} (use one of: {})",
                    index + 1,
                    tone,
                    TONES.join(", ")
                ));
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            bail!("invalid diagram:\n  - {}", errors.join("\n  - "))
        }
    }
}

fn default_version() -> u32 {
    1
}

fn default_padding() -> f64 {
    32.0
}

fn default_nic_ports() -> usize {
    1
}

fn default_true() -> bool {
    true
}

fn default_draw_ms() -> u32 {
    1200
}

fn default_flow_ms() -> u32 {
    2600
}

fn default_stagger() -> u32 {
    80
}

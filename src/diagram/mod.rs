pub mod icons;
pub mod layout;
pub mod model;
pub mod render;
pub mod router;

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

pub const DIAGRAM_DIR: &str = "diagrams";
pub const GENERATED_DIR: &str = "generated/diagrams";

pub fn parse(input: &str) -> Result<model::Diagram> {
    serde_yaml_ng::from_str(input).context("could not parse diagram YAML")
}

pub fn render_yaml(input: &str, prefix: &str, standalone: bool) -> Result<String> {
    let diagram = parse(input)?;
    let layout = layout::layout(&diagram, prefix)?;
    render::render(&layout, standalone)
}

pub fn render_file(path: &Path, standalone: bool) -> Result<String> {
    let input = fs::read_to_string(path)
        .with_context(|| format!("could not read diagram {}", path.display()))?;
    let prefix = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .map(slugify)
        .unwrap_or_else(|| "diagram".to_string());
    render_yaml(&input, &prefix, standalone).with_context(|| format!("in {}", path.display()))
}

pub fn list_files() -> Result<Vec<PathBuf>> {
    let dir = Path::new(DIAGRAM_DIR);
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut files = Vec::new();
    for entry in fs::read_dir(dir).with_context(|| format!("could not read {}", dir.display()))? {
        let path = entry?.path();
        if path.is_file()
            && path
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| {
                    ext.eq_ignore_ascii_case("yaml") || ext.eq_ignore_ascii_case("yml")
                })
        {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

/// Render every diagram into generated/diagrams. Returns the number of source
/// diagrams found. Existing SVGs that no longer have a YAML source are removed.
pub fn build_all() -> Result<usize> {
    let files = list_files()?;
    let output_dir = Path::new(GENERATED_DIR);
    fs::create_dir_all(output_dir)
        .with_context(|| format!("could not create {}", output_dir.display()))?;

    let mut expected = HashSet::new();
    let mut errors = Vec::new();
    let mut changed = 0usize;

    for path in &files {
        let name = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .map(slugify)
            .unwrap_or_else(|| "diagram".to_string());
        expected.insert(format!("{name}.svg"));

        match render_file(path, false) {
            Ok(svg) => {
                let output = output_dir.join(format!("{name}.svg"));
                let unchanged = fs::read_to_string(&output)
                    .map(|existing| existing == svg)
                    .unwrap_or(false);
                if !unchanged {
                    fs::write(&output, svg)
                        .with_context(|| format!("could not write {}", output.display()))?;
                    changed += 1;
                }
            }
            Err(error) => errors.push(format!("{error:#}")),
        }
    }

    // Remove stale files without touching unrelated generated assets.
    if let Ok(entries) = fs::read_dir(output_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let is_svg = path
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| ext.eq_ignore_ascii_case("svg"));
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("");
            if is_svg && !expected.contains(name) {
                let _ = fs::remove_file(path);
            }
        }
    }

    if errors.is_empty() {
        if changed > 0 {
            println!(
                "diagrams: {} source file(s), {changed} updated",
                files.len()
            );
        } else {
            println!("diagrams: {} source file(s), up to date", files.len());
        }
        Ok(files.len())
    } else {
        bail!("{} diagram(s) failed:\n{}", errors.len(), errors.join("\n"));
    }
}

pub fn check_all() -> Result<usize> {
    let files = list_files()?;
    let mut errors = Vec::new();
    for path in &files {
        if let Err(error) = render_file(path, false) {
            errors.push(format!("{error:#}"));
        }
    }
    if errors.is_empty() {
        println!("diagrams: {} source file(s), all valid", files.len());
        Ok(files.len())
    } else {
        bail!("{} diagram(s) failed:\n{}", errors.len(), errors.join("\n"));
    }
}

pub fn slugify(input: &str) -> String {
    let mut output = String::new();
    let mut last_dash = false;
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() {
            output.push(ch.to_ascii_lowercase());
            last_dash = false;
        } else if !last_dash && !output.is_empty() {
            output.push('-');
            last_dash = true;
        }
    }
    while output.ends_with('-') {
        output.pop();
    }
    if output.is_empty() {
        "diagram".to_string()
    } else {
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINIMAL: &str = r#"
version: 1
title: Request path
description: A client calls an API.
nodes:
  - id: client
    label: "Client & Co"
    tone: blue
  - id: api
    label: API
    tone: violet
    icon: kubernetes
edges:
  - from: client
    to: api
    label: HTTPS
animation:
  draw: true
"#;

    #[test]
    fn slugifies_names() {
        assert_eq!(slugify("K8s Control Plane!"), "k8s-control-plane");
        assert_eq!(slugify("  "), "diagram");
    }

    #[test]
    fn renders_and_escapes_text() {
        let svg = render_yaml(MINIMAL, "request-path", false).unwrap();
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("Request path"));
        assert!(svg.contains("Client &amp; Co"));
        assert!(svg.contains("dg-edge--draw"));
        assert!(svg.contains(r#"data-from="client""#));
        assert!(svg.contains("request-path-title"));
        assert!(svg.contains("dg-node-icon"));
        assert!(svg.contains(r#"data-dg-flow-ms="2600""#));
        assert!(svg.contains(r#"data-dg-direction="alternate""#));
        assert!(svg.contains(r#"data-dg-autoplay="true""#));
        assert!(svg.contains(r#"data-dg-route="elbow""#));
    }

    #[test]
    fn standalone_export_embeds_styles_and_dimensions() {
        let svg = render_yaml(MINIMAL, "request-path", true).unwrap();
        assert!(svg.contains("<style>"));
        assert!(svg.contains("width="));
        assert!(svg.contains("viewBox="));
    }

    #[test]
    fn per_edge_direction_is_serialized() {
        let yaml = r#"
version: 1
title: Direction
nodes:
  - id: a
    label: A
  - id: b
    label: B
edges:
  - from: a
    to: b
    animation: flow
    direction: reverse
"#;
        let svg = render_yaml(yaml, "direction", false).unwrap();
        assert!(svg.contains(r#"data-dg-direction="reverse""#));
        assert!(svg.contains("dg-edge--flow"));
        assert!(svg.contains("dg-packet"));
    }

    #[test]
    fn load_balancer_node_serializes_algorithm_and_flow() {
        let yaml = r#"
version: 1
title: Load balancer
nodes:
  - id: lb
    label: Load balancer
    load_balancer: true
    lb_algorithm: random
  - id: a
    label: A
  - id: b
    label: B
edges:
  - from: lb
    to: a
    animation: flow
  - from: lb
    to: b
    animation: flow
"#;
        let svg = render_yaml(yaml, "lb", false).unwrap();
        assert!(svg.contains(r#"data-dg-lb="true""#));
        assert!(svg.contains(r#"data-dg-lb-algorithm="random""#));
        assert!(svg.contains("dg-edge--flow"));
        assert_eq!(svg.matches("dg-packet ").count(), 2);
    }

    #[test]
    fn architecture_style_renders_port_grids_and_links() {
        let yaml = r#"
version: 1
style: architecture
title: Architecture
direction: TB
nodes:
  - id: leaf
    kind: switch
    label: leaf-01
    ports: 8
    port_columns: 4
  - id: host
    kind: server
    label: host-01
    nics:
      - id: nic1
        ports: 2
      - id: nic2
        ports: 2
edges:
  - from: host
    from_nic: nic1
    from_port: 1
    to: leaf
    to_port: 1
    bond: bond0
  - from: host
    from_nic: nic2
    from_port: 1
    to: leaf
    to_port: 2
    bond: bond0
"#;
        let svg = render_yaml(yaml, "arch", false).unwrap();
        assert!(svg.contains(r#"data-dg-kind="switch""#));
        assert!(svg.contains(r#"data-dg-kind="server""#));
        assert!(svg.contains("arch-port"));
        assert!(svg.contains("arch-port-label"));
        assert!(svg.contains("is-used"));
        assert!(svg.contains(r#"data-dg-static="true""#));
        assert_eq!(svg.matches("dg-edge ").count(), 2);
    }

    #[test]
    fn architecture_simple_renders_network_rails() {
        let yaml = r#"
version: 1
style: architecture-simple
title: Logical networks
direction: TB
nodes:
  - id: a
    label: A
  - id: b
    label: B
networks:
  - id: public
    label: Public network
    tone: blue
    members: [a, b]
  - id: cluster
    label: Cluster network
    tone: amber
    members: [a, b]
"#;
        let svg = render_yaml(yaml, "simple", false).unwrap();
        assert!(svg.contains("network-rail"));
        assert!(svg.contains("network-stub"));
        assert!(svg.contains("network-legend-label"));
        assert!(svg.contains(r#"data-dg-no-drag="true""#));
        assert!(svg.contains(r#"data-dg-static="true""#));
        assert!(!svg.contains("dg-edge "));
    }

    #[test]
    fn detailed_style_rejects_architecture_fields() {
        let yaml = r#"
version: 1
style: detailed
title: Mixed
nodes:
  - id: a
    label: A
    kind: switch
    ports: 8
  - id: b
    label: B
edges:
  - from: a
    to: b
"#;
        assert!(render_yaml(yaml, "mixed", false).is_err());
    }

    #[test]
    fn architecture_edges_require_ports() {
        let yaml = r#"
version: 1
style: architecture
title: Missing ports
direction: TB
nodes:
  - id: leaf
    kind: switch
    label: leaf
    ports: 8
  - id: host
    kind: server
    label: host
    nics:
      - id: nic1
        ports: 2
edges:
  - from: host
    from_nic: nic1
    to: leaf
"#;
        assert!(render_yaml(yaml, "missing-ports", false).is_err());
    }

    #[test]
    fn rejects_duplicate_node_ids() {
        let yaml = r#"
version: 1
title: Broken
nodes:
  - id: api
    label: One
  - id: api
    label: Two
"#;
        let error = render_yaml(yaml, "broken", false).unwrap_err().to_string();
        assert!(error.contains("duplicate node id"));
    }

    #[test]
    fn rejects_unknown_edge_targets() {
        let yaml = r#"
version: 1
title: Broken
nodes:
  - id: api
    label: API
edges:
  - from: api
    to: missing
"#;
        let error = render_yaml(yaml, "broken", false).unwrap_err().to_string();
        assert!(error.contains("unknown node"));
    }

    #[test]
    fn manual_layout_requires_positions() {
        let yaml = r#"
version: 1
title: Broken
layout: manual
nodes:
  - id: api
    label: API
"#;
        let error = render_yaml(yaml, "broken", false).unwrap_err().to_string();
        assert!(error.contains("missing 'at'"));
    }

    #[test]
    fn repository_diagrams_render() {
        let files = list_files().unwrap();
        assert!(!files.is_empty(), "expected at least one diagram fixture");
        for file in files {
            let svg = render_file(&file, false).unwrap();
            assert!(svg.contains("<svg"));
            assert!(svg.ends_with("</svg>"));
        }
    }
}

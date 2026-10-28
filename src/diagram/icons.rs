//! A small, curated inline icon set for diagrams.
//!
//! These are generic line glyphs rather than vendor logos. They are deliberately
//! simple so the SVG output stays small and readable at 20px.

pub const ICON_NAMES: &[&str] = &[
    "api",
    "bucket",
    "ceph",
    "cloud",
    "cog",
    "container",
    "database",
    "disk",
    "gateway",
    "globe",
    "kubernetes",
    "linux",
    "loadbalancer",
    "lock",
    "monitor",
    "network",
    "openstack",
    "queue",
    "server",
    "storage",
    "switch",
    "user",
    "vm",
];

pub fn exists(name: &str) -> bool {
    ICON_NAMES.contains(&name)
}

/// Returns the inner markup for a 24x24 stroke icon.
pub fn markup(name: &str) -> Option<&'static str> {
    Some(match name {
        "api" => r#"<path d="M9 5H6v14h3M15 5h3v14h-3"/><path d="M10 9h4M10 12h4M10 15h4"/>"#,
        "bucket" => r#"<path d="M5 6h14l-2 14H7L5 6z"/><path d="M9 6V4h6v2"/>"#,
        "ceph" => {
            r#"<circle cx="12" cy="12" r="8"/><path d="M12 4a8 8 0 0 0 0 16M8 8l8 8M16 8l-8 8"/>"#
        }
        "cloud" => r#"<path d="M7 18h10a4 4 0 0 0 .5-8A6 6 0 0 0 6 11a3.5 3.5 0 0 0 1 7z"/>"#,
        "cog" => {
            r#"<circle cx="12" cy="12" r="3"/><path d="M12 3v3M12 18v3M3 12h3M18 12h3M5.6 5.6l2.1 2.1M16.3 16.3l2.1 2.1M18.4 5.6l-2.1 2.1M7.7 16.3l-2.1 2.1"/>"#
        }
        "container" => {
            r#"<path d="M4 8l8-4 8 4-8 4-8-4z"/><path d="M4 8v8l8 4 8-4V8"/><path d="M12 12v8"/>"#
        }
        "database" => {
            r#"<ellipse cx="12" cy="6" rx="7" ry="3"/><path d="M5 6v12c0 1.7 3.1 3 7 3s7-1.3 7-3V6"/><path d="M5 12c0 1.7 3.1 3 7 3s7-1.3 7-3"/>"#
        }
        "disk" => {
            r#"<ellipse cx="12" cy="6" rx="7" ry="3"/><path d="M5 6v12c0 1.7 3.1 3 7 3s7-1.3 7-3V6"/>"#
        }
        "gateway" => {
            r#"<rect x="4" y="8" width="16" height="8" rx="2"/><path d="M8 12h8M13 9l3 3-3 3"/>"#
        }
        "globe" => {
            r#"<circle cx="12" cy="12" r="8"/><path d="M4 12h16M12 4a12 12 0 0 1 0 16M12 4a12 12 0 0 0 0 16"/>"#
        }
        "kubernetes" => {
            r#"<circle cx="12" cy="12" r="8"/><path d="M12 4v16M4 12h16M6.5 6.5l11 11M17.5 6.5l-11 11"/><circle cx="12" cy="12" r="2"/>"#
        }
        "linux" => {
            r#"<rect x="4" y="5" width="16" height="14" rx="2"/><path d="M8 9l3 3-3 3M13 15h4"/>"#
        }
        "loadbalancer" => {
            r#"<circle cx="6" cy="12" r="2"/><circle cx="18" cy="6" r="2"/><circle cx="18" cy="18" r="2"/><path d="M8 12h4M12 12l4-6M12 12l4 6"/>"#
        }
        "lock" => {
            r#"<rect x="5" y="10" width="14" height="10" rx="2"/><path d="M8 10V8a4 4 0 0 1 8 0v2"/>"#
        }
        "monitor" => {
            r#"<rect x="4" y="5" width="16" height="10" rx="2"/><path d="M12 15v4M8 19h8"/>"#
        }
        "network" => {
            r#"<circle cx="5" cy="5" r="2"/><circle cx="19" cy="5" r="2"/><circle cx="12" cy="19" r="2"/><path d="M7 5h10M6 7l5 10M18 7l-5 10"/>"#
        }
        "openstack" => {
            r#"<path d="M4 7h16M4 12h16M4 17h16"/><circle cx="7" cy="7" r="1"/><circle cx="12" cy="12" r="1"/><circle cx="17" cy="17" r="1"/>"#
        }
        "queue" => {
            r#"<rect x="4" y="5" width="16" height="4" rx="1"/><rect x="4" y="11" width="16" height="4" rx="1"/><rect x="4" y="17" width="16" height="2" rx="1"/>"#
        }
        "server" => {
            r#"<rect x="4" y="4" width="16" height="6" rx="1.5"/><rect x="4" y="14" width="16" height="6" rx="1.5"/><path d="M8 7h.01M8 17h.01"/>"#
        }
        "storage" => {
            r#"<rect x="4" y="4" width="16" height="16" rx="2"/><path d="M4 10h16M4 14h16M8 7h.01M8 12h.01M8 17h.01"/>"#
        }
        "switch" => {
            r#"<rect x="3" y="8" width="18" height="9" rx="2"/><path d="M7 12.5h.01M10 12.5h.01M13 12.5h.01M16 12.5h.01M8 8V5h8v3"/>"#
        }
        "user" => r#"<circle cx="12" cy="8" r="3.5"/><path d="M5 20c0-4 3-6 7-6s7 2 7 6"/>"#,
        "vm" => {
            r#"<rect x="4" y="5" width="16" height="12" rx="2"/><path d="M9 21h6M12 17v4M10 9l4 2-4 2V9z"/>"#
        }
        _ => return None,
    })
}

+++
title = "Everything a post can render"
date = 2026-11-07
description = "A reference post covering every heading level, text format, block type, and diagram style this site supports."
template = "blog/page.html"

[taxonomies]
tags = ["reference", "markdown", "diagrams"]

[extra]
demo = true
highlight = "a post"
+++
This post exercises everything the site can generate. Use it as a reference when you write, and as a visual test when you change the styles.

## Headings

The post title at the top of this page is the only heading level 1. Each page keeps one `h1`, so a post body starts at level 2. The table of contents lists level 2 headings only.

### Heading level 3

Level 3 starts a subsection.

#### Heading level 4

Level 4 divides a subsection.

##### Heading level 5

Level 5 is for short labelled groups.

###### Heading level 6

Level 6 is the smallest heading.

## Text formats

A paragraph can mix **bold**, *italic*, ***bold italic***, ~~strikethrough~~, <u>underline</u>, `inline code`, and a [link to the writing page](@/blog/_index.md). External links, such as [Zola](https://www.getzola.org/), open in a new tab.

| Format | Source | Result |
| :--- | :--- | :--- |
| Bold | `**text**` | **text** |
| Italic | `*text*` | *text* |
| Bold italic | `***text***` | ***text*** |
| Strikethrough | `~~text~~` | ~~text~~ |
| Underline | `<u>text</u>` | <u>text</u> |
| Superscript | `x<sup>2</sup>` | x<sup>2</sup> |
| Subscript | `H<sub>2</sub>O` | H<sub>2</sub>O |
| Highlight | `<mark>text</mark>` | <mark>text</mark> |
| Small | `<small>text</small>` | <small>text</small> |
| Keyboard | `<kbd>Ctrl</kbd>` | <kbd>Ctrl</kbd> |
| Abbreviation | `<abbr title="Object Storage Daemon">OSD</abbr>` | <abbr title="Object Storage Daemon">OSD</abbr> |
| Inline code | `` `code` `` | `code` |

Smart punctuation converts "straight quotes", dashes -- like this -- and ellipses... as you type.

A footnote keeps a side comment out of the sentence.[^1] It also works with a named label.[^named]

[^1]: Footnotes collect at the end of the post, with a link back to the reference.
[^named]: The label in the source does not appear in the output. The site numbers footnotes in order.

## Lists

An unordered list can nest:

- Linux hosts
- OpenStack control planes
  - Nova
  - Neutron
- Ceph storage clusters

An ordered list is a numbered checklist:

1. Map the system.
2. Define done.
3. Keep a rollback path.

A task list tracks state:

- [x] Write the post
- [x] Render the diagrams
- [ ] Replace the demo content

A definition list pairs a term with its meaning:

Failure domain
: A set of components that can fail together.

Placement group
: A Ceph unit that maps objects to OSDs.

## Special blocks

### Markdown alerts

Five alert types use standard Markdown syntax, for example `> [!NOTE]`.

> [!NOTE]
> A note adds useful context that a reader can skip.

> [!TIP]
> A tip shows an easier way to do something.

> [!IMPORTANT]
> An important block states what a reader must know to succeed.

> [!WARNING]
> A warning flags a risk that needs attention now.

> [!CAUTION]
> A caution flags an action with negative consequences.

### Notice blocks

Four more types use the `notice` component. Their body accepts **Markdown**.

{%<notice kind="info">%}
An info block gives neutral background, such as a `version` requirement.
{%</notice>%}

{%<notice kind="suggestion">%}
A suggestion offers an option that a reader can take or leave.
{%</notice>%}

{%<notice kind="recommendation">%}
A recommendation states the option that the author prefers, and why.
{%</notice>%}

{%<notice kind="disclaimer">%}
A disclaimer limits the scope of the content. This post uses demo content only.
{%</notice>%}

### Blockquote

A plain blockquote carries a key statement:

> A boring upgrade is built in advance: map the system, write down what success means, and rehearse the failures.

## Code

A code block sets its language for syntax highlighting:

```rust
fn main() {
    println!("Calm infrastructure is designed.");
}
```

```yaml
nodes:
  - id: client
    label: Client
    tone: blue
```

```bash
cargo run -- diagrams check
```

A code block can also show line numbers and highlight lines:

```toml,linenos,hl_lines=2
[extra]
demo = true
highlight = "a post"
```

A long line scrolls inside the code box, and the language label and Copy button stay in place:

```bash
openstack --os-cloud production-region-one compute service set --disable --disable-reason "planned-maintenance-window" compute-node-0042.example.internal nova-compute
```

## Tables and media

Table columns can align left, centre, or right. Wide tables scroll inside their own box.

| Component | Role | Replicas |
| :--- | :---: | ---: |
| MON | Quorum | 3 |
| OSD | Data | 6 |
| RGW | Gateway | 2 |

An image can sit on its own or in a figure with a caption:

<figure>
  <img src="../../images/footer-art.png" alt="Pixel-art scene: a figure and a campfire on the dunes under shooting stars" width="2880" height="1200" loading="lazy">
  <figcaption>A figure with a caption, using the site footer art.</figcaption>
</figure>

A horizontal rule separates two parts of a post:

---

## Diagrams

Each diagram compiles from a YAML file in `diagrams/`. Every diagram has a toolbar to zoom, fit, reset, and copy as PNG. Animated diagrams also have play, speed, and direction controls.

### Detailed style

The `detailed` style is the default. This is the smallest useful diagram:

{{<diagram name="showcase-minimal" caption="Minimal diagram: two nodes and one labelled edge, with automatic left-to-right layout." />}}

Edges have three line styles, three routes, and three arrow options:

{{<diagram name="showcase-edges" caption="Edge options: solid, dashed, and dotted lines; elbow, straight, and curved routes; arrows on both ends or none. The default arrow is on the end only." />}}

Edges have four animation types, and flow has three directions:

{{<diagram name="showcase-animation" caption="Animation options: none, draw, pulse, and flow in alternating, forward, and reverse directions." />}}

Manual layout places each node at explicit coordinates. This diagram also shows all eight tones and both node shapes:

{{<diagram name="showcase-manual" caption="Manual layout: the eight tones, box and pill shapes, and two groups." />}}

Automatic layout can also run top to bottom, with rank overrides and groups:

{{<diagram name="ceph-data-path" caption="Top-to-bottom layout: groups and rank overrides in a real diagram." />}}

### Architecture detailed style

The `architecture-detailed` style draws physical infrastructure with switch, server, and VM nodes, NIC and switch port grids, port-to-port links, and named bonds. Hover a node to trace its links.

{{<diagram name="showcase-arch-detailed" caption="Architecture detailed: two switches, a server with two bonds, and a VM, linked port to port." />}}

### Architecture simple style

The `architecture-simple` style draws logical network membership. Networks are rails and nodes attach to them. Hover a legend entry to highlight its network.

{{<diagram name="showcase-arch-simple" caption="Architecture simple: three logical networks as rails, with no ports or physical links." />}}

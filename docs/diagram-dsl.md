# Diagram DSL

Diagrams are plain YAML files in `diagrams/` — the source of truth is text,
reviewable and versioned with the rest of the site. There are no binary diagram
assets. The Rust compiler validates the definition, lays out the graph, and
emits an inline SVG. Posts reference the result by name:

```markdown
{{<diagram name="k8s-control-plane" caption="A request path through the cluster." />}}
```

The compiler writes `generated/diagrams/<name>.svg`. Zola reads it with
`load_data` and inlines it, so the diagram inherits the page's color variables.
The small viewer script adds pan/zoom and animation controls without changing
the SVG source.

## Commands

```bash
cargo run -- diagrams check
cargo run -- diagrams build
cargo run -- diagrams render diagrams/k8s-control-plane.yaml --standalone
cargo run -- diagrams render diagrams/k8s-control-plane.yaml --out /tmp/diagram.svg
```

`cargo run -- serve` renders all diagrams, starts Zola, watches `diagrams/`, and
regenerates the SVGs on change. Invalid YAML prints a diagnostic and leaves the
last good SVG in place.

## Minimal example

```yaml
version: 1
title: Request path
description: A client calls an API.
direction: LR

nodes:
  - id: client
    label: Client
    icon: user
    tone: blue
  - id: api
    label: kube-apiserver
    icon: kubernetes
    tone: violet

edges:
  - from: client
    to: api
    label: HTTPS
```

## Top-level fields

| Field | Type | Default | Notes |
| --- | --- | --- | --- |
| `version` | integer | `1` | Must be `1`. |
| `title` | string | required | Used in the SVG `<title>`. |
| `description` | string | derived | Used in the SVG `<desc>`. |
| `direction` | `LR` or `TB` | `LR` | Rank direction for auto layout. |
| `layout` | `auto` or `manual` | `auto` | Manual mode requires `at` on every node. |
| `padding` | number | `32` | Canvas padding, 0–200. |
| `nodes` | list | required | 1–200 nodes. |
| `groups` | list | `[]` | One level of visual containers. |
| `edges` | list | `[]` | Up to 500. |
| `animation` | map | subtle draw | See animation below. |

## Node fields

```yaml
- id: api                  # required; letters, numbers, - and _ only
  label: kube-apiserver    # required; use "\n" for an explicit line break
  icon: kubernetes         # optional; see icon list
  tone: violet             # optional; default neutral
  shape: box               # box | pill
  group: control-plane     # optional group id
  rank: 2                  # optional auto-layout rank override
  at: [120, 80]            # required in manual mode; top-left in px
  width: 180               # optional
  height: 52               # optional
```

The auto layout computes node sizes from label width, so `width`/`height` are
usually unnecessary. Node labels wrap at roughly 26 characters and are capped at
three lines.

## Group fields

```yaml
groups:
  - id: control-plane
    label: Control plane
    tone: violet
```

Groups are visual containers drawn behind their member nodes. Auto layout keeps
members of the same group contiguous within a rank; it does not perform a full
nested-cluster layout. For complex VPC/subnet diagrams, use `layout: manual` and
position the nodes explicitly.

Nested groups are not supported in v1.

## Edge fields

```yaml
edges:
  - from: client
    to: api
    label: HTTPS           # optional
    style: solid           # solid | dashed | dotted
    route: elbow           # elbow | straight | curve
    arrow: end             # end | both | none
    tone: blue             # optional; defaults to the source node tone
    animation: flow        # inherit | none | draw | flow | pulse
    direction: alternate   # optional: forward | reverse | alternate
```

Edges connect to the nearest side of each node. Elbows use an orthogonal
midpoint; curves use a cubic Bézier. Arrow markers are generated per tone.

## Animation

```yaml
animation:
  preset: subtle      # none | subtle | showcase
  draw: true          # draw-in for edges when preset is subtle/showcase
  flow: false         # packet flow when preset is showcase
  draw_ms: 1200       # draw-in duration, 100–10000
  flow_ms: 2600       # flow cycle duration, 200–10000
  stagger_ms: 80      # 0–2000; per-edge delay
  direction: alternate # forward | reverse | alternate
  autoplay: true      # false starts paused
```

`animation` on an edge overrides the global preset. `flow` renders a normal
solid line with a glowing packet travelling along it; `draw` uses a normalized
path length for the draw-in. `direction` on an edge overrides the global
direction for that flow. The packet animation is part of the interactive
viewer, so a static/standalone SVG keeps the solid line without the packet.

The default is deliberately calm: 2.6s flow cycles, a 1.2s draw-in, and
alternating flow so packets travel out and back. The viewer starts paused when
the reader prefers reduced motion, and pauses when the diagram is off-screen.

## Interactive viewer

Diagrams are rendered inside a fixed-height SVG canvas with a toolbar:

- **Play/Pause**: start or freeze the animation at its current position.
- **Disable/Enable**: turn motion off entirely; packets and trails hide and the
  draw-in/pulse effects settle at their completed state.
- **Packet distribution**: Flow (all edges), round robin, random, and least
  connections. Non-Flow modes dispatch one packet at a time per source node,
  like a load balancer spreading requests across its backends.
- **Speed**: 0.5×, 1×, 1.5×, 2×.
- **Direction**: forward, reverse, alternating (the default).
- **Zoom**: mouse wheel over the canvas, buttons, `+`/`-`, or pinch.
- **Pan**: right-drag, drag empty canvas space, or arrow keys when focused.
- **Move nodes**: hold and drag any node. Only the node and its incident edges
  move; other routes stay untouched, and incident elbows are translated rather
  than rerouted through the diagram.
- **Fit**: show the whole diagram.
- **Reset**: restore the original geometry, initial view, speed, direction, and
  animation state.
- **PNG**: copy the current diagram as a PNG image to the clipboard (downloads
  it when clipboard image writing is unavailable). The export is static, shows
  the caption and an inline `@ajiarya` credit on the same row, and is sized
  dynamically so the caption stays readable.
- **Keyboard**: `+`/`-`, `0`/`f` fit, arrows pan, Space play/pause.

Packets leave a short glowing trail behind the moving light. The trail follows
the current direction, including the reverse leg of alternating flow.

On phones the viewer starts at a legible scale rather than shrinking a wide
diagram into a strip. The page itself does not scroll horizontally; the canvas
pans. With JavaScript disabled, the SVG is static and the controls are hidden.

## Tones

`neutral`, `blue`, `violet`, `green`, `amber`, `red`, `cyan`, `pink`.

Tones map to CSS variables (`--dg-blue`, etc.) defined in `sass/_tokens.scss`.
The same variables are used for nodes, groups, edges, markers, and icons, so a
diagram follows the site's terminal theme automatically.

## Icons

`api`, `bucket`, `ceph`, `cloud`, `cog`, `container`, `database`, `disk`,
`gateway`, `globe`, `kubernetes`, `linux`, `loadbalancer`, `lock`, `monitor`,
`network`, `openstack`, `queue`, `server`, `storage`, `user`, `vm`.

Icons are generic line glyphs embedded in the binary; no external icon requests
are made. Unknown names fail validation with the available list.

## Layout notes

- `auto` ranks nodes by the longest path through the edge graph. Cycles are
  broken by removing DFS back edges for ranking; the edges are still drawn.
- `rank` lets you pin a node to a layer when the default graph suggests an
  awkward order.
- `manual` places nodes at their `at` coordinates. Groups are computed as
  bounding boxes around their members, and the canvas is expanded to fit.
- `TB` direction is useful for portrait-friendly diagrams; the viewer starts at
  a readable scale on phones rather than shrinking the whole diagram.
- Edges that share a node side get distributed ports and, where useful, separate
  lanes, so parallel paths do not leave from the same pixel.
- `elbow` edges use an obstacle-aware orthogonal router that avoids other nodes
  and group labels. `straight` and `curve` edges are direct; use elbow for dense
  architecture diagrams.
- The canvas is derived from node and group bounds plus `padding`.

## Diagnostics

Validation catches:

- duplicate node/group ids
- unknown group references
- unknown edge endpoints and self-loops
- unknown tones and icons
- missing `at` in manual mode
- `at` in auto mode
- empty labels, invalid ids, and out-of-range animation values

Errors include the diagram file and, where possible, the node or edge number.

## Standalone export

`--standalone` embeds the palette and diagram CSS into the SVG so it can be
shared outside the site. Inline SVGs intentionally rely on the site stylesheet to
avoid duplicating CSS on every page.

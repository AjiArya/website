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
style: detailed
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

## Diagram styles

A diagram declares exactly one style:

- **`detailed`** — the original style, used by the Ceph data-path and Kubernetes
  diagrams. It supports groups, flow/pulse animation, load-balancer nodes, and
  node dragging.
- **`architecture-detailed`** — physical and virtual infrastructure with
  switch/server/VM node kinds, full NIC and switch port grids, and port-to-port
  links. Architecture diagrams are static by default and use the same
  pan/zoom/drag/PNG viewer.
- **`architecture-simple`** — logical network membership only. Networks are drawn
  as rails and nodes attach to the rails they belong to; there are no ports or
  physical links.

### Architecture detailed style

```yaml
version: 1
style: architecture-detailed
title: Ceph storage fabric
direction: TB

nodes:
  - id: leaf1
    kind: switch
    label: leaf-01
    ports: 48
    port_columns: 24

  - id: mon1
    kind: server
    label: ceph-mon-01
    nics:
      - id: nic1
        ports: 2
        tone: blue
      - id: nic2
        ports: 2
        tone: amber

edges:
  - from: mon1
    from_nic: nic1
    from_port: 1
    to: leaf1
    to_port: 1
```

Architecture rules:

- `kind`: `switch`, `server`, `vm`, or `generic`.
- A switch needs `ports` and can set `port_columns`; the full port grid is drawn.
- A server or VM needs at least one NIC. Each NIC declares its port count and an
  optional tone.
- Every architecture edge needs `from_port` and `to_port`. A server/VM endpoint
  also needs `from_nic` or `to_nic`.
- Architecture style currently requires `direction: TB`, `layout: auto`, and no
  groups.
- Architecture edges are solid, unanimated port-to-port links. Switch ports in
  use are highlighted and take the bond or link tone.
- Optional `bond` groups edges with the same bond on the same source host. Bond
  members share a color and get a dashed badge below the host, labeled with the
  bond name.
- Hovering a node highlights its incident links and bond markers and dims the
  rest, which makes a dense fabric traceable without zooming.
- `groups` and manual layout are not supported in architecture styles.

### Architecture simple style

```yaml
version: 1
style: architecture-simple
title: Ceph logical networks
direction: TB
nodes:
  - id: client
    kind: generic
    label: Clients
  - id: mon1
    kind: server
    label: ceph-mon-01
  - id: osd1
    kind: server
    label: ceph-osd-01
  - id: osd2
    kind: server
    label: ceph-osd-02
networks:
  - id: public
    label: Public network
    tone: blue
    members: [client, mon1, osd1]
  - id: cluster
    label: Cluster network
    tone: amber
    members: [osd1, osd2]
```

Logical diagrams use `networks` instead of `edges`. Each network has a label,
optional tone, and a member list. The first network is drawn as the top rail,
the second as the bottom rail, and additional networks alternate outward. Nodes
are arranged in a horizontal row between the rails, with a legend in the
top-left. Hovering a legend entry highlights that network's rail and stubs.
Server port cells keep their NIC tones. Node dragging is disabled in this style
because the rails are fixed; pan, zoom, fit, reset, and PNG remain.

Architecture-simple rules:

- `direction: TB`, `layout: auto`, no groups, and no edges.
- No NIC or port fields on nodes.
- At least one network, each with two or more existing node members.

## Top-level fields

| Field | Type | Default | Notes |
| --- | --- | --- | --- |
| `version` | integer | `1` | Must be `1`. |
| `style` | `detailed`, `architecture-detailed`, or `architecture-simple` | `detailed` | One style per diagram. |
| `title` | string | required | Used in the SVG `<title>`. |
| `description` | string | derived | Used in the SVG `<desc>`. |
| `direction` | `LR` or `TB` | `LR` | Rank direction for auto layout. |
| `layout` | `auto` or `manual` | `auto` | Manual mode requires `at` on every node. |
| `padding` | number | `32` | Canvas padding, 0–200. |
| `nodes` | list | required | 1–200 nodes. |
| `groups` | list | `[]` | One level of visual containers; detailed style only. |
| `edges` | list | `[]` | Up to 500; not used by architecture-simple. |
| `networks` | list | `[]` | Logical networks; architecture-simple only. |
| `animation` | map | subtle draw | See animation below. |

## Node fields

```yaml
- id: api                  # required; letters, numbers, - and _ only
  label: kube-apiserver    # required; use "\n" for an explicit line break
  icon: kubernetes         # optional; Iconify name, see Icons
  tone: violet             # optional; default neutral
  shape: box               # box | pill
  group: control-plane     # optional group id
  rank: 2                  # optional auto-layout rank override
  at: [120, 80]            # required in manual mode; top-left in px
  width: 180               # optional
  height: 52               # optional
  load_balancer: true      # optional; distribute packets across outgoing edges
  lb_algorithm: round-robin # optional: round-robin | random | least-connections
  kind: server             # architecture style: switch | server | vm | ...
  ports: 48                # architecture style switch port count
  port_columns: 24         # architecture style switch grid columns
  nics:                    # architecture style server/vm NICs
    - id: nic1
      ports: 2
      tone: blue
```

The auto layout computes node sizes from label width, so `width`/`height` are
usually unnecessary. Node labels wrap at roughly 26 characters and are capped at
three lines.

### Load balancing

A node marked `load_balancer: true` distributes its outgoing **flow** packets
instead of starting every edge packet at once. Set `animation: flow` on the
edges that should carry requests; other outgoing edges keep their configured
animation. Use `lb_algorithm` to choose `round-robin` (default), `random`, or
`least-connections`; the algorithm is applied per load-balancer node in the
viewer, with no global mode switch.

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
    from_nic: nic1         # architecture style only
    from_port: 1           # architecture style only
    to_nic: nic2           # architecture style only, for server targets
    to_port: 2             # architecture style only
    bond: bond0            # architecture style optional bond grouping
```

Edges connect to the nearest side of each node. Elbows use an orthogonal
midpoint; curves use a cubic Bézier. Arrow markers are generated per tone.

## Animation

```yaml
animation:
  preset: subtle      # none | subtle | showcase
  draw: false         # optional draw-in when preset is subtle/showcase
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

The default is deliberately calm: 2.6s flow cycles and alternating flow so
packets travel out and back. Draw-in is off by default so links appear fully
drawn on first load; set `draw: true` explicitly to opt into the wipe. The
viewer starts paused when the reader prefers reduced motion, and pauses when the
diagram is off-screen.

## Interactive viewer

Diagrams are rendered inside a fixed-height SVG canvas with a toolbar:

- **Animation menu**: one animation button. Hover, focus, or click it to choose
  **Start**, **Pause**, or **Disable**. Disable hides packets and trails and
  settles draw/pulse effects at their completed state. The trigger shows the
  current state; Escape closes the menu and returns focus to the trigger.
- **Speed**: hover or focus the speed value for a slider with 0.25×, 0.5×, 1×,
  1.5×, and 2× stops.
- **Direction**: hover, focus, or click the arrow for clickable **Forward**,
  **Reverse**, and **Alternating** (default) options.
- **Zoom**: mouse wheel over the canvas, buttons, `+`/`-`, or pinch.
  Static diagrams hide only the animation controls; pan/zoom, selection, reset,
  and PNG remain available. Logical architecture diagrams keep pan/zoom and PNG
  but do not allow node dragging.
- **Pan**: right-drag on desktop, drag empty canvas space on touch, or use the
  arrow keys when the canvas is focused.
- **Select**: left-drag an empty area to draw a selection box around nodes.
  Hold Shift to add to the current selection; a plain click on empty canvas
  clears it.
- **Move nodes**: drag any node. Select several nodes by box or Shift-click,
  then drag any selected node to move the whole set. Only selected nodes and
  their incident edges move; other routes stay untouched, and incident elbows
  are translated rather than rerouted through the diagram.
- **Move groups**: drag a group box or label; every node inside the group moves
  with it and keeps its relative position.
- **Fit**: show the whole diagram.
- **Reset**: restore the original geometry, initial view, speed, direction, and
  animation state, and clear the current selection.
- **PNG**: copy the current diagram as a PNG image to the clipboard (downloads
  it when clipboard image writing is unavailable). The export is static, shows
  the caption and an inline `@ajiarya` credit on the same row, and is sized
  dynamically so the caption stays readable.
- **Keyboard**: `+`/`-`, `0`/`f` fit, arrows pan, Space play/pause, Escape
  clears the selection.

Packets leave a glowing trail drawn directly on the edge behind the moving
light. The trail follows the current direction, including the reverse leg of
alternating flow.

On phones the viewer starts at a legible scale rather than shrinking a wide
diagram into a strip. The page itself does not scroll horizontally; the canvas
pans. With JavaScript disabled, the SVG is static and the controls are hidden.

## Tones

`neutral`, `blue`, `violet`, `green`, `amber`, `red`, `cyan`, `pink`.

Tones map to CSS variables (`--dg-blue`, etc.) defined in `sass/_tokens.scss`.
The same variables are used for nodes, groups, edges, markers, and icons, so a
diagram follows the site's terminal theme automatically.

## Icons

Icons come from [Iconify](https://iconify.design/). Bare names resolve to
Tabler first, then Simple Icons. Prefix with `tabler:` or `simple-icons:` to
force a set.

```yaml
icon: server
icon: brand-docker
icon: simple-icons:ceph
```

Common aliases: `api`, `ceph`, `cog`, `container`, `database`, `disk`,
`gateway`, `globe`, `kubernetes`, `monitor`, `server`, `storage`, `switch`,
`user`, `vm`.

Only icons referenced in `diagrams/*.yaml` plus the defaults are generated.
After adding or changing an `icon:` value, regenerate the Rust map:

```bash
npm install
npm run icons
```

The generator writes `src/diagram/icons_generated.rs`. The site build makes no
external icon requests.

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

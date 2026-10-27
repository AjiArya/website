# Diagram sources

Every diagram on the site is a plain YAML file in this directory. These files
are the source of truth and are committed to git; the rendered SVGs in
`generated/diagrams/` are build artifacts and are ignored.

A post references a diagram by filename stem:

```markdown
{{<diagram name="k8s-control-plane" caption="The request path through the cluster." />}}
```

Minimal definition:

```yaml
version: 1
title: Client to API
direction: TB
nodes:
  - id: client
    label: Client
    icon: user
    tone: blue
  - id: api
    label: API
    icon: api
    tone: green
edges:
  - from: client
    to: api
    label: HTTPS
    animation: flow
```

Useful commands:

```bash
cargo run -- diagrams check
cargo run -- diagrams build
cargo run -- diagrams render diagrams/k8s-control-plane.yaml --standalone
```

See [`docs/diagram-dsl.md`](../docs/diagram-dsl.md) for the full schema, themes,
layout rules, animation controls, and icons.

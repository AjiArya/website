# Aji Arya — personal site

A fast, static personal site for a cloud engineer and architect. Built with
[Zola](https://www.getzola.org/) and a small Rust toolchain that compiles
YAML definitions into accessible, animated SVG architecture diagrams.

The site is one repository: content, theme, diagram compiler, and deployment
configuration versioned together. There is no separate theme repo and no
client-side framework.

## Quick start

Requirements:

- Rust (the pinned toolchain in `rust-toolchain.toml`)
- `curl` and `tar` for the Zola bootstrap

```bash
cargo run -- serve
```

The first run downloads Zola `0.23.6` into `.tools/` and then starts the site at
<http://127.0.0.1:3000> with drafts enabled. Zola reloads pages on content,
template, Sass, and static changes. The wrapper watches `diagrams/` and rebuilds
the generated SVGs.

Other commands:

```bash
cargo run -- build             # production build into public/
cargo run -- build --drafts    # include draft posts
cargo run -- new "Post title"  # create content/blog/<slug>/index.md
cargo run -- diagrams check    # validate every YAML diagram
cargo run -- diagrams build    # render YAML into generated/diagrams/
cargo run -- diagrams render diagrams/k8s-control-plane.yaml --standalone
cargo run -- setup             # install the pinned Zola binary
```

`./scripts/check.sh` runs formatting, Clippy, tests, and a production build.

## Theme

The site uses a dark, terminal-inspired theme built around the signal green
`#1CB900`: near-black surfaces, mono display typography, scanlines, technical
labels, and green accents for links, focus states, diagrams, and interactive
elements. It is intentionally dark-only; there is no theme switch or
system-preference override.

## Structure

```text
website/
├── Cargo.toml                 # Rust CLI + diagram compiler
├── zola.toml                  # Zola 0.23 configuration
├── content/                   # Markdown pages, page bundles, blog posts
├── data/                      # Home page / about data (YAML)
├── diagrams/                  # YAML diagram sources
├── generated/diagrams/        # Rendered SVGs (gitignored)
├── templates/                 # Tera templates + components
├── sass/                      # Terminal theme styles
├── static/                    # Fonts, images, JS, favicon
├── src/                       # Rust CLI and diagram engine
├── scripts/                   # Zola bootstrap and local checks
└── .github/workflows/         # CI and GitHub Pages deployment
```

Generated SVG files live outside `static/` so they are inlined into posts without
also being copied to the public output. The shortcode/component reads them with
`load_data`, and `zola serve --extra-watch-path generated` triggers a rebuild
when the compiler updates them.

## Writing

Create a post:

```bash
cargo run -- new "Designing a Ceph cluster" --tag ceph --tag storage
```

This creates a page bundle at `content/blog/<slug>/index.md`. Edit the
frontmatter and body; co-locate images in the same directory and link to them
relatively.

Diagrams are referenced from Markdown with a Tera component:

```markdown
{{<diagram name="k8s-control-plane" caption="The request path through Kubernetes." />}}
```

Diagram definitions are text YAML in `diagrams/`, versioned with the content —
no binary diagram assets. The compiler uses distributed ports, obstacle-aware
routing, and separated lanes so edges avoid boxes and parallel paths do not
collapse into one line. The viewer adds play/pause, speed, flow direction,
scroll/pinch zoom, right-drag panning, draggable nodes, fit, reset, and
clipboard PNG export; it loads only on pages that actually contain a diagram.

See [docs/diagram-dsl.md](docs/diagram-dsl.md) for the YAML schema.

## Deployment

The included workflow builds the site and deploys `public/` to GitHub Pages on
every push to `main`.

1. In the GitHub repository, open **Settings → Pages** and set **Source** to
   **GitHub Actions**.
2. Push to `main`.
3. The site is published at <https://ajiarya.github.io/website/>.

If Pages is not enabled, the workflow still builds the site, uploads a
`site-build` artifact, and skips deployment with a warning. Enable Pages and
re-run the workflow to publish.

`base_url` in `zola.toml` is set to the project-site URL. For a custom domain,
change `base_url` and add the domain to `static/CNAME`. For a user site, rename
the repository to `AjiArya.github.io` and update `base_url`.

## Before publishing

- Replace the demo posts with real writing.
- Fill in `linkedin` and `email` under `[extra.social]` in `zola.toml` (empty
  values are hidden automatically).
- Add a real profile image if desired; the current identity mark is typographic.
- If using a custom domain, update `base_url` and add `static/CNAME`.

## Responsive behaviour

- Tested at 320×568, 360×640, 390×844, 412×915, 768×1024, and desktop widths.
- The header becomes a 44px-target menu on phones and tablets.
- The post table of contents becomes a collapsible “On this page” block.
- The diagram viewer uses a portrait-friendly canvas with a readable initial
  scale, scroll/pinch zoom, right-drag panning, draggable nodes, and a full
  layout reset.
- Code blocks and tables scroll inside their own containers; the page itself
  does not scroll horizontally.

## Performance and accessibility

- No client-side framework, no external requests, no tracking.
- System font stacks only: no font downloads.
- A small stylesheet, a separate syntax-highlighting stylesheet, and a ~1 KB
  progressive-enhancement script.
- Diagrams are inline SVG with `<title>`/`<desc>`, semantic groups, keyboard
  controls, and `prefers-reduced-motion` support. The viewer script is loaded
  only on diagram pages.
- Zola minifies HTML and compresses Sass output.
- The home page is roughly 3.2 KB HTML + 7.2 KB CSS gzipped; a diagram post is
  roughly 6.1 KB HTML + the inline SVG and ~9 KB gzipped viewer JS (loaded only
  on diagram pages).

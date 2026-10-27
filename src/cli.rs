use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "site",
    version,
    about = "Build and serve the personal site, including YAML diagrams"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Generate diagrams and start Zola with live reload.
    Serve(ServeArgs),
    /// Generate diagrams and build the production site.
    Build(BuildArgs),
    /// Create a new blog post bundle.
    New(NewArgs),
    /// Work with diagram YAML directly.
    Diagrams(DiagramsArgs),
    /// Download the pinned Zola binary into .tools/.
    Setup,
}

#[derive(Debug, clap::Args)]
pub struct ServeArgs {
    /// Port for the local Zola server.
    #[arg(long, default_value_t = 3000)]
    pub port: u16,

    /// Interface to bind.
    #[arg(long, default_value = "127.0.0.1")]
    pub interface: String,

    /// Hide draft content.
    #[arg(long)]
    pub no_drafts: bool,

    /// Open the site in the default browser.
    #[arg(long)]
    pub open: bool,
}

#[derive(Debug, clap::Args)]
pub struct BuildArgs {
    /// Include draft content in the build.
    #[arg(long)]
    pub drafts: bool,

    /// Override the base URL configured in zola.toml.
    #[arg(long)]
    pub base_url: Option<String>,
}

#[derive(Debug, clap::Args)]
pub struct NewArgs {
    /// Post title.
    pub title: String,

    /// Tag to attach to the post. Repeat for multiple tags.
    #[arg(long = "tag")]
    pub tags: Vec<String>,

    /// Optional description used in listings and social metadata.
    #[arg(long)]
    pub description: Option<String>,
}

#[derive(Debug, clap::Args)]
pub struct DiagramsArgs {
    #[command(subcommand)]
    pub command: DiagramCommand,
}

#[derive(Debug, Subcommand)]
pub enum DiagramCommand {
    /// Render every diagram into generated/diagrams.
    Build,
    /// Validate every diagram without writing output.
    Check,
    /// Render a single YAML file to stdout or a file.
    Render {
        /// Diagram YAML file.
        file: std::path::PathBuf,
        /// Write to this path instead of stdout.
        #[arg(long)]
        out: Option<std::path::PathBuf>,
        /// Embed styles for sharing the SVG outside the site.
        #[arg(long)]
        standalone: bool,
    },
}

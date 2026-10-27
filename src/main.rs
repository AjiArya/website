mod cli;
mod diagram;
mod scaffold;
mod watch;
mod zola;

use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};
use clap::Parser;

use crate::cli::{Cli, Command, DiagramCommand};

fn main() {
    if let Err(error) = run() {
        eprintln!("\nerror: {error:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Serve(args) => serve(args),
        Command::Build(args) => build(args),
        Command::New(args) => {
            let path = scaffold::new_post(&args.title, &args.tags, args.description.as_deref())?;
            println!("created {}", path.display());
            println!("next: cargo run -- serve");
            Ok(())
        }
        Command::Diagrams(args) => match args.command {
            DiagramCommand::Build => {
                diagram::build_all()?;
                Ok(())
            }
            DiagramCommand::Check => {
                diagram::check_all()?;
                Ok(())
            }
            DiagramCommand::Render {
                file,
                out,
                standalone,
            } => {
                let svg = diagram::render_file(&file, standalone)?;
                if let Some(path) = out {
                    if let Some(parent) = path.parent() {
                        fs::create_dir_all(parent)
                            .with_context(|| format!("could not create {}", parent.display()))?;
                    }
                    fs::write(&path, svg)
                        .with_context(|| format!("could not write {}", path.display()))?;
                    println!("wrote {}", path.display());
                } else {
                    print!("{svg}");
                }
                Ok(())
            }
        },
        Command::Setup => {
            zola::install()?;
            Ok(())
        }
    }
}

fn serve(args: cli::ServeArgs) -> Result<()> {
    fs::create_dir_all(Path::new(diagram::GENERATED_DIR))
        .context("could not create generated diagram directory")?;
    diagram::build_all()?;

    let mut zola_args = vec![
        "serve".to_string(),
        "--extra-watch-path".to_string(),
        "generated".to_string(),
        "--interface".to_string(),
        args.interface.clone(),
        "--port".to_string(),
        args.port.to_string(),
    ];
    if !args.no_drafts {
        zola_args.push("--drafts".to_string());
    }
    if args.open {
        zola_args.push("--open".to_string());
    }

    let mut child = zola::spawn(&zola_args)?;
    watch::spawn_diagram_watcher()?;
    println!(
        "Serving http://{}:{} (drafts: {})",
        args.interface,
        args.port,
        if args.no_drafts { "hidden" } else { "visible" }
    );

    let status = child.wait().context("could not wait for zola serve")?;
    if !status.success() {
        bail!("zola serve exited with {status}");
    }
    Ok(())
}

fn build(args: cli::BuildArgs) -> Result<()> {
    fs::create_dir_all(Path::new(diagram::GENERATED_DIR))
        .context("could not create generated diagram directory")?;
    diagram::build_all()?;

    let mut zola_args = vec!["build".to_string()];
    if args.drafts {
        zola_args.push("--drafts".to_string());
    }
    if let Some(base_url) = args.base_url {
        zola_args.push("--base-url".to_string());
        zola_args.push(base_url);
    }

    zola::run(&zola_args)?;
    println!("site: public/");
    Ok(())
}

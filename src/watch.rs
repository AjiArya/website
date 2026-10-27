use std::path::Path;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use anyhow::{Context, Result};
use notify::{EventKind, RecursiveMode, Watcher};

use crate::diagram;

pub fn spawn_diagram_watcher() -> Result<()> {
    let diagrams = Path::new(diagram::DIAGRAM_DIR);
    std::fs::create_dir_all(diagrams)
        .with_context(|| format!("could not create {}", diagrams.display()))?;

    let (tx, rx) = mpsc::channel();
    let mut watcher = notify::recommended_watcher(move |event| {
        let _ = tx.send(event);
    })
    .context("could not create file watcher")?;
    watcher
        .watch(diagrams, RecursiveMode::Recursive)
        .with_context(|| format!("could not watch {}", diagrams.display()))?;

    thread::spawn(move || {
        // Keep the watcher alive for the lifetime of this thread.
        let _watcher = watcher;
        loop {
            match rx.recv() {
                Ok(Ok(event)) => {
                    // Ignore access events: build_all reads the YAML files, and
                    // rebuilding on our own reads creates an endless loop.
                    if !matches!(
                        event.kind,
                        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
                    ) {
                        continue;
                    }

                    // Editors often emit several events per save; collapse them.
                    thread::sleep(Duration::from_millis(90));
                    while rx.try_recv().is_ok() {}

                    match diagram::build_all() {
                        Ok(_) => {}
                        Err(error) => {
                            eprintln!("\ndiagram error (last good SVG kept):\n{error:#}\n");
                        }
                    }
                }
                Ok(Err(error)) => eprintln!("diagram watcher error: {error}"),
                Err(_) => break,
            }
        }
    });

    Ok(())
}

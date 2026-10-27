use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use chrono::Local;

use crate::diagram::slugify;

pub fn new_post(title: &str, tags: &[String], description: Option<&str>) -> Result<PathBuf> {
    if title.trim().is_empty() {
        bail!("title must not be empty");
    }

    let slug = slugify(title);
    if slug.is_empty() || slug == "diagram" {
        bail!("could not derive a useful slug from {title:?}");
    }

    let directory = Path::new("content/blog").join(&slug);
    if directory.exists() {
        bail!(
            "{} already exists; choose a different title or edit the post",
            directory.display()
        );
    }

    fs::create_dir_all(&directory)
        .with_context(|| format!("could not create {}", directory.display()))?;

    let date = Local::now().format("%Y-%m-%d").to_string();
    let description = description
        .unwrap_or("")
        .replace('\\', "\\\\")
        .replace('"', "\\\"");
    let title = title.replace('\\', "\\\\").replace('"', "\\\"");
    let tags = tags
        .iter()
        .map(|tag| format!("\"{}\"", tag.replace('\\', "\\\\").replace('"', "\\\"")))
        .collect::<Vec<_>>()
        .join(", ");

    let body = format!(
        r#"+++
title = "{title}"
date = {date}
description = "{description}"
template = "blog/page.html"

[taxonomies]
tags = [{tags}]

[extra]
demo = false
+++

Write the post here.

Use a diagram with:

{{{{<diagram name="k8s-control-plane" caption="Kubernetes control plane on OpenStack" />}}}}
"#
    );

    let path = directory.join("index.md");
    fs::write(&path, body).with_context(|| format!("could not write {}", path.display()))?;
    Ok(path)
}

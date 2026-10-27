use std::env;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};

use anyhow::{Context, Result, bail};

pub fn ensure_zola() -> Result<PathBuf> {
    if let Some(bin) = env::var_os("ZOLA_BIN") {
        let path = PathBuf::from(bin);
        if path.is_file() {
            return Ok(path);
        }
        bail!("ZOLA_BIN points to a missing file: {}", path.display());
    }

    let local = PathBuf::from(".tools/zola");
    if local.is_file() {
        return Ok(local);
    }

    if Command::new("zola")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok()
    {
        return Ok(PathBuf::from("zola"));
    }

    if env::var_os("SITE_NO_BOOTSTRAP").is_some() {
        bail!("Zola was not found. Run ./scripts/install-zola.sh or set ZOLA_BIN, then try again.");
    }

    eprintln!("Zola was not found; downloading the pinned version...");
    install()?;

    if local.is_file() {
        Ok(local)
    } else {
        bail!("Zola installation did not produce .tools/zola")
    }
}

pub fn install() -> Result<()> {
    let status = Command::new("bash")
        .arg("scripts/install-zola.sh")
        .status()
        .context("could not run scripts/install-zola.sh")?;
    if !status.success() {
        bail!("scripts/install-zola.sh failed with {status}");
    }
    Ok(())
}

pub fn run(args: &[String]) -> Result<()> {
    let bin = ensure_zola()?;
    let status = Command::new(&bin)
        .args(args)
        .status()
        .with_context(|| format!("could not run {}", bin.display()))?;
    if !status.success() {
        bail!("zola {} exited with {status}", args.join(" "));
    }
    Ok(())
}

pub fn spawn(args: &[String]) -> Result<Child> {
    let bin = ensure_zola()?;
    Command::new(&bin)
        .args(args)
        .spawn()
        .with_context(|| format!("could not run {}", bin.display()))
}

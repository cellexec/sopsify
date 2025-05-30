use anyhow::Context;
use clap::Parser;
use std::{collections::HashMap, fs, path::PathBuf};

// CLI arguments
#[derive(Parser, Debug)]
#[command(name = "sopsify")]
#[command(
    about = "Replace placeholders in YAML files and encrypt them with GPG",
    long_about = "Sopsify replaces ${placeholders} in YAML manifests with real values
from a central YAML secrets file and encrypts the output using a GPG key.
Designed for GitOps workflows using SOPS + Flux."
)]
struct Args {
    #[arg(short, long, help = "Path to the GPG key (e.g. gpg.asc)")]
    gpg_key: PathBuf,

    #[arg(short, long, help = "Path to the secrets file (YAML format)")]
    secrets_file: PathBuf,

    #[arg(short, long, help = "Folder containing secret templates")]
    templates_dir: PathBuf,

    #[arg(short, long, help = "Output directory (to write encrypted files)")]
    output_dir: Option<PathBuf>,
}

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let args = Args::parse();
    println!("⚙️ Passed Arguments");
    println!("   > GPG file: {:?}", args.gpg_key);
    println!("   > Secrets file: {:?}", args.secrets_file);
    println!("   > Templates directory: {:?}", args.templates_dir);
    println!("");

    let secrets = load_secrets(&args.secrets_file)?;
    println!("✅ Loaded secrets: {:#?}", secrets.keys());

    // for now return OK
    Ok(())
}

fn load_secrets(path: &PathBuf) -> anyhow::Result<HashMap<String, String>> {
    let contents = fs::read_to_string(path)
        .with_context(|| format!("Failed to read secrets file at {:?}", path))?;

    let secrets: HashMap<String, String> = serde_yaml::from_str(&contents)
        .with_context(|| format!("Failed to parse YAML in secrets file at {:?}", path))?;

    Ok(secrets)
}

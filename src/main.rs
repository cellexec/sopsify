use clap::Parser;
use std::path::PathBuf;

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
    println!("🔐 Using GPG key: {:?}", args.gpg_key);
    println!("👀 Loading secrets from: {:?}", args.secrets_file);
    println!("📂 Scanning templates in: {:?}", args.templates_dir);

    // for now return OK
    Ok(())
}

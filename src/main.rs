use clap::Parser;
use std::path::PathBuf;

// CLI arguments
#[derive(Parser, Debug)]
#[command(name = "sopsify")]
#[command(about = "Replace placeholders in YAML files and encrypt them with GPG", long_about = None)]
struct Args {
    // Path to the GPG key (e.g. gpg.asc)
    #[arg(short, long)]
    gpg_key: PathBuf,

    // Path to the secrets file (YAML format)
    #[arg(short, long)]
    secrets_file: PathBuf,

    // Folder containing secret templates
    #[arg(short, long)]
    templates_dir: PathBuf,

    // Output directory (where to write encrypted files)
    #[arg(short, long)]
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

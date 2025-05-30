use anyhow::Context;
use clap::Parser;
use regex::Regex;
use std::{collections::HashMap, fs, io::Write, path::PathBuf};
use walkdir::WalkDir;

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

    let output_dir = args
        .output_dir
        .unwrap_or_else(|| args.templates_dir.clone());

    process_templates(&args.templates_dir, &output_dir, &secrets)?;

    Ok(())
}

fn load_secrets(path: &PathBuf) -> anyhow::Result<HashMap<String, String>> {
    let contents = fs::read_to_string(path)
        .with_context(|| format!("Failed to read secrets file at {:?}", path))?;

    let secrets: HashMap<String, String> = serde_yaml::from_str(&contents)
        .with_context(|| format!("Failed to parse YAML in secrets file at {:?}", path))?;

    Ok(secrets)
}

fn process_templates(
    templates_dir: &PathBuf,
    output_dir: &PathBuf,
    secrets: &HashMap<String, String>,
) -> anyhow::Result<()> {
    let placeholder_regex = Regex::new(r"\$\{(\w+)\}")?;

    for entry in WalkDir::new(templates_dir)
        .into_iter()
        .filter_map(Result::ok)
    {
        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path();

        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read template file {:?}", path))?;

        let replaced = placeholder_regex.replace_all(&content, |caps: &regex::Captures| {
            let key = &caps[1];
            secrets.get(key).cloned().unwrap_or_else(|| {
                eprintln!("Warning: No secret found for placeholder '{}'", key);
                caps[0].to_string()
            })
        });

        // Determine output path
        let relative_path = path.strip_prefix(templates_dir)?;
        let output_path = output_dir.join(relative_path);
        fs::create_dir_all(output_path.parent().unwrap())?;

        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directories for {:?}", output_path))?;
        }

        let mut file = fs::File::create(&output_path)
            .with_context(|| format!("Failed to create output file {:?}", output_path))?;

        file.write_all(replaced.as_bytes())
            .with_context(|| format!("Failed to write to output file {:?}", output_path))?;

        println!("✅ Processed template: {:?}", output_path);
    }

    Ok(())
}

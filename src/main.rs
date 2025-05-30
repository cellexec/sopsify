use anyhow::Context;
use clap::Parser;
use regex::Regex;
use std::process::Command;
use std::{collections::HashMap, fs, io::Write, path::PathBuf};
use tempfile::NamedTempFile;
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
    gpg_key: String,

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

    process_templates(&args.templates_dir, &output_dir, &secrets, &args.gpg_key)?;

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
    gpg_key: &String,
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

        // Write replaced content to temp file
        let mut temp_file =
            NamedTempFile::new().context("Failed to create temporary file for encryption")?;
        write!(temp_file, "{}", replaced)?;

        // Determine output path
        let relative_path = path.strip_prefix(templates_dir)?;
        let output_path = output_dir.join(relative_path);

        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directories for {:?}", output_path))?;
        }

        let status = Command::new("sops")
            .arg("--encrypt")
            .arg("--output-type")
            .arg("yaml")
            .arg("--pgp")
            .arg(gpg_key)
            .arg(temp_file.path())
            .output()
            .context("Failed to execute sops command")?;

        if !status.status.success() {
            return Err(anyhow::anyhow!(
                "SOPS encryption failed:\n{}",
                String::from_utf8_lossy(&status.stderr)
            ));
        }

        let encrypted_output_path = output_path;
        fs::write(&encrypted_output_path, &status.stdout).with_context(|| {
            format!("Failed to write encrypted file {:?}", encrypted_output_path)
        })?;

        println!("🔐 Encrypted and saved: {:?}", encrypted_output_path);
    }

    Ok(())
}

use serde_yaml::Value;

fn replace_placeholders_in_value(
    value: &mut Value,
    secrets: &HashMap<String, String>,
    placeholder_regex: &Regex,
) {
    match value {
        Value::String(s) => {
            let replaced = placeholder_regex.replace_all(s, |caps: &regex::Captures| {
                let key = &caps[1];
                secrets
                    .get(key)
                    .cloned()
                    .unwrap_or_else(|| caps[0].to_string())
            });
            *s = replaced.into_owned();
        }
        Value::Mapping(map) => {
            for (_, v) in map.iter_mut() {
                replace_placeholders_in_value(v, secrets, placeholder_regex);
            }
        }
        Value::Sequence(seq) => {
            for v in seq.iter_mut() {
                replace_placeholders_in_value(v, secrets, placeholder_regex);
            }
        }
        _ => {} // no placeholders in other types
    }
}

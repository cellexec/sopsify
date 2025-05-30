use anyhow::Context;
use base64::engine::{Engine as _, general_purpose};
use clap::Parser;
use log::{error, info};
use regex::Regex;
use serde_yaml::Value;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use walkdir::WalkDir;

#[derive(Parser, Debug)]
#[command(name = "sopsify")]
#[command(
    about = "Replace placeholders in YAML files and encrypt them with GPG",
    long_about = "Sopsify replaces ${placeholders} in YAML manifests with real values
from a central YAML secrets file and encrypts the output using a GPG key.
Designed for GitOps workflows using SOPS + Flux.

IMPORTANT: This tool expects to be run from the root of your GitOps repository,
where your .sops.yaml file is located. All paths must be relative to this root."
)]
struct Args {
    #[arg(short, long, help = "GPG key fingerprint or ID to use for encryption")]
    gpg_key: String,

    #[arg(
        short,
        long,
        help = "Path to the secrets file (YAML format, relative to repo root)"
    )]
    secrets_file: PathBuf,

    #[arg(
        short,
        long,
        help = "Folder containing secret templates (relative to repo root)"
    )]
    templates_dir: PathBuf,

    #[arg(
        short,
        long,
        help = "Output directory (relative to repo root, defaults to templates_dir)"
    )]
    output_dir: Option<PathBuf>,
}

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let args = Args::parse();

    info!("⚙️ Passed Arguments");
    info!("    > GPG key: {}", args.gpg_key);
    info!("    > Secrets file: {:?}", args.secrets_file);
    info!("    > Templates directory: {:?}", args.templates_dir);

    let sops_config_file = PathBuf::from(".sops.yaml");
    if !sops_config_file.exists() {
        error!(
            "Error: '.sops.yaml' not found in the current directory. Please run this program from the root of your GitOps repository."
        );
        return Err(anyhow::anyhow!(
            "Error: '.sops.yaml' not found in the current directory. Please run this program from the root of your GitOps repository."
        ));
    }
    info!("✅ Found .sops.yaml in current directory.");

    if !args.secrets_file.exists() {
        error!("Secrets file {:?} does not exist.", args.secrets_file);
        return Err(anyhow::anyhow!(
            "Secrets file {:?} does not exist.",
            args.secrets_file
        ));
    }
    if !args.templates_dir.exists() {
        error!(
            "Templates directory {:?} does not exist.",
            args.templates_dir
        );
        return Err(anyhow::anyhow!(
            "Templates directory {:?} does not exist.",
            args.templates_dir
        ));
    }

    let secrets = load_secrets(&args.secrets_file)?;
    info!("✅ Loaded secrets: {:?}", secrets.keys());

    let output_dir = args
        .output_dir
        .clone()
        .unwrap_or_else(|| args.templates_dir.clone());

    process_templates(&args.templates_dir, &output_dir, &secrets, &args.gpg_key)?;

    Ok(())
}

fn load_secrets(path: &Path) -> anyhow::Result<HashMap<String, String>> {
    let contents = fs::read_to_string(path)
        .with_context(|| format!("Failed to read secrets file at {:?}", path))?;

    let secrets: HashMap<String, String> = serde_yaml::from_str(&contents)
        .with_context(|| format!("Failed to parse YAML in secrets file at {:?}", path))?;

    Ok(secrets)
}

fn process_templates(
    templates_dir: &Path,
    output_dir: &Path,
    secrets: &HashMap<String, String>,
    gpg_key: &str,
) -> anyhow::Result<()> {
    let placeholder_regex = Regex::new(r"\$\{(\w+)\}")?;

    for entry in WalkDir::new(templates_dir)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        // Optional: Filter only YAML files
        .filter(|e| {
            if let Some(ext) = e.path().extension() {
                ext == "yaml" || ext == "yml"
            } else {
                false
            }
        })
    {
        let path = entry.path();
        info!("Processing template: {:?}", path);

        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read template file {:?}", path))?;

        let mut yaml_value: Value = serde_yaml::from_str(&content)
            .with_context(|| format!("Failed to parse YAML in {:?}", path))?;

        replace_placeholders_in_value(&mut yaml_value, secrets, &placeholder_regex, None);

        let replaced_yaml_string =
            serde_yaml::to_string(&yaml_value).context("Failed to serialize replaced YAML")?;

        let relative_path = path.strip_prefix(templates_dir)?;
        let output_path = output_dir.join(relative_path);

        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directories for {:?}", output_path))?;
        }

        fs::write(&output_path, replaced_yaml_string)
            .with_context(|| format!("Failed to write pre-encryption YAML to {:?}", output_path))?;

        // Encrypt with sops
        let status = Command::new("sops")
            .arg("--encrypt")
            .arg("--in-place")
            .arg("--pgp")
            .arg(gpg_key)
            .arg(&output_path)
            .output()
            .context("Failed to execute sops command")?;

        if !status.status.success() {
            error!(
                "SOPS encryption failed for {:?}:\n{}",
                output_path,
                String::from_utf8_lossy(&status.stderr)
            );
            return Err(anyhow::anyhow!(
                "SOPS encryption failed for {:?}:\n{}",
                output_path,
                String::from_utf8_lossy(&status.stderr)
            ));
        }

        info!("🔐 Encrypted and saved: {:?}", output_path);
    }

    Ok(())
}

/// Recursively replace placeholders in the YAML `Value`.
///
/// Only base64-encodes secret values if they appear under `data:` or `stringData:` keys,
/// otherwise inserts the plaintext secret value.
///
/// `parent_key` tracks the key name in the parent mapping for context.
fn replace_placeholders_in_value(
    value: &mut Value,
    secrets: &HashMap<String, String>,
    placeholder_regex: &Regex,
    parent_key: Option<&str>,
) {
    match value {
        Value::String(s) => {
            let replaced = placeholder_regex.replace_all(s, |caps: &regex::Captures| {
                let key = &caps[1];
                if let Some(secret_value) = secrets.get(key) {
                    // Base64 encode only inside `data` or `stringData` fields
                    if matches!(parent_key, Some("data") | Some("stringData")) {
                        general_purpose::STANDARD.encode(secret_value)
                    } else {
                        secret_value.clone()
                    }
                } else {
                    // If no secret found, keep original placeholder
                    caps[0].to_string()
                }
            });
            *s = replaced.into_owned();
        }
        Value::Mapping(map) => {
            for (k, v) in map.iter_mut() {
                let next_parent_key = if let Value::String(key_str) = k {
                    Some(key_str.as_str())
                } else {
                    None
                };
                replace_placeholders_in_value(v, secrets, placeholder_regex, next_parent_key);
            }
        }
        Value::Sequence(seq) => {
            for v in seq.iter_mut() {
                replace_placeholders_in_value(v, secrets, placeholder_regex, None);
            }
        }
        _ => {}
    }
}

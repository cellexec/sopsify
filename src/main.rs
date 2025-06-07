use anyhow::Context;
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

    if !PathBuf::from(".sops.yaml").exists() {
        return Err(anyhow::anyhow!(".sops.yaml not found in current directory"));
    }

    if !args.secrets_file.exists() {
        return Err(anyhow::anyhow!(
            "Secrets file {:?} does not exist.",
            args.secrets_file
        ));
    }

    if !args.templates_dir.exists() {
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
        .filter(|e| {
            e.path()
                .extension()
                .map_or(false, |ext| ext == "yaml" || ext == "yml")
        })
    {
        let path = entry.path();
        info!("📄 Processing template: {:?}", path);

        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read template file {:?}", path))?;

        let mut yaml_value: Value = serde_yaml::from_str(&content)
            .with_context(|| format!("Failed to parse YAML in {:?}", path))?;

        replace_placeholders_in_value(&mut yaml_value, secrets, &placeholder_regex);

        let replaced_yaml_string =
            serde_yaml::to_string(&yaml_value).context("Failed to serialize replaced YAML")?;

        let relative_path = path.strip_prefix(templates_dir)?;
        let mut new_file_name = relative_path.file_stem().unwrap_or_default().to_os_string();
        new_file_name.push(".enc.yaml");
        let output_path = output_dir.join(relative_path.with_file_name(new_file_name));

        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directories for {:?}", output_path))?;
        }

        fs::write(&output_path, replaced_yaml_string)
            .with_context(|| format!("Failed to write replaced YAML to {:?}", output_path))?;

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

fn replace_placeholders_in_value(
    value: &mut Value,
    secrets: &HashMap<String, String>,
    placeholder_regex: &Regex,
) {
    match value {
        Value::String(s) => {
            let replaced = placeholder_regex.replace_all(s, |caps: &regex::Captures| {
                secrets
                    .get(&caps[1])
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
        _ => {}
    }
}

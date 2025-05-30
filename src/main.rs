use anyhow::Context;
use base64::engine::{Engine as _, general_purpose};
use clap::Parser;
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
\nIMPORTANT: This tool expects to be run from the root of your GitOps repository,
where your .sops.yaml file is located. All paths must be relative to this root."
)]
struct Args {
    #[arg(short, long, help = "Path to the GPG key (e.g. gpg.asc)")]
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
    // Removed: sops_config argument
}

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let args = Args::parse();
    println!("⚙️ Passed Arguments");
    println!("    > GPG key: {:?}", args.gpg_key);
    println!("    > Secrets file: {:?}", args.secrets_file);
    println!("    > Templates directory: {:?}", args.templates_dir);
    println!("");

    // --- NEW: Check for .sops.yaml in current directory ---
    let sops_config_file = PathBuf::from(".sops.yaml");
    if !sops_config_file.exists() {
        return Err(anyhow::anyhow!(
            "Error: '.sops.yaml' not found in the current directory. Please run this program from the root of your GitOps repository."
        ));
    }
    println!("✅ Found .sops.yaml in current directory.");
    // --- END NEW ---

    let secrets = load_secrets(&args.secrets_file)?;
    println!("✅ Loaded secrets: {:#?}", secrets.keys());

    let output_dir = args
        .output_dir
        .unwrap_or_else(|| args.templates_dir.clone());

    // Removed sops_config_path from the call
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
    // Removed sops_config_path parameter
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

        let mut yaml_value: Value = serde_yaml::from_str(&content)
            .with_context(|| format!("Failed to parse YAML in {:?}", path))?;

        replace_placeholders_in_value(&mut yaml_value, secrets, &placeholder_regex);

        let replaced_yaml_string =
            serde_yaml::to_string(&yaml_value).context("Failed to serialize replaced YAML")?;

        let relative_path = path.strip_prefix(templates_dir)?;
        let output_path = output_dir.join(relative_path);

        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directories for {:?}", output_path))?;
        }

        fs::write(&output_path, replaced_yaml_string)
            .with_context(|| format!("Failed to write pre-encryption YAML to {:?}", output_path))?;

        let mut command = Command::new("sops");
        command
            .arg("--encrypt")
            .arg("--in-place")
            .arg("--pgp")
            .arg(gpg_key);

        // Removed the --config argument logic, sops will now find .sops.yaml automatically
        // in the current directory or its ancestors.

        command.arg(&output_path);

        let status = command.output().context("Failed to execute sops command")?;

        if !status.status.success() {
            return Err(anyhow::anyhow!(
                "SOPS encryption failed for {:?}:\n{}",
                output_path,
                String::from_utf8_lossy(&status.stderr)
            ));
        }

        println!("🔐 Encrypted and saved: {:?}", output_path);
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
                let key = &caps[1];
                if let Some(secret_value) = secrets.get(key) {
                    general_purpose::STANDARD.encode(secret_value)
                } else {
                    caps[0].to_string()
                }
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

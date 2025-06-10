use clap::{Arg, Command};
use regex::Regex;
use serde::Deserialize;
use serde_yaml::Value;
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    process::Command as ProcessCommand,
};

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum SecretValue {
    Single(String),
    Multiple(Vec<ScopedSecret>),
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct ScopedSecret {
    #[serde(default)]
    namespace: Option<String>,

    #[serde(default)]
    namespaces: Option<Vec<String>>,

    value: String,
}

type SopsifyConfig = HashMap<String, SecretValue>;

fn main() {
    let matches = Command::new("sopsify")
        .version("1.0.1")
        .about("Encrypts template files using sops with placeholders from .sopsify.yaml")
        .arg(
            Arg::new("file")
                .short('f')
                .long("file")
                .help("A single template file to encrypt")
                .value_name("FILE")
                .conflicts_with("templates"),
        )
        .arg(
            Arg::new("templates")
                .short('t')
                .long("templates")
                .help("A folder containing multiple template files to encrypt")
                .value_name("FOLDER")
                .conflicts_with("file"),
        )
        .arg(
            Arg::new("output")
                .short('o')
                .long("output")
                .help("Optional output directory for encrypted files")
                .value_name("OUTPUT_DIR"),
        )
        .get_matches();

    let mut template_files = Vec::new();

    // Read templates
    if let Some(file) = matches.get_one::<String>("file") {
        let path = PathBuf::from(file);
        if path.is_file() {
            template_files.push(path);
        } else {
            eprintln!("Error: --file path is not a valid file.");
            std::process::exit(1);
        }
    } else if let Some(folder) = matches.get_one::<String>("templates") {
        let dir = PathBuf::from(folder);
        if dir.is_dir() {
            for entry in fs::read_dir(dir).expect("Failed to read directory") {
                let entry = entry.expect("Failed to read file in directory");
                let path = entry.path();
                if path.is_file() {
                    template_files.push(path);
                }
            }
        } else {
            eprintln!("Error: --templates path is not a valid directory.");
            std::process::exit(1);
        }
    } else {
        eprintln!("Error: either --file or --templates must be provided.");
        std::process::exit(1);
    }

    let output_root = matches
        .get_one::<String>("output")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("output"));

    // Load config
    let config_content = fs::read_to_string(".sopsify.yaml").expect("Failed to read .sopsify.yaml");
    let config: SopsifyConfig =
        serde_yaml::from_str(&config_content).expect("Invalid .sopsify.yaml format");

    let namespaces = collect_all_namespaces(&config);

    for namespace in &namespaces {
        let vars = extract_namespace_vars(&config, namespace);

        for template_path in &template_files {
            let content = fs::read_to_string(template_path)
                .unwrap_or_else(|_| panic!("Failed to read template: {}", template_path.display()));

            let rendered = render_template(&content, &vars);

            let filename = template_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("output");

            let output_dir = output_root.join(namespace);
            fs::create_dir_all(&output_dir).expect("Failed to create output namespace directory");

            let output_path = output_dir.join(format!("{}.enc.yaml", filename));
            let tmp_path = output_dir.join(format!("{}.tmp.yaml", filename));

            fs::write(&tmp_path, &rendered).expect("Failed to write temporary file");

            let status = ProcessCommand::new("sops")
                .arg("--encrypt")
                .arg("--output")
                .arg(&output_path)
                .arg(&tmp_path)
                .status()
                .expect("Failed to run sops");

            if !status.success() {
                eprintln!(
                    "sops encryption failed for file: {} in namespace: {}",
                    template_path.display(),
                    namespace
                );
                std::process::exit(1);
            }

            fs::remove_file(&tmp_path).ok();
            println!("Encrypted: {}", output_path.display());
        }
    }
}

fn collect_all_namespaces(config: &SopsifyConfig) -> HashSet<String> {
    let mut namespaces = HashSet::new();

    for value in config.values() {
        match value {
            SecretValue::Single(_) => {
                namespaces.insert("default".to_string());
            }
            SecretValue::Multiple(list) => {
                for entry in list {
                    if let Some(ns) = &entry.namespace {
                        namespaces.insert(ns.clone());
                    }
                    if let Some(nss) = &entry.namespaces {
                        for ns in nss {
                            namespaces.insert(ns.clone());
                        }
                    }
                }
            }
        }
    }

    namespaces
}

fn extract_namespace_vars(config: &SopsifyConfig, namespace: &str) -> HashMap<String, String> {
    let mut vars = HashMap::new();
    for (key, value) in config {
        match value {
            SecretValue::Single(val) => {
                vars.insert(key.clone(), val.clone());
            }
            SecretValue::Multiple(entries) => {
                for entry in entries {
                    let mut applies = false;

                    if let Some(ns) = &entry.namespace {
                        if ns == namespace {
                            applies = true;
                        }
                    }

                    if let Some(nss) = &entry.namespaces {
                        if nss.contains(&namespace.to_string()) {
                            applies = true;
                        }
                    }

                    if applies {
                        vars.insert(key.clone(), entry.value.clone());
                    }
                }
            }
        }
    }
    vars
}

fn render_template(template: &str, vars: &HashMap<String, String>) -> String {
    let re = Regex::new(r"\$\{(\w+)\}").unwrap();
    re.replace_all(template, |caps: &regex::Captures| {
        let key = &caps[1];
        vars.get(key)
            .cloned()
            .unwrap_or_else(|| caps[0].to_string())
    })
    .to_string()
}

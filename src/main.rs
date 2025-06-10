use clap::{Arg, Command};
use regex::Regex;
use serde::Deserialize;
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::PathBuf,
    process::Command as ProcessCommand,
};

#[derive(Debug, Deserialize)]
struct ScopedSecret {
    namespaces: Vec<String>,
    value: String,
}

type SopsifyConfig = HashMap<String, Vec<ScopedSecret>>;

fn main() {
    let matches = Command::new("sopsify")
        .version("1.0.3")
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

    let template_files = match read_template_files(&matches) {
        Ok(files) => files,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };

    let output_root = matches
        .get_one::<String>("output")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("output"));

    let config_content = fs::read_to_string(".sopsify.yaml").expect("Failed to read .sopsify.yaml");
    let config: SopsifyConfig = match serde_yaml::from_str(&config_content) {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("\u{274c} Failed to parse .sopsify.yaml:\n{e}");
            std::process::exit(1);
        }
    };

    let namespaces = collect_all_namespaces(&config);

    for namespace in &namespaces {
        let vars = extract_namespace_vars(&config, namespace);

        for template_path in &template_files {
            let content = fs::read_to_string(template_path)
                .unwrap_or_else(|_| panic!("Failed to read template: {}", template_path.display()));

            let rendered = render_template(&content, &vars);
            let missing_vars = find_missing_placeholders(&rendered);
            let provided_vars: HashSet<_> = vars.keys().cloned().collect();
            let unresolved: Vec<_> = missing_vars
                .iter()
                .filter(|var| !provided_vars.contains(*var))
                .cloned()
                .collect();

            if !unresolved.is_empty() {
                // Skip this file for this namespace
                continue;
            }

            // Inject namespace into rendered content if key 'namespace' exists
            let mut final_rendered = rendered.clone();
            if rendered.contains("${namespace}") {
                final_rendered = final_rendered.replace("${namespace}", namespace);
            } else if content.contains("namespace:") {
                let replaced = Regex::new(r"namespace:\s*\S+").unwrap();
                if replaced.is_match(&final_rendered) {
                    eprintln!(
                        "\u{26a0}\u{fe0f} Warning: 'namespace' was defined in template and will be overridden for {}",
                        template_path.display()
                    );
                    final_rendered = replaced.replace_all(&final_rendered, format!("namespace: {}", namespace)).to_string();
                }
            }

            let filename = template_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("output");

            let output_dir = output_root.join(namespace);
            fs::create_dir_all(&output_dir).expect("Failed to create output namespace directory");

            let output_path = output_dir.join(format!("{}.enc.yaml", filename));
            let tmp_path = output_dir.join(format!("{}.tmp.yaml", filename));

            fs::write(&tmp_path, &final_rendered).expect("Failed to write temporary file");

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
            println!("\u{2705} Encrypted: {}", output_path.display());
        }
    }
}

fn read_template_files(matches: &clap::ArgMatches) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();

    if let Some(file) = matches.get_one::<String>("file") {
        let path = PathBuf::from(file);
        if path.is_file() {
            files.push(path);
        } else {
            return Err("\u{274c} --file path is not a valid file.".into());
        }
    } else if let Some(folder) = matches.get_one::<String>("templates") {
        let dir = PathBuf::from(folder);
        if dir.is_dir() {
            for entry in fs::read_dir(dir).map_err(|_| "\u{274c} Failed to read directory")? {
                let path = entry.map_err(|_| "\u{274c} Failed to read entry")?.path();
                if path.is_file() {
                    files.push(path);
                }
            }
        } else {
            return Err("\u{274c} --templates path is not a valid directory.".into());
        }
    }

    Ok(files)
}

fn collect_all_namespaces(config: &SopsifyConfig) -> HashSet<String> {
    let mut namespaces = HashSet::new();
    for entries in config.values() {
        for entry in entries {
            namespaces.extend(entry.namespaces.clone());
        }
    }
    namespaces
}

fn extract_namespace_vars(config: &SopsifyConfig, namespace: &str) -> HashMap<String, String> {
    let mut vars = HashMap::new();
    for (key, entries) in config {
        for entry in entries {
            if entry.namespaces.contains(&namespace.to_string()) {
                vars.insert(key.clone(), entry.value.clone());
            }
        }
    }
    vars
}

fn render_template(template: &str, vars: &HashMap<String, String>) -> String {
    let re = Regex::new(r"\$\{(\w+)}").unwrap();
    re.replace_all(template, |caps: &regex::Captures| {
        let key = &caps[1];
        vars.get(key)
            .cloned()
            .unwrap_or_else(|| caps[0].to_string())
    })
    .to_string()
}

fn find_missing_placeholders(rendered: &str) -> Vec<String> {
    let re = Regex::new(r"\$\{(\w+)}").unwrap();
    let mut missing = HashSet::new();
    for caps in re.captures_iter(rendered) {
        missing.insert(caps[1].to_string());
    }
    missing.into_iter().collect()
}


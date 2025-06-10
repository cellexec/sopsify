use clap::{Arg, Command};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::PathBuf,
    process::Command as ProcessCommand,
};

#[derive(Debug, Deserialize)]
struct ScopedSecret {
    #[serde(default)]
    namespaces: Vec<String>, // MUST be present for every secret entry

    value: String,
}

#[derive(Debug, Deserialize)]
struct SopsifyConfig(HashMap<String, Vec<ScopedSecret>>);

type SopsifyConfigMap = HashMap<String, Vec<ScopedSecret>>;

fn main() {
    let matches = Command::new("sopsify")
        .version("1.0.2")
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

    // Deserialize enforcing every secret value must be an array of scoped secrets
    let config: SopsifyConfigMap = match serde_yaml::from_str(&config_content) {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("❌ Failed to parse .sopsify.yaml:\n{e}");
            std::process::exit(1);
        }
    };

    // Validate no empty namespaces and all namespaces present
    for (key, scoped_list) in &config {
        for entry in scoped_list {
            if entry.namespaces.is_empty() {
                eprintln!(
                    "❌ Secret '{}' has an entry with empty 'namespaces' list — all secrets must define namespaces explicitly",
                    key
                );
                std::process::exit(1);
            }
        }
    }

    let namespaces = collect_all_namespaces(&config);

    for namespace in &namespaces {
        let vars = extract_namespace_vars(&config, namespace);

        for template_path in &template_files {
            let content = fs::read_to_string(template_path)
                .unwrap_or_else(|_| panic!("Failed to read template: {}", template_path.display()));

            let rendered = render_template(&content, &vars);

            let missing_vars = find_missing_placeholders(&rendered);
            let provided_vars: HashSet<_> = vars.keys().cloned().collect();

            // partial missing vars logic — only error if some placeholders partially defined
            let present_vars: Vec<_> = missing_vars
                .iter()
                .filter(|var| provided_vars.contains(*var))
                .cloned()
                .collect();

            let truly_missing: Vec<_> = missing_vars
                .into_iter()
                .filter(|var| !provided_vars.contains(var))
                .collect();

            if !present_vars.is_empty() && !truly_missing.is_empty() {
                eprintln!(
                    "❌ Some placeholders in file '{}' are partially defined for namespace '{}'. Missing: {:?}",
                    template_path.display(),
                    namespace,
                    truly_missing
                );
                std::process::exit(1);
            }

            // Parse YAML rendered content so we can inject or override namespace key
            let mut yaml_value: serde_yaml::Value = match serde_yaml::from_str(&rendered) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!(
                        "❌ Failed to parse rendered YAML for '{}': {}",
                        template_path.display(),
                        e
                    );
                    std::process::exit(1);
                }
            };

            // Inject or override `namespace` key at root level
            if let serde_yaml::Value::Mapping(map) = &mut yaml_value {
                if let Some(existing_ns) = map.get(&serde_yaml::Value::String("namespace".into())) {
                    eprintln!(
                        "⚠️ Warning: overriding existing 'namespace' key in file '{}' for namespace '{}'",
                        template_path.display(),
                        namespace
                    );
                }
                map.insert(
                    serde_yaml::Value::String("namespace".into()),
                    serde_yaml::Value::String(namespace.clone()),
                );
            } else {
                eprintln!(
                    "❌ Expected YAML root to be a mapping/object in file '{}'",
                    template_path.display()
                );
                std::process::exit(1);
            }

            let new_rendered =
                serde_yaml::to_string(&yaml_value).expect("Failed to serialize YAML");

            let filename = template_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("output");

            let output_dir = output_root.join(namespace);
            fs::create_dir_all(&output_dir).expect("Failed to create output namespace directory");

            let output_path = output_dir.join(format!("{}.enc.yaml", filename));
            let tmp_path = output_dir.join(format!("{}.tmp.yaml", filename));

            fs::write(&tmp_path, &new_rendered).expect("Failed to write temporary file");

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
            println!("✅ Encrypted: {}", output_path.display());
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
            return Err("❌ --file path is not a valid file.".into());
        }
    } else if let Some(folder) = matches.get_one::<String>("templates") {
        let dir = PathBuf::from(folder);
        if dir.is_dir() {
            for entry in fs::read_dir(dir).map_err(|_| "❌ Failed to read directory")? {
                let path = entry.map_err(|_| "❌ Failed to read entry")?.path();
                if path.is_file() {
                    files.push(path);
                }
            }
        } else {
            return Err("❌ --templates path is not a valid directory.".into());
        }
    } else {
        return Err("❌ Either --file or --templates must be specified.".into());
    }

    Ok(files)
}

fn collect_all_namespaces(config: &SopsifyConfigMap) -> HashSet<String> {
    let mut namespaces = HashSet::new();

    for scoped_list in config.values() {
        for entry in scoped_list {
            namespaces.extend(entry.namespaces.iter().cloned());
        }
    }

    namespaces
}

fn extract_namespace_vars(config: &SopsifyConfigMap, namespace: &str) -> HashMap<String, String> {
    let mut vars = HashMap::new();
    for (key, scoped_list) in config {
        for entry in scoped_list {
            if entry.namespaces.contains(&namespace.to_string()) {
                vars.insert(key.clone(), entry.value.clone());
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

fn find_missing_placeholders(rendered: &str) -> Vec<String> {
    let re = Regex::new(r"\$\{(\w+)\}").unwrap();
    let mut missing = HashSet::new();
    for caps in re.captures_iter(rendered) {
        missing.insert(caps[1].to_string());
    }
    missing.into_iter().collect()
}

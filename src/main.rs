use clap::{Arg, Command};
use regex::Regex;
use serde::Deserialize;
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    process::Command as ProcessCommand,
};

#[derive(Debug, Deserialize)]
struct SopsifyConfig(HashMap<String, String>);

fn main() {
    let matches = Command::new("sopsify")
        .version("1.0.0")
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

    // Handle input sources
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

    // Handle output directory
    let output_dir = matches.get_one::<String>("output").map(PathBuf::from);

    // Load .sopsify.yaml config
    let config_contents =
        fs::read_to_string(".sopsify.yaml").expect("Failed to read .sopsify.yaml");
    let config: SopsifyConfig =
        serde_yaml::from_str(&config_contents).expect("Invalid .sopsify.yaml format");

    for template_path in template_files {
        let content = fs::read_to_string(&template_path)
            .unwrap_or_else(|_| panic!("Failed to read template: {}", template_path.display()));
        let rendered = render_template(&content, &config.0);

        let filename = template_path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("output");

        let output_filename = format!("{}.enc.yaml", filename);
        let output_path = output_dir
            .as_ref()
            .map(|dir| dir.join(&output_filename))
            .unwrap_or_else(|| Path::new(&output_filename).to_path_buf());

        // Write to temporary file
        let tmp_path = output_path.with_extension("tmp.yaml");
        fs::write(&tmp_path, &rendered).expect("Failed to write temporary file");

        // Encrypt with sops
        let status = ProcessCommand::new("sops")
            .arg("--encrypt")
            .arg("--output")
            .arg(&output_path)
            .arg(&tmp_path)
            .status()
            .expect("Failed to run sops");

        if !status.success() {
            eprintln!(
                "sops encryption failed for file: {}",
                template_path.display()
            );
            std::process::exit(1);
        }

        fs::remove_file(&tmp_path).ok(); // Clean up
        println!("Encrypted: {}", output_path.display());
    }
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

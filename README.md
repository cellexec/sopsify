sopsify: Flux Secrets Encryption Helper
sopsify is a Rust-based command-line tool designed to streamline the process of managing Kubernetes secrets in a GitOps workflow using SOPS and Flux. It replaces placeholders in YAML secret templates with actual values from a central secrets file and then encrypts the resulting Kubernetes Secret manifests using GPG via sops.

This tool is intended to be run from the root of your GitOps repository, where your .sops.yaml configuration file should reside.

Features
Placeholder Replacement: Dynamically replaces ${placeholder} values in your YAML templates with real secrets.

GPG Encryption: Encrypts the generated Kubernetes Secret manifests using your specified GPG key via sops.

SOPS Integration: Leverages sops's intelligent encryption to ensure only sensitive data or stringData fields are encrypted, leaving Kubernetes metadata (like apiVersion, kind, metadata.name, metadata.namespace, type) in plain text.

Directory Traversal: Processes all YAML files within a specified templates directory.

Prerequisites
Before using sopsify, ensure you have the following installed and configured:

Rust: The Rust programming language and Cargo package manager. Follow the instructions on rustup.rs.

SOPS: The sops command-line tool. Installation instructions can be found on the SOPS GitHub repository.

GnuPG (GPG): A GPG key pair configured on your system. sops will use this key for encryption and decryption.

Ensure your GPG key is imported and trusted.

You'll need the fingerprint of your GPG key. You can find it by running:

gpg --list-keys --fingerprint <your-key-id-or-email>


Look for the Key fingerprint line.

Setup
1. Clone the Repository (if applicable)
If sopsify is part of a larger project, clone it:

git clone <your-repo-url>
cd <your-repo-directory>


2. Compile sopsify
Navigate to the sopsify project directory (where Cargo.toml is located) and compile the binary:

cargo build --release


The executable will be located at target/release/sopsify.

3. Create Your .sops.yaml Configuration
This file tells sops how to encrypt your secrets. It must be placed in the root of your GitOps repository (the directory from which you will run sopsify).

Create a file named .sops.yaml with the following content:

# .sops.yaml
creation_rules:
  - path_regex: .*/secrets/.*\.yaml$ # Adjust this regex to match the paths of your output secret files
    unencrypted_regex: "^(apiVersion|kind|metadata|type)$"
    encrypted_regex: "^(data|stringData)$"
    pgp: "YOUR_GPG_KEY_FINGERPRINT" # REPLACE THIS with your actual GPG key fingerprint


Explanation of .sops.yaml:

creation_rules: Defines rules for sops when encrypting new files.

path_regex: A regular expression that sops uses to match file paths. Only files matching this regex will have these rules applied. Crucially, this regex should match the path where sopsify writes the output encrypted files.

Example: If your output files are in clusters/homelab/secrets/my-secret.yaml, then .*/secrets/.*\.yaml$ is a good match.

unencrypted_regex: A regex that tells sops which top-level keys not to encrypt. This ensures Kubernetes fields like apiVersion, kind, metadata, and type remain in plain text.

encrypted_regex: A regex that tells sops which top-level keys to encrypt. For Kubernetes Secrets, data and stringData are the fields containing sensitive information.

pgp: Replace "YOUR_GPG_KEY_FINGERPRINT" with the actual fingerprint of the GPG key you intend to use for encryption. This is how sops knows which key to use.

4. Prepare Your Secrets File
Create a central YAML file (e.g., secrets.yaml) containing your key-value pairs for secrets. This file is not encrypted by sopsify itself; it's just a source of values.

Example secrets.yaml:

# secrets.yaml
my_app_username: "admin"
my_app_password: "supersecretpassword"
woodpecker_namespace: "woodpecker-ci"
woodpecker_agent_secret_value: "agent-token-123"


5. Create Your Secret Templates
Create YAML files in your secret-templates directory that define the structure of your Kubernetes Secrets, using ${placeholder} for values that should come from your secrets.yaml.

Example clusters/homelab/secret-templates/woodpecker-agent-secret.yaml:

# clusters/homelab/secret-templates/woodpecker-agent-secret.yaml
apiVersion: v1
kind: Secret
metadata:
  name: woodpecker-agent-secret
  namespace: ${woodpecker_namespace} # Placeholder for namespace
type: Opaque
data:
  WOODPECKER_AGENT_SECRET: ${woodpecker_agent_secret_value} # Placeholder for secret value


Usage
IMPORTANT: You must run sopsify from the root of your GitOps repository, where your .sops.yaml file is located. All paths provided as arguments should be relative to this root.

# Navigate to your GitOps repository root
cd /path/to/your/flux-gitops-repo/

# Run sopsify
./target/release/sopsify \
    --gpg-key "YOUR_GPG_KEY_FINGERPRINT" \
    --secrets-file "secrets.yaml" \
    --templates-dir "clusters/homelab/secret-templates" \
    --output-dir "clusters/homelab/secrets"


Command-Line Arguments:
-g, --gpg-key <FINGERPRINT>: (Required) The fingerprint of the GPG key to use for encryption. This should match the pgp fingerprint in your .sops.yaml.

-s, --secrets-file <PATH>: (Required) Path to your central YAML secrets file (e.g., secrets.yaml), relative to the repository root.

-t, --templates-dir <PATH>: (Required) Path to the directory containing your YAML secret templates, relative to the repository root.

-o, --output-dir <PATH>: (Optional) Path to the directory where the encrypted secret files will be written, relative to the repository root. If not provided, encrypted files will be written back to the templates_dir.

Example Workflow:
Define your secrets in secrets.yaml.

Create secret templates in clusters/homelab/secret-templates using placeholders.

Ensure your .sops.yaml is in the repository root and configured correctly with your GPG fingerprint and path regex.

Run sopsify from the repository root.

sopsify will:

Read secrets.yaml.

Read each template from clusters/homelab/secret-templates.

Replace placeholders in the templates.

Write the plain-text (but value-filled) YAML to clusters/homelab/secrets.

Invoke sops --in-place on each of these files. sops will find the .sops.yaml in the current directory, apply the unencrypted_regex and encrypted_regex rules, encrypt the data fields, and add its own sops block.

After execution, your clusters/homelab/secrets directory will contain encrypted Kubernetes Secret manifests ready for use with Flux.

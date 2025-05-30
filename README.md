# 🔐 sopsify

`sopsify` is a CLI tool built for GitOps workflows using **SOPS** and **Flux**.  
It replaces `${placeholders}` in YAML manifest templates with real secret values from a central YAML secrets file, then encrypts the results using a GPG key.

---

## 🚀 Features

- 🔄 Replace `${KEY}` placeholders inside YAML templates  
- 🛡️ Base64-encode secret values **only** when under `data:` or `stringData:` keys (Kubernetes Secrets)  
- 🔐 Encrypt output manifests in-place with [SOPS](https://github.com/mozilla/sops) using your GPG key  
- 📁 Processes templates recursively, maintaining folder structure  
- 🏠 Designed to run from your GitOps repo root with `.sops.yaml` present  

---

## ⚙️ Installation

### Requirements

- Rust toolchain (if building from source)  
- [SOPS](https://github.com/mozilla/sops) installed and in your PATH  
- GPG installed and keys configured  

### Build from source

```bash
git clone https://github.com/yourusername/sopsify.git
cd sopsify
cargo build --release
````

The compiled binary will be located at `target/release/sopsify`.

---

## 📋 Usage

Run `sopsify` **from the root of your GitOps repo** where `.sops.yaml` is located:

```bash
sopsify \
  --gpg-key YOUR_GPG_KEY_FINGERPRINT \
  --secrets-file path/to/secrets.yaml \
  --templates-dir path/to/templates \
  [--output-dir path/to/output]
```

### 🧩 Arguments

| Argument                 | Description                                           | Required | Default                 |
| ------------------------ | ----------------------------------------------------- | -------- | ----------------------- |
| `--gpg-key` (`-g`)       | GPG key fingerprint or ID for encryption              | Yes      |                         |
| `--secrets-file` (`-s`)  | Path to YAML file with secret key-value pairs         | Yes      |                         |
| `--templates-dir` (`-t`) | Directory containing YAML templates with placeholders | Yes      |                         |
| `--output-dir` (`-o`)    | Directory to write processed files (relative path)    | No       | Same as `templates-dir` |

---

## 🗝️ Secrets File Format

A simple YAML with flat key-value pairs, e.g.:

```yaml
DB_PASSWORD: mysupersecretpassword
API_TOKEN: abcd1234
```

---

## 📝 Template Files

Templates contain placeholders like `${KEY}` matching keys in your secrets file.

Example:

```yaml
apiVersion: v1
kind: Secret
metadata:
  name: my-secret
data:
  password: ${DB_PASSWORD}
stringData:
  token: ${API_TOKEN}
```

`sopsify` replaces these placeholders with secrets — base64-encoding values under `data:` and `stringData:` automatically.

---

## ⚙️ How it works

1. 🔍 Loads secrets YAML file
2. 📂 Recursively scans templates directory for YAML files
3. 🧩 Parses each YAML file and replaces `${KEY}` placeholders
4. 🗃️ Writes replaced YAML files to output directory, preserving folder structure
5. 🔐 Runs SOPS encryption **in-place** on replaced files using your GPG key

---

## ⚠️ Notes

* Run from your GitOps repository root with `.sops.yaml` present.
* Ensure your GPG key is configured locally and accessible.
* Placeholders not found in the secrets file remain unchanged.
* The default output directory is the same as your templates directory — be careful to avoid overwriting originals.

---

## 🛠️ Troubleshooting

* **`.sops.yaml` missing:** Run from your GitOps root or add `.sops.yaml` config.
* **SOPS encryption errors:** Check your GPG key validity and permissions.
* **Placeholders not replaced:** Verify secrets keys match placeholders exactly (case-sensitive).

---

## 📄 License

MIT License

---

## 🤝 Contributing

Contributions are welcome! Open an issue or pull request on GitHub.

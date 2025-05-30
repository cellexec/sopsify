# sopsify

`sopsify` is a CLI tool designed for GitOps workflows that use SOPS and Flux. It replaces `${placeholders}` in YAML manifest templates with real secret values from a central YAML secrets file, and then encrypts the resulting manifests using a specified GPG key.

---

## Features

- Replace placeholders of the form `${KEY}` inside YAML templates.
- Base64-encode secret values **only** when they appear under `data:` or `stringData:` keys (common in Kubernetes Secret manifests).
- Encrypt resulting YAML manifests in-place with [SOPS](https://github.com/mozilla/sops) using your GPG key.
- Designed to be run from the root of your GitOps repo, where `.sops.yaml` config file is located.
- Supports recursive processing of YAML files in templates directories.
- Maintains directory structure when writing processed and encrypted files to output directory.

---

## Installation

### Requirements

- Rust toolchain (for building from source) or precompiled binary (if provided).
- [SOPS](https://github.com/mozilla/sops) installed and available in your PATH.
- GPG installed and configured with your encryption keys.

### Build from source

```bash
git clone https://github.com/yourusername/sopsify.git
cd sopsify
cargo build --release
````

The compiled binary will be at `target/release/sopsify`.

---

## Usage

Run `sopsify` from the root of your GitOps repository, where your `.sops.yaml` is located.

```bash
sopsify \
  --gpg-key YOUR_GPG_KEY_FINGERPRINT \
  --secrets-file path/to/secrets.yaml \
  --templates-dir path/to/templates \
  [--output-dir path/to/output]
```

### Arguments

| Argument                 | Description                                                 | Required | Default                 |
| ------------------------ | ----------------------------------------------------------- | -------- | ----------------------- |
| `--gpg-key` (`-g`)       | GPG key fingerprint or ID to use for encryption             | Yes      |                         |
| `--secrets-file` (`-s`)  | Path to YAML file containing secret key-value pairs         | Yes      |                         |
| `--templates-dir` (`-t`) | Directory containing YAML templates with `${PLACEHOLDER}`   | Yes      |                         |
| `--output-dir` (`-o`)    | Directory where processed files are written (relative path) | No       | Same as `templates-dir` |

---

## Secrets file format

The secrets file should be a YAML file containing flat key-value pairs. Example:

```yaml
DB_PASSWORD: mysupersecretpassword
API_TOKEN: abcd1234
```

---

## Template files

Your template YAML manifests should contain placeholders of the form `${KEY}` where `KEY` corresponds to keys in your secrets file.

Example template snippet:

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

`sopsify` will replace `${DB_PASSWORD}` and `${API_TOKEN}` with the corresponding secret values, base64-encoding only the values inside `data:` or `stringData:` keys.

---

## How it works

1. Loads the secrets YAML file into memory.
2. Recursively scans the templates directory for `.yaml` or `.yml` files.
3. For each file, parses the YAML content.
4. Replaces placeholders `${KEY}` with secret values.

   * Values under `data:` or `stringData:` are base64-encoded.
   * Values elsewhere are inserted as plaintext.
5. Writes the replaced YAML to the output directory, preserving folder structure.
6. Encrypts the output YAML files **in-place** using SOPS with the provided GPG key.

---

## Notes

* This tool expects to be run from the root of your GitOps repository where `.sops.yaml` exists. SOPS will use this config automatically.
* Make sure your GPG key is available locally for encryption.
* Placeholders not found in the secrets file remain unchanged.
* The output directory defaults to the templates directory if not specified, so be cautious to not overwrite originals unintentionally.

---

## Troubleshooting

* **`.sops.yaml` not found error**: Run `sopsify` from the repository root, or ensure `.sops.yaml` is present.
* **SOPS encryption errors**: Confirm that your GPG key is valid, available locally, and you have the right permissions.
* **Placeholders not replaced**: Check spelling and casing of keys in your secrets file and templates.

---

## License

MIT License

---

## Contributing

Contributions and issues are welcome! Please open a GitHub issue or pull request.


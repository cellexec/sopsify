# sopsify

Encrypt Kubernetes Secret templates per cluster and namespace using [SOPS](https://github.com/getsops/sops).

## Problem

Managing encrypted secrets across multiple Kubernetes clusters and namespaces is tedious.
The same secret template often needs different values per cluster (production vs staging)
and must be placed in the right folder for each namespace — all encrypted with SOPS.
Doing this manually means repeating the same steps for every cluster, namespace and template,
which is error-prone and hard to maintain.

## Solution

Sopsify takes a single configuration file (`.sopsify.yaml`) that declares which template
gets which values in which cluster and namespace. It renders the templates, encrypts them
with SOPS, and writes the output to the correct folder structure — in one command.

```
.sopsify.yaml + templates/     →  sopsify  →  clusters/{name}/secrets/{ns}/*.enc.yaml
(configuration)  (templates)       (CLI)       (encrypted secrets)
```

## Features

- Validates required config files: `.sops.yaml` and `.sopsify.yaml`
- Loads Kubernetes Secret YAML templates with `${PLACEHOLDER}` syntax
- Renders templates with namespace-specific values from configuration
- Encrypts secrets using `sops`
- Organizes output by cluster and namespace folders
- Validates configuration before generating (duplicate detection, missing values, unused keys)

## Installation

> [!NOTE]
> Make sure [SOPS](https://github.com/getsops/sops) is installed and available in your `PATH`.

```bash
sudo npm link
```

## Usage

```bash
sopsify -t <templates-folder>
```

- `-t, --templates <FOLDER>`: Folder containing your Secret YAML templates.

The tool expects `.sops.yaml` and `.sopsify.yaml` in the current working directory.

## Templates

Templates must be Kubernetes Secrets (`kind: Secret`) with placeholders in `data` or `stringData` fields:

```yaml
apiVersion: v1
kind: Secret
metadata:
  name: registry
data:
  registry: ${gitlab-registry}
  url: "static_value"
```

All placeholders must have corresponding values for each namespace in `.sopsify.yaml`.

## Configuration

### .sops.yaml

Standard SOPS configuration. Defines which fields to encrypt and which key to use:

```yaml
creation_rules:
  - path_regex: '.*\.yaml$'
    encrypted_regex: "^(data|stringData)$"
    pgp: A71EBC32B9144B2C231F7887405D5C987B44047E
```

### .sopsify.yaml

Maps clusters, templates and namespace-specific values:

> [!NOTE]
> The `template` filename must match a file in the folder passed via `sopsify -t <template_folder>`.

```yaml
sopsify:

  # Production cluster
  - production:
    - template: "app-secret.yaml"
      values:
        - key: api-token
          value: prodApiToken123
          namespaces: [frontend, backend]
        - key: db-password
          value: superSecurePass!
          namespaces: [frontend, backend]

  # Staging cluster
  - staging:
    - template: "app-secret.yaml"
      values:
        - key: api-token
          value: stagingTokenXYZ
          namespaces: [frontend, backend]
        - key: db-password
          value: stagingPass!
          namespaces: [frontend, backend]
```

### Value assignment rules

You can **reuse the same value for multiple namespaces** by listing them together:

```yaml
- key: api-token
  value: prodApiToken123
  namespaces: [frontend, backend]   # same value for both
```

You can **use different values per namespace** by repeating the key with different namespaces:

```yaml
- key: user-password
  value: adminPass
  namespaces: [frontend]            # one value for frontend
- key: user-password
  value: backendOnlyPass
  namespaces: [backend]             # different value for backend
```

But you **cannot define a key twice for the same namespace**:

```yaml
- key: user-password
  value: adminPass
  namespaces: [frontend, backend]
- key: user-password
  value: backendOnlyPass
  namespaces: [backend]             # ERROR: backend already defined above
```

## Output structure

Encrypted secrets are written to:

```
clusters/
  └── <cluster-name>/
      └── secrets/
          └── <namespace>/
              └── <template>.enc.yaml
```

## How it works

1. **Pre-checks** — verifies `.sops.yaml`, `.sopsify.yaml` and the `sops` binary exist
2. **Load templates** — reads all YAML files from the templates folder, validates they are `kind: Secret`
3. **Per cluster and template:**
   - Extracts all `${...}` placeholders from `data`/`stringData` fields
   - Validates that every placeholder has a value for every namespace
   - Per namespace: copies template, sets `metadata.namespace`, replaces placeholders
   - Writes rendered file to `clusters/{cluster}/secrets/{namespace}/`
   - Encrypts in-place via `sops -e -i`
   - Renames to `.enc.yaml`

## Error handling

| Scenario | Message |
|----------|---------|
| Missing config | `ENOENT: no such file or directory, open '.sops.yaml'` |
| Missing template file | `Template file not found for: app-secret.yaml` |
| Duplicate namespace for key | `Duplicate namespaces detected in key '...' for template '...'` |
| Same key twice for namespace | `Duplicate value for key '...' in namespace '...'` |
| Placeholder without value | `Placeholder 'api-token' in template '...' has no values defined` |
| Namespace missing for placeholder | `Key 'db-password' in template '...' is missing namespaces: backend` |
| Unused key in config | `Warning: key 'unused-key' is defined but not used in template '...'` |
| SOPS not installed | `sops is not installed or not in PATH` |
| Template not a Secret | `Error in 'file.yaml': Template is not of kind 'Secret'` |
| Template missing data | `Template '...' must contain 'data' or 'stringData'` |
| Cluster folder missing | `Cluster folder 'clusters/production' does not exist` |
| YAML parse error | Shows exact location with marker |

## License

MIT

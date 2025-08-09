# sopsify

Encrypt Kubernetes Secret templates per cluster and namespace using [sops](https://github.com/mozilla/sops).

---

## Features

- Validates required config files: `.sops.yaml` & `.sopsify.yaml`
- Loads Kubernetes Secret YAML templates with placeholders
- Renders templates with namespace-specific values
- Encrypts secrets using `sops`
- Organizes output by cluster and namespace folders

---

## Installation

> [!NOTE]
> Make sure [sops](https://github.com/mozilla/sops) is installed and available in your `PATH`.

```bash
sudo npm link
````

---

## Usage

```bash
sopsify -t <templates-folder>
```

* `-t, --templates <FOLDER>`: Folder containing your Secret YAML templates.

---

## Configuration Files

* `.sops.yaml` — sops config (see [sops docs](https://github.com/mozilla/sops#configuration))
* `.sopsify.yaml` — maps clusters, templates & namespace-specific values

### Example `.sopsify.yaml`

> [!NOTE]
> The `template` filename need to be organized in a folder that we later access with `sopsify -t <template_folder>`.

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

    - template: "user-secret.yaml"
      values:
        - key: user-name
          value: adminUser
          namespaces: [frontend, backend]
        - key: user-password
          value: adminPass
          namespaces: [frontend, backend]
        - key: user-password
          value: backendOnlyPass
          namespaces: [backend]

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

    - template: "user-secret.yaml"
      values:
        - key: user-name
          value: stagingUser
          namespaces: [frontend, backend]
        - key: user-password
          value: stagingPass123
          namespaces: [frontend, backend]
```

**Different Usage:**

* You can **reuse the same value for multiple namespaces** by listing them together:

  ```yaml
  - key: api-token
    value: prodApiToken123
    namespaces: [frontend, backend] # ✅ Valid to combine
  ```

* Or you can **use different values per namespace** by repeating the key with different namespaces:

  ```yaml
  - key: user-password
    value: adminPass
    namespaces: [frontend] # ✅ Valid to split
  - key: user-password
    value: backendOnlyPass
    namespaces: [backend]  # ✅ Valid to split
  ```

* But you **cannot define it multiple times**:

  ```yaml
  - key: user-password
    value: adminPass
    namespaces: [frontend,backend]
  - key: user-password
    value: backendOnlyPass
    namespaces: [backend] # ❌ ERROR: Already defined above
  ```

---

## Template Requirements

* Must be a Kubernetes Secret (`kind: Secret`)
* Placeholders in `data` or `stringData` fields using `${PLACEHOLDER}` syntax
* All placeholders must have corresponding values for each namespace in `.sopsify.yaml`

---

## Output Structure

Encrypted secrets will be saved in:

```
  ─ clusters
    └── <cluster-name>                  # Allow manage of multiple clusters
        └── secrets
            └── <namespace>             # Allow manage of multiple namespaces
                └── <template>.enc.yaml # Allow manage of multiple templates

```

---

## Error Handling & Warnings

| Error   | Logs    |
|--------------- | --------------- |
| Missing Configs   | `❌ ENOENT: no such file or directory, open '.sops.yaml'`   |
| | `❌ ENOENT: no such file or directory, open '.sopsify.yaml'`   |
| Missing Template   | `⚠️ Template file not found for: com-certificate.yaml` |
| Duplicate Namespaces   | Item2.3   |
| Missing Placeholders   | Item2.4   |
| Unused Keys   | Item2.4   |


Yaml parse errors:

```
❌ bad indentation of a mapping entry (4:3)

 1 | sopsify:
 2 |
 3 |   # Homelab
 4 |   @- homelab:
-------^
 5 |     - template: "certificates/towe ...
 6 |       values:

```

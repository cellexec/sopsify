# sopsify

Encrypt Kubernetes Secret templates per cluster and namespace using [sops](https://github.com/mozilla/sops).

---

## Features

- Validates required config files: `.sops.yaml` & `.sopsify.yaml`
- Loads Kubernetes Secret YAML templates with placeholders
- Renders templates with namespace-specific values
- Encrypts secrets in-place using `sops`
- Organizes output by cluster and namespace folders

---

## Installation

```bash
npm install -g sopsify
````

> [!INFO] SOPS is required!
> Make sure [sops](https://github.com/mozilla/sops) is installed and available in your `PATH`.

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

**Notes:**

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

* But you **cannot define it multiple times:

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
clusters/<cluster-name>/secrets/<namespace>/<template>.enc.yaml
```

---

## Error Handling & Warnings

* Missing config files or templates abort execution
* Duplicate namespaces or missing placeholder values cause errors
* Warns about unused keys in `.sopsify.yaml`


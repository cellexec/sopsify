# sopsify

Render and encrypt Kubernetes Secrets per cluster and namespace — in one command.

You define your secrets once in a `.sopsify.yaml`. Sopsify replaces placeholders
in your templates with the right values, encrypts them with [SOPS](https://github.com/getsops/sops),
and writes everything to the correct folder.

```
sopsify -t secrets/
```

## Quick start

> [!NOTE]
> Requires [SOPS](https://github.com/getsops/sops) in your `PATH`.

```bash
sudo npm link
```

## How it works

**1. You write a Secret template** with `${...}` placeholders:

```yaml
# secrets/app-secret.yaml
apiVersion: v1
kind: Secret
metadata:
  name: app
data:
  token: ${api-token}
```

**2. You define values per cluster and namespace** in `.sopsify.yaml`:

```yaml
sopsify:
  - production:
    - template: "app-secret.yaml"
      values:
        - key: api-token
          value: cHJvZF90b2tlbg==
          namespaces: [frontend, backend]

  - staging:
    - template: "app-secret.yaml"
      values:
        - key: api-token
          value: c3RhZ2luZ190b2tlbg==
          namespaces: [frontend]
```

**3. Sopsify generates encrypted files** in your cluster folder structure:

```
clusters/
  ├── production/secrets/
  │   ├── frontend/app-secret.enc.yaml
  │   └── backend/app-secret.enc.yaml
  └── staging/secrets/
      └── frontend/app-secret.enc.yaml
```

Each file has the placeholders replaced, `metadata.namespace` set, and is encrypted with SOPS.

## Value rules

Same value for multiple namespaces — list them together:

```yaml
- key: token
  value: abc
  namespaces: [frontend, backend]
```

Different values per namespace — repeat the key:

```yaml
- key: token
  value: abc
  namespaces: [frontend]
- key: token
  value: xyz
  namespaces: [backend]
```

A key cannot appear twice for the same namespace.

## License

MIT

# Klyster Helm Chart

This chart installs Klyster, a capacity planning and scaling recommendation platform for Kubernetes workloads.

## Install

```bash
helm repo add klyster https://klyster.github.io/klyster
helm repo update
helm install klyster klyster/klyster --namespace klyster --create-namespace
```

For a local checkout:

```bash
helm install klyster ./deploy/helm/klyster --namespace klyster --create-namespace
```

## Images

By default the chart uses GitHub Container Registry images:

- `ghcr.io/klyster/klyster`
- `ghcr.io/klyster/klyster-seer`

Override both repositories when publishing from a fork or private registry.

## Database Modes

SQLite is enabled by default for simple single-replica deployments. Enable persistence for durable local storage:

```yaml
database:
  type: sqlite
persistence:
  enabled: true
  size: 10Gi
```

For production and horizontal scaling, use PostgreSQL:

```yaml
database:
  type: postgres
  postgres:
    internal:
      enabled: false
    external:
      host: postgres.example.com
      port: 5432
      database: klyster
      username: klyster
      existingSecret: klyster-postgres
      existingSecretPasswordKey: password
replicaCount: 2
```

The chart can also run a small bundled PostgreSQL instance for evaluation:

```yaml
database:
  type: postgres
  postgres:
    internal:
      enabled: true
      auth:
        password: change-me
```

## Kubernetes Access

Klyster needs read access to Kubernetes workloads when Kubernetes integration is enabled. Cluster-wide RBAC is enabled by default. Use namespace-scoped RBAC for restricted installations:

```yaml
rbac:
  scope: namespace
klyster:
  kubernetes:
    namespaces:
      - default
      - production
```

## Monitoring

Prometheus metrics are exposed at `/metrics`. If the Prometheus Operator is installed, enable ServiceMonitor creation:

```yaml
serviceMonitor:
  enabled: true
```

## Release

Tagged releases package and publish the chart through the `helm-release.yml` workflow. The workflow publishes to GitHub Pages as an ordinary Helm repository.

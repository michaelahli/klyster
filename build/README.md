# Klyster Build & Deployment

This directory contains everything needed to build and deploy Klyster as a container.

## Quick Start

### 1. Build the Docker images
```bash
cd build
./build.sh           # builds both core and seer
# or build individually:
# ./build.sh core
# ./build.sh seer
```

### 2. Run standalone (SQLite)
```bash
docker run -p 8080:8080 klyster:latest
```

### 3. Run with docker-compose (PostgreSQL + Prometheus + Seer)
```bash
cd build
docker-compose up -d
```

## Directory Structure

```
build/
├── Dockerfile                  # Multi-stage build for Klyster (Rust)
├── Dockerfile.seer             # Build for Seer (Python analytics sidecar)
├── docker-compose.yml          # Full stack orchestration
├── build.sh                    # Build script (core | seer | all)
├── README.md                   # This file
├── config/
│   ├── klyster.toml           # Default config (SQLite)
│   └── klyster.postgres.toml  # PostgreSQL config
├── init/
│   └── init-db.sql            # Database initialization
├── prometheus/
│   └── prometheus.yml         # Prometheus configuration
└── grafana/
    └── datasources.yml        # Grafana datasource config
```

## Deployment Options

### Option 1: Standalone Container (SQLite)

Run Klyster as a single container with SQLite database:

```bash
docker run -d \
  --name klyster \
  -p 8080:8080 \
  -v klyster_data:/app/data \
  klyster:latest
```

**Use case**: Development, testing, single-node deployments

### Option 2: Docker Compose (PostgreSQL)

Run full stack with PostgreSQL and Prometheus:

```bash
cd build
docker-compose up -d
```

**Services**:
- Klyster API: http://localhost:8080
- PostgreSQL: localhost:5432
- Prometheus: http://localhost:9090

**Use case**: Production, multi-node deployments

### Option 3: Docker Compose with Monitoring

Include Grafana for visualization:

```bash
cd build
docker-compose --profile monitoring up -d
```

**Additional services**:
- Grafana: http://localhost:3000 (admin/admin)

**Use case**: Production with full observability

## Configuration

### Environment Variables

Override configuration using environment variables:

```bash
docker run -d \
  -e KLYSTER_DATABASE__DB_TYPE=postgres \
  -e KLYSTER_DATABASE__POSTGRES_URL=postgresql://user:pass@host:5432/db \
  -e KLYSTER_WEB__PORT=8080 \
  -e KLYSTER_LOGGING__LEVEL=debug \
  -p 8080:8080 \
  klyster:latest
```

### Config File

Mount custom config file:

```bash
docker run -d \
  -v /path/to/klyster.toml:/app/klyster.toml:ro \
  -p 8080:8080 \
  klyster:latest
```

### Database Options

**SQLite** (default):
```bash
-e KLYSTER_DATABASE__DB_TYPE=sqlite
-e KLYSTER_DATABASE__SQLITE_PATH=/app/data/klyster.db
-v klyster_data:/app/data
```

**PostgreSQL**:
```bash
-e KLYSTER_DATABASE__DB_TYPE=postgres
-e KLYSTER_DATABASE__POSTGRES_URL=postgresql://user:pass@host:5432/db
```

## Build Options

### Custom Image Name/Tag

```bash
IMAGE_NAME=myregistry/klyster IMAGE_TAG=v1.0.0 ./build.sh
```

### Build Arguments

```bash
docker build \
  -f build/Dockerfile \
  --build-arg RUST_VERSION=1.81 \
  -t klyster:custom \
  .
```

## Docker Compose Commands

### Start services
```bash
cd build
docker-compose up -d
```

### View logs
```bash
docker-compose logs -f klyster
docker-compose logs -f postgres
```

### Stop services
```bash
docker-compose stop
```

### Remove everything (including data)
```bash
docker-compose down -v
```

### Restart a service
```bash
docker-compose restart klyster
```

### Scale Klyster (requires load balancer)
```bash
docker-compose up -d --scale klyster=3
```

## Health Checks

### Check Klyster health
```bash
curl http://localhost:8080/healthz
```

### Check readiness
```bash
curl http://localhost:8080/readyz
```

### Check Prometheus targets
```bash
curl http://localhost:9090/api/v1/targets
```

## API Usage

### Create a metric source
```bash
curl -X POST http://localhost:8080/api/v1/sources \
  -H "Content-Type: application/json" \
  -d '{
    "name": "my_prometheus",
    "type": "prometheus",
    "config": {"url": "http://prometheus:9090"}
  }'
```

### List sources
```bash
curl http://localhost:8080/api/v1/sources | jq
```

### Query metrics
```bash
curl "http://localhost:8080/api/v1/metrics/cpu_usage?start=2026-05-26T00:00:00Z" | jq
```

## Troubleshooting

### Container won't start
```bash
# Check logs
docker logs klyster

# Check health
docker inspect klyster | jq '.[0].State.Health'
```

### Database connection issues
```bash
# Check PostgreSQL
docker-compose exec postgres pg_isready -U klyster

# Check connection from Klyster
docker-compose exec klyster curl http://localhost:8080/readyz
```

### Prometheus not scraping
```bash
# Check Prometheus targets
curl http://localhost:9090/api/v1/targets | jq

# Check Klyster metrics endpoint
curl http://localhost:8080/metrics
```

### Reset everything
```bash
cd build
docker-compose down -v
docker-compose up -d
```

## Production Deployment

### Recommended Settings

1. **Use PostgreSQL** for persistence
2. **Enable monitoring** with Prometheus + Grafana
3. **Set resource limits**:
   ```yaml
   deploy:
     resources:
       limits:
         cpus: '2'
         memory: 2G
       reservations:
         cpus: '1'
         memory: 1G
   ```
4. **Use secrets** for passwords (not environment variables)
5. **Enable TLS** with reverse proxy (nginx/traefik)
6. **Configure backups** for PostgreSQL
7. **Set up log aggregation** (ELK/Loki)

### Example Production Compose

```yaml
services:
  klyster:
    image: klyster:v1.0.0
    environment:
      KLYSTER_DATABASE__POSTGRES_URL: postgresql://klyster:${DB_PASSWORD}@postgres:5432/klyster
      KLYSTER_LOGGING__LEVEL: info
    deploy:
      replicas: 3
      resources:
        limits:
          cpus: '2'
          memory: 2G
    secrets:
      - db_password
```

## Security

### Non-root User

Container runs as non-root user `klyster` (UID 1000).

### Secrets Management

Use Docker secrets instead of environment variables:

```bash
echo "my_password" | docker secret create db_password -
```

### Network Isolation

Services communicate via internal network. Only expose necessary ports.

## Monitoring

### Prometheus Metrics

Available at `http://localhost:8080/metrics`:
- HTTP request metrics
- Database connection pool metrics
- Application metrics

### Grafana Dashboards

Access Grafana at http://localhost:3000 (admin/admin)

Import dashboards for:
- HTTP request rates and latency
- Database performance
- System resources

## Backup & Restore

### Backup PostgreSQL
```bash
docker-compose exec postgres pg_dump -U klyster klyster > backup.sql
```

### Restore PostgreSQL
```bash
docker-compose exec -T postgres psql -U klyster klyster < backup.sql
```

### Backup SQLite
```bash
docker cp klyster:/app/data/klyster.db ./backup.db
```

## Updates

### Update to new version
```bash
# Pull new image
docker pull klyster:v1.1.0

# Update docker-compose.yml with new tag
# Then restart
cd build
docker-compose up -d
```

### Rolling update (zero downtime)
```bash
docker-compose up -d --no-deps --scale klyster=2 klyster
docker-compose up -d --no-deps --scale klyster=1 klyster
```

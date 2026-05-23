# Klyster Docker Development Environment

This docker-compose setup provides a complete development and testing environment for Klyster.

## Services Included

### Core Services
- **PostgreSQL** (port 5432): Main database for distributed deployment
- **pgAdmin** (port 5050): Database management UI

### Observability Stack (for M2+)
- **Prometheus** (port 9090): Metrics collection and storage
- **Grafana** (port 3000): Visualization and dashboards
- **Jaeger** (port 16686): Distributed tracing

## Quick Start

### 1. Start All Services

```bash
docker-compose up -d
```

### 2. Verify Services

```bash
docker-compose ps
```

All services should show as "healthy" or "running".

### 3. Access UIs

- **Grafana**: http://localhost:3000 (admin/admin)
- **Prometheus**: http://localhost:9090
- **Jaeger**: http://localhost:16686
- **pgAdmin**: http://localhost:5050 (admin@klyster.local/admin)

### 4. Configure Klyster for PostgreSQL

Copy the example config:

```bash
cp klyster.postgres.toml klyster.toml
```

The default config connects to PostgreSQL at `localhost:5432`.

### 5. Run Klyster

```bash
cargo run
```

Or with specific components:

```bash
cargo run -- --web
cargo run -- --agent
cargo run -- --analytics
```

## Testing with PostgreSQL

### Run Migrations

Migrations run automatically on startup, but you can verify:

```bash
# Check database
docker-compose exec postgres psql -U klyster -d klyster -c "\dt"
```

You should see all tables:
- metrics, metric_labels, metric_sources
- resources, resource_groups, scaling_targets
- forecasts, forecast_points, recommendations
- analytics_functions

### Insert Test Data

```bash
# Connect to PostgreSQL
docker-compose exec postgres psql -U klyster -d klyster

# Check analytics functions (should have 4 predefined)
SELECT name, type FROM analytics_functions;

# Exit
\q
```

## Service Management

### Stop All Services

```bash
docker-compose down
```

### Stop and Remove Volumes (Clean Slate)

```bash
docker-compose down -v
```

### View Logs

```bash
# All services
docker-compose logs -f

# Specific service
docker-compose logs -f postgres
docker-compose logs -f prometheus
```

### Restart a Service

```bash
docker-compose restart postgres
```

## Database Connection Strings

### From Host (macOS/Linux)
```
postgresql://klyster:klyster_dev_password@localhost:5432/klyster
```

### From Docker Container
```
postgresql://klyster:klyster_dev_password@postgres:5432/klyster
```

## pgAdmin Setup

1. Open http://localhost:5050
2. Login: admin@klyster.local / admin
3. Add Server:
   - Name: Klyster Dev
   - Host: postgres
   - Port: 5432
   - Database: klyster
   - Username: klyster
   - Password: klyster_dev_password

## Prometheus Setup

Prometheus is pre-configured to scrape:
- Klyster metrics at `host.docker.internal:8080/metrics` (M2+)
- Self-monitoring

Config: `docker/prometheus.yml`

## Grafana Setup

Grafana is pre-configured with:
- Prometheus datasource
- Jaeger datasource
- PostgreSQL datasource

Dashboards will be added in M10.

## Troubleshooting

### PostgreSQL Connection Failed

```bash
# Check if PostgreSQL is running
docker-compose ps postgres

# Check logs
docker-compose logs postgres

# Restart
docker-compose restart postgres
```

### Port Already in Use

If ports are already in use, edit `docker-compose.yml` to change port mappings:

```yaml
ports:
  - "5433:5432"  # Change host port
```

### Reset Everything

```bash
# Stop and remove everything
docker-compose down -v

# Start fresh
docker-compose up -d
```

## Development Workflow

### 1. Start Infrastructure

```bash
docker-compose up -d postgres
```

### 2. Run Klyster

```bash
cargo run
```

### 3. Run Tests

```bash
# Unit tests (use in-memory SQLite)
cargo test

# Integration tests with PostgreSQL
KLYSTER_DATABASE__POSTGRES_URL="postgresql://klyster:klyster_dev_password@localhost:5432/klyster" cargo test
```

### 4. Check Logs

```bash
# Klyster logs (JSON format)
cargo run 2>&1 | jq

# PostgreSQL logs
docker-compose logs -f postgres
```

## Production-Like Testing

### Multi-Instance HA Test

Run multiple Klyster instances:

```bash
# Terminal 1
cargo run

# Terminal 2
KLYSTER_WEB__PORT=8081 cargo run -- --web

# Terminal 3
KLYSTER_WEB__PORT=8082 cargo run -- --web
```

All instances share the same PostgreSQL database (stateless design).

## Cleanup

### Remove All Data

```bash
docker-compose down -v
rm -rf klyster.toml
```

### Keep Configuration

```bash
docker-compose down
```

## Next Steps

- M2: Web API will expose `/metrics` endpoint for Prometheus
- M3: Prometheus integration for metric collection
- M10: Grafana dashboards for visualization

## Notes

- **Security**: This setup uses default passwords for development only
- **Performance**: PostgreSQL is configured for development, not production
- **Data**: All data is stored in Docker volumes and persists between restarts
- **Network**: All services are on the same Docker network for easy communication

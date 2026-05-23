# Klyster

**Capacity Planning Application for Kubernetes and VM Workloads**

Klyster is a capacity planning tool that analyzes infrastructure metrics, forecasts resource usage, and provides intelligent scaling recommendations for your Kubernetes clusters and VM environments.

## Features

- **Metric Collection**: Prometheus integration and agent-based collection
- **Forecasting**: Time-series prediction using multiple models (ARIMA, linear regression, seasonal decomposition)
- **Smart Recommendations**: Automated scaling suggestions based on predicted capacity needs
- **Flexible Deployment**: Single binary with SQLite or distributed with PostgreSQL
- **Observability**: Built-in metrics, structured logging, and distributed tracing
- **High Availability**: Stateless design for horizontal scaling

## Quick Start

### Installation

```bash
git clone https://github.com/klyster/klyster.git
cd klyster

cp klyster.postgres.toml klyster.toml

cargo build --release

./target/release/klyster
```

## Usage

### Run All Components

```bash
klyster
```

### Run Specific Components

```bash
# Web API only
klyster --web

# Agent only
klyster --agent

# Analytics only
klyster --analytics
```

### Configuration

See `klyster.example.toml` for full configuration options.

## Architecture

Klyster is built as a modular monolith with the following components:

- **Web API**: REST API for managing resources and viewing recommendations
- **Agent**: Collects metrics from infrastructure
- **Analytics**: Python-based forecasting engine with ML models
- **Database**: SQLite (single instance) or PostgreSQL (distributed)

All components can run together in a single binary or separately for distributed deployment.

## License

MIT

## Authors

Klyster Contributors

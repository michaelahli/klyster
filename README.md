# Klyster

> Capacity planning application for Kubernetes and VMs with ML-powered forecasting

**Status**: 🚧 Planning Complete, Implementation Starting

---

## Quick Start for Contributors

**New to this project?** Start here:

1. Read [`docs/PROJECT_STATE.md`](docs/PROJECT_STATE.md) — current state and next steps
2. Review [`docs/PRD.md`](docs/PRD.md) — product requirements
3. Check [`docs/tickets/README.md`](docs/tickets/README.md) — implementation tickets

**Resuming work?** Go straight to [`docs/PROJECT_STATE.md`](docs/PROJECT_STATE.md)

---

## What is Klyster?

Klyster analyzes infrastructure metrics and provides intelligent scaling recommendations:

- 📊 **Collect** metrics from Prometheus or built-in agents
- 🤖 **Analyze** trends using predefined or custom Python functions
- 📈 **Forecast** resource needs (days, weeks, months ahead)
- ✅ **Recommend** scaling actions with confidence scores
- 🎯 **Approve** recommendations via web UI

---

## Architecture

```
┌──────────────────────────────────────────────┐
│                klyster binary                 │
├────────┬─────────┬─────────┬─────────────────┤
│   UI   │   Web   │  Agent  │   Core + DB     │
│(embed) │ (axum)  │ (coll.) │ (sqlx+migrate)  │
└────────┴────┬────┴─────────┴────────┬────────┘
              │                        │
              │ HTTP API               │ gRPC
              │                        │
         Clients              ┌────────▼─────────┐
                              │ Python Analytics  │
                              │ (sidecar process) │
                              └──────────────────┘
```

**Tech Stack**:
- Rust (core, web, agent)
- Python 3.11+ (analytics)
- SQLite / PostgreSQL
- axum, sqlx, clap, tracing
- Svelte (UI - TBD)

---

## Project Status

| Milestone | Status | Progress |
|-----------|--------|----------|
| M1: Core + DB | 🔜 Next | 0/16 |
| M2: Web API | 📋 Planned | 0/14 |
| M3: Prometheus | 📋 Planned | 0/10 |
| M4: Analytics | 📋 Planned | 0/14 |
| M5: UI | 📋 Planned | 0/12 |
| M6: Agent | 📋 Planned | 0/8 |
| M7: Kubernetes | 📋 Planned | 0/10 |
| M8: PostgreSQL | 📋 Planned | 0/8 |
| M9: Custom ML | 📋 Planned | 0/10 |
| M10: Production | 📋 Planned | 0/12 |

**Total**: 0/114 tickets complete

---

## Documentation

- [`docs/PROJECT_STATE.md`](docs/PROJECT_STATE.md) — **Start here** for current state
- [`docs/PRD.md`](docs/PRD.md) — Product requirements
- [`docs/tickets/`](docs/tickets/) — Implementation tickets (114 total)
- [`docs/tickets/MILESTONES_SUMMARY.md`](docs/tickets/MILESTONES_SUMMARY.md) — All milestones overview

---

## Development

**Prerequisites**:
- Rust 1.75+ (will be specified in CP-M1-001)
- Python 3.11+
- Git

**Setup** (will be documented as we build):
```bash
# Clone
git clone <repo-url>
cd klyster

# Build (after CP-M1-001)
cargo build

# Run (after CP-M1-016)
cargo run
```

---

## Contributing

This project is currently in active development. Implementation follows the ticket system in `docs/tickets/`.

**Workflow**:
1. Pick next unchecked ticket from current milestone
2. Implement according to acceptance criteria
3. Update ticket status (mark as done)
4. Commit with conventional commit message
5. Move to next ticket

---

## License

MIT (see [LICENSE](LICENSE))

---

## Contact

(To be added)


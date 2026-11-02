# Liquid Harness

Liquid Harness is an experimental polyglot runtime and recursive agent harnessing platform. It enables continuous, self-improving agent execution across heterogeneous runtimes without weight fine-tuning.

## Inspirations & Foundations

1. **[Harness² (Google Research)](https://github.com/google-research/harness2)**: Recursive Agent Harnessing for an Open World.
   - Dynamic optimization and rewriting of the execution harness (prompts, memory, tools, skills, routers).
   - **Propose–Probe–Compose** loop for novel task execution.
   - Cross-task learning via persistent experience ledgers.

2. **[RRSI (Google Research)](https://github.com/google-research/rrsi)**: Regularized Recursive Self-Improvement of Agent Harnesses.
   - Out-of-Distribution (OOD) validation to eliminate benchmark overfitting and noise-chasing.
   - Critic and pruner mechanisms filtering low-value or counterproductive harness changes.
   - Token-cost constraints and sandbox/isolation boundaries.

---

## Polyglot Architecture & Roles

```
Liquid-Harness/
├── core/             # RUST: High-performance runtime, sandboxing, differential probe analysis
├── orchestrator/     # GO: Concurrent distributed orchestration, worker pools, IPC/gRPC
├── research/         # PYTHON: Evals, benchmarks, OOD validation metrics, RRSI critics
├── runtime/          # BUN: TypeScript agent runtime, MCP tool servers, skill dispatch
├── supervision/      # ELIXIR: OTP supervision trees, resilient state actors, fault tolerance
├── proto/            # Cross-language IPC contracts & schemas (Protobuf)
└── Makefile          # Unified polyglot workspace automation
```

| Layer | Runtime | Primary Responsibility |
| :--- | :--- | :--- |
| **`core/`** | Rust | Zero-copy execution harness, safe sandbox boundaries, memory primitives, probe diffs |
| **`orchestrator/`** | Go | Process supervision, worker pools, task streaming, gRPC orchestration |
| **`research/`** | Python (`uv`) | Evaluation datasets, RRSI regularization critics, benchmark integration |
| **`runtime/`** | Bun (TypeScript) | Fast agent runtime, tool execution, MCP interfaces, dynamic skill loader |
| **`supervision/`** | Elixir (OTP) | Self-healing supervision trees, actor mesh, distributed state resilience |
| **`proto/`** | Protobuf | Unified schemas for harness components, proposals, probes, and ledgers |

---

## Workspace Quickstart

### Prerequisites
- Rust & Cargo (`>= 1.80`)
- Go (`>= 1.24`)
- Python (`>= 3.12`) with `uv`
- Bun (`>= 1.4`)
- Elixir (`>= 1.17`) & Erlang/OTP (`>= 27`)

### Commands

```bash
# Run static checks across all 5 language stacks
make check

# Run unit tests across all 5 language stacks
make test

# Build binaries and artifacts
make build
```

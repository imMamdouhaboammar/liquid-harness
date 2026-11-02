# Liquid Harness (v1.1.0)

Liquid Harness is an ephemeral, micro-kernel agent infrastructure built for an Open World, synthesizing the **Propose-Probe-Compose** paradigm from [Harness²](https://github.com/google-research/harness2), the **Regularized Recursive Self-Improvement (RRSI)** anti-overfitting guards from [RRSI](https://github.com/google-research/rrsi), and autonomous self-evolution workflows inspired by [AutoHarness](https://github.com/kayba-ai/autoharness).

Upon deployment, only a bare minimum runtime kernel boots (`discover_harness_capabilities`). The agent dynamically self-assembles its harness across modular components, executes multi-stage evaluation pipelines (Screening, Validation, Holdout), runs continuous optimization campaigns over candidate mutations, and promotes champions with zero downtime.

---

## Polyglot Micro-Services Topology

All internal services communicate strictly over **local loopback (IPC / UDS)**, with only the Elixir BEAM gateway exposed externally.

```
                  ┌──────────────────────────────────────────────┐
                  │    External Gateway: Elixir / Bandit         │
                  │    Port: 127.0.0.1:4000 (REST / SSE)         │
                  │    Campaign State Machine & Champion Rollback│
                  └──────────────────────┬───────────────────────┘
                                         │
        ┌────────────────────────────────┼────────────────────────────────┐
        │                                │                                │
┌───────▼─────────────────┐   ┌──────────▼──────────────┐   ┌─────────────▼──────────┐
│  Rust Security Critic   │   │  Go Discovery Registry  │   │  Bun Ephemeral Sandbox │
│  Tonic gRPC: :50051     │   │  Uber FX gRPC: :50052   │   │  ElysiaJS HTTP: :50053 │
│  AST Denylists & Memory │   │  Track Catalog & Queue  │   │  Multi-Stage Pipeline  │
│  Paired Welch's T-CI    │   │  24h Scout & Blueprints │   │  Benchmark Adapters    │
└─────────────────────────┘   └─────────────────────────┘   └────────────────────────┘
                                         │
                              ┌──────────▼──────────────┐
                              │  Python Prompt Engine   │
                              │  FastAPI / LiteLLM      │
                              │  Port: 127.0.0.1:50054  │
                              │  Failure-Driven Gen     │
                              │  Root Memory & Transfer │
                              └─────────────────────────┘
```

| Engine | Framework & Technology | Role in Liquid Harness | Loopback Port |
| :--- | :--- | :--- | :--- |
| **Orchestrator & Live State** | **Elixir** (Bandit + Finch + OTP DynamicSupervisor) | Immutable root kernel, "Let It Crash" actor resilience, atomic champion promotion and instant rollback | `127.0.0.1:4000` |
| **Security Critic & Statistics** | **Rust** (Tonic gRPC + Regex + AST Sanitizer) | Gatekeeper auditing diff safety, paired Welch's t-confidence intervals, stability scores, and payload limits | `127.0.0.1:50051` |
| **Tool Discovery & Router** | **Go** (Uber FX + gRPC-Go + Zap) | Track catalog, campaign queue scheduler, 24-hour scout, lightweight blueprint registry | `127.0.0.1:50052` |
| **Execution Sandbox & Staging** | **Bun / TypeScript** (ElysiaJS + V8 Isolates) | Multi-stage staging pipeline (Screening, Validation, Holdout) and pluggable benchmark adapters | `127.0.0.1:50053` |
| **Inference & Optimizer** | **Python** (FastAPI + LiteLLM + Uvicorn) | Failure-driven proposal generator, AST prompt compressor, cross-campaign Root Memory manager | `127.0.0.1:50054` |

---

## AutoHarness-Inspired Subsystems

### 1. Multi-Stage Staging Pipeline
Candidates must pass through three distinct evaluation stages before reaching champion consideration:
1. **Stage 1 (Screening):** High-speed 1-shot execution probe. Fast-rejects broken syntax, infinite loops, and timeouts (< 1,500ms) to conserve computational budget.
2. **Stage 2 (Validation):** Multi-iteration contrastive evaluation ($N \ge 3$) against baseline. Computes paired difference mean, confidence interval, and stability score ($\ge 0.80$).
3. **Stage 3 (Holdout):** Evaluation against unseen test vectors to prevent benchmark overfitting and enforce RRSI generalization guarantees.

### 2. Pluggable Benchmark Adapters
- `generic_command`: Shell command benchmark runner with timeout protection and process isolation.
- `bun_test`: Native Bun test runner parsing test counts, pass rates, and failure traces.
- `agent_eval`: Multi-task agent evaluation suite measuring tool-call accuracy, task weight completion, and execution latency.

### 3. Failure-Driven Proposal Generators
- Inspects benchmark error logs and stack traces to classify failures (`timeout`, `syntax_error`, `token_bloat`, `security_violation`).
- Generates targeted corrective mutations (e.g. timeout circuit breakers, prompt compression, AST bounds).

### 4. Cross-Campaign Root Memory
- Persists failure fingerprints and successful interventions across campaigns in `.liquid/root_memory.json`.
- Prevents overnight loops from repeating disproven hypotheses.
- Recommends transfer learning strategies for new tracks based on prior champion lineages.

---

## Operator CLI Commands

```bash
# Polyglot system audit of toolchains, loopback services, and adapters
bun bin/liquid.ts doctor

# Initialize workspace directories (.liquid/) and starter configuration
bun bin/liquid.ts guide

# Execute benchmark adapter directly
bun bin/liquid.ts benchmark agent_eval

# Generate candidate harness mutation proposal
bun bin/liquid.ts proposal prompt_optimization

# Run autonomous multi-stage optimization campaign loop
bun bin/liquid.ts campaign prompt_optimization 3

# View operational summary and champion lineage
bun bin/liquid.ts report

# Execute complete 5-phase lifecycle simulation
bun bin/liquid.ts simulate
```

---

## Verification & Build Commands

```bash
# Run all static compiler and type checks (Rust, Go, Python, Bun, Elixir)
make check

# Run full unit and integration test suite across all 5 engines
make test

# Execute the 5-phase lifecycle simulation
make simulate

# Build production binaries
make build
```

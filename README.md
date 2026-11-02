# Liquid Harness MVP

Liquid Harness is an ephemeral, micro-kernel agent infrastructure built for an Open World, synthesizing the **Propose-Probe-Compose** paradigm from [Harness²](https://github.com/google-research/harness2) and the **Regularized Recursive Self-Improvement (RRSI)** anti-overfitting guards from [RRSI](https://github.com/google-research/rrsi).

Upon deployment, only a bare minimum runtime kernel boots (`discover_harness_capabilities`). The agent dynamically self-assembles its harness across 8 modular components and continuously executes an automated 24-hour self-improvement loop with zero downtime.

---

## Polyglot Micro-Services Topology

All internal services communicate strictly over **local loopback (IPC / UDS)**, with only the Elixir BEAM gateway exposed externally.

```
                  ┌──────────────────────────────────────────────┐
                  │    External Gateway: Elixir / Bandit         │
                  │    Port: 127.0.0.1:4000 (REST / SSE)         │
                  └──────────────────────┬───────────────────────┘
                                         │
        ┌────────────────────────────────┼────────────────────────────────┐
        │                                │                                │
┌───────▼─────────────────┐   ┌──────────▼──────────────┐   ┌─────────────▼──────────┐
│  Rust Security Critic   │   │  Go Discovery Registry  │   │  Bun Ephemeral Sandbox │
│  Tonic gRPC: :50051     │   │  Uber FX gRPC: :50052   │   │  ElysiaJS HTTP: :50053 │
│  AST Denylists & Memory │   │  24h Scout & Blueprints │   │  V8 Contrastive Probes │
└─────────────────────────┘   └─────────────────────────┘   └────────────────────────┘
                                         │
                              ┌──────────▼──────────────┐
                              │  Python Prompt Engine   │
                              │  FastAPI / LiteLLM      │
                              │  Port: 127.0.0.1:50054  │
                              │  Token Budget & AST     │
                              └─────────────────────────┘
```

| Engine | Framework & Technology | Role in Liquid Harness | Loopback Port |
| :--- | :--- | :--- | :--- |
| **Orchestrator & Live State** | **Elixir** (Bandit + Finch + OTP DynamicSupervisor) | Immutable root kernel, "Let It Crash" actor resilience, atomic in-memory hot-swaps | `127.0.0.1:4000` |
| **Security Critic** | **Rust** (Tonic gRPC + Regex + AST Sanitizer) | Gatekeeper auditing mutation proposals for code injection, path traversal, and payload bounds | `127.0.0.1:50051` |
| **Tool Discovery & Router** | **Go** (Uber FX + gRPC-Go + Zap) | 24-hour automated cron crawler, lightweight blueprint registry (no git clones) | `127.0.0.1:50052` |
| **Execution Sandbox** | **Bun / TypeScript** (ElysiaJS + V8 Isolates) | Microsecond ephemeral sandbox running parallel contrastive probes (Baseline vs Candidate) | `127.0.0.1:50053` |
| **Inference & Optimizer** | **Python** (FastAPI + LiteLLM + Uvicorn) | Standardized model execution, AST prompt compressor, RRSI token-cost budget gate | `127.0.0.1:50054` |

---

## 5-Phase Mutation Lifecycle

1. **Phase 1: Boot (Minimalism):** Agent boots with exactly one active tool: `discover_harness_capabilities`.
2. **Phase 2: Self-Assembling:** Queries Go registry, selects 4–5 Day-1 capabilities (`ast_sanitizer`, `vector_memory_bus`, `ephemeral_sandbox_runner`, `prompt_compressor`), compiling Generation 2.
3. **Phase 3: The 24h Scout:** Elixir cron interrupt fires, prompting Go to scout external blueprints for version upgrades.
4. **Phase 4: Regularized Auditing (The RRSI Guard):**
   - **Rust Critic:** Verifies AST safety, regex denylists (zero shell/eval/credential violations).
   - **Bun Sandbox:** Runs contrastive probe executions calculating latency delta, success rate, and divergence.
   - **Python Optimizer:** Enforces RRSI token budget rules (rejects token bloat without accuracy gains).
5. **Phase 5: Hot-Swap Rollout:** Atomically updates live memory maps in Elixir with zero service downtime.

---

## Verification & Execution Commands

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

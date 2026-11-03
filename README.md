# Liquid Harness (v2.0.0 - 100% Rust Engine)

Liquid Harness is an ephemeral, micro-kernel agent infrastructure built for an Open World, synthesizing the **Propose-Probe-Compose** paradigm from [Harness²](https://github.com/google-research/harness2), the **Regularized Recursive Self-Improvement (RRSI)** anti-overfitting guards from [RRSI](https://github.com/google-research/rrsi), and autonomous self-evolution workflows inspired by [AutoHarness](https://github.com/kayba-ai/autoharness).

Refactored from fragmented polyglot microservices into a **pure 100% Rust single-binary micro-kernel**, eliminating all network transport latency, multi-toolchain maintenance toil, and simulated mocks.

---

## Unified Zero-Cost Architecture

All subsystems execute natively within a unified Rust runtime without loopback sockets, serialization bottlenecks, or runtime overhead:

```
┌────────────────────────────────────────────────────────────────────────┐
│                   Liquid Harness Single-Binary (Rust)                  │
├────────────────────────────────────────────────────────────────────────┤
│ • Agent Kernel State Machine  : Atomic In-Memory Snapshot & Rollback   │
│ • Security Critic             : AST Sanitizer & Regex Denylists        │
│ • Discovery Registry          : Lock-Free Thread-Safe Blueprint Catalog│
│ • Ephemeral Execution Sandbox : Microsecond Isolated Probe Execution   │
│ • AutoHarness Staging Pipeline: 3-Stage Gating (Screen, Validate, Hold)│
│ • Statistical Evaluator       : Paired Welch's t-test & Stability Index│
│ • Prompt Optimizer & Gen      : Failure-Driven Mutation & Token Budget │
│ • Cross-Campaign Memory       : Durable JSON Root Memory Engine        │
└────────────────────────────────────────────────────────────────────────┘
```

| Engine Subsystem | Module | Role & Guarantees | Latency Boundary |
| :--- | :--- | :--- | :--- |
| **Agent Kernel State** | `liquid_core::state` | Immutable snapshot transitions, atomic hot-swap promotion, and instant rollback | `< 1 µs` |
| **Security Critic** | `liquid_core::critic` | Audits syntax, AST depth limits, forbidden execution primitives (`exec`, `rm -rf`) | `< 10 µs` |
| **Blueprint Registry** | `liquid_core::registry` | Dynamic tool discovery, Day-1 self-assembly, and 24h catalog scout | `< 2 µs` |
| **Ephemeral Sandbox** | `liquid_core::sandbox` | Timeout-fenced probe execution, infinite loop detection, and contrastive deltas | `< 50 µs` |
| **Staging Pipeline** | `liquid_core::stages` | 3-Stage evaluation gating: Stage 1 (Screening), Stage 2 (Validation), Stage 3 (Holdout) | `< 2 ms` |
| **Statistical Engine** | `liquid_core::evaluator` | Paired Welch's t-confidence intervals ($95\%$) and coefficient-of-variation stability | `< 5 µs` |
| **Prompt Optimizer** | `liquid_core::optimizer` | Failure-driven corrective mutations, token footprint estimation, and budget fences | `< 15 µs` |
| **Root Memory** | `liquid_core::memory` | Cross-campaign persistence of failure fingerprints and winning lineages in `.liquid/` | `< 100 µs` |

---

## AutoHarness 3-Stage Staging Pipeline

Candidates must pass three deterministic gates before champion consideration:
1. **Stage 1 (Screening):** High-speed 1-shot execution probe. Fast-rejects broken syntax, infinite loops, and execution errors ($< 1,500\text{ms}$).
2. **Stage 2 (Validation):** Multi-iteration contrastive evaluation ($N \ge 3$) against baseline. Computes paired difference mean, confidence interval, and stability score ($\ge 0.85$).
3. **Stage 3 (Holdout):** Evaluation against unseen test vectors to prevent benchmark overfitting and enforce RRSI generalization guarantees.

---

## Operator CLI Commands

The standalone compiled binary `bin/liquid` provides complete operational control:

```bash
# Pure Rust toolchain and adapter health audit
./bin/liquid doctor

# Initialize workspace directories (.liquid/) and starter configuration
./bin/liquid guide

# Execute benchmark adapter directly (generic_command, cargo_test, agent_eval)
./bin/liquid benchmark agent_eval

# Generate candidate harness mutation proposal
./bin/liquid proposal prompt_optimization

# Run autonomous multi-stage optimization campaign loop
./bin/liquid campaign prompt_optimization 3

# View operational summary and champion lineage
./bin/liquid report

# List online infinity slots and active attached plugins
./bin/liquid slots

# List installed plugins across all slots
./bin/liquid plugin list

# Audit and install a plugin manifest file
./bin/liquid plugin install ./plugins/examples/weather_lookup.json

# Execute an installed plugin through its slot
./bin/liquid plugin run weather_lookup

# Execute complete 5-phase lifecycle simulation
./bin/liquid simulate
```

---

## Infinity Slots Architecture (Everything is a Plugin)

Liquid Harness exposes typed, dynamic extension points termed **Infinity Slots**. External tools, cognitive skills, security guardrails, benchmark adapters, and memory providers attach to slots with zero kernel recompilation:

```
┌────────────────────────────────────────────────────────────────────────┐
│                     Liquid Microkernel Core Engine                     │
│  State Machine  │  Atomic Rollback  │  AutoHarness 3-Stage Evaluator   │
└───────────────────────────────────┬────────────────────────────────────┘
                                    │
                        [ Infinity Slots Bus ]
                                    │
       ┌──────────────┬─────────────┼─────────────┬──────────────┐
       ▼              ▼             ▼             ▼              ▼
┌──────────────┐┌────────────┐┌────────────┐┌────────────┐┌────────────┐
│  Tool Slot   ││ Skill Slot ││Critic Slot ││ Benchmark  ││Memory Slot │
│(capabilities)││(workflows) ││(guardrails)││(benchmarks)││ (storage)   │
└──────────────┘└────────────┘└────────────┘└────────────┘└────────────┘
```

Every plugin is defined via a standard JSON manifest (`PluginManifest`), audited via the Rust `SecurityCritic` AST and mutation scanner, and bound to its slot at runtime.

---

## Verification & Build

```bash
# Run compiler checks across all Rust crates
make check

# Execute complete unit test suite
make test

# Build optimized release binary to bin/liquid
make build

# Run 5-phase lifecycle simulation
make simulate
```

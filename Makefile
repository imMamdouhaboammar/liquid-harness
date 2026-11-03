.PHONY: all check test build simulate clean doctor guide help

all: check test simulate

simulate:
	@echo "==> Executing 5-Phase Liquid Harness lifecycle simulation (100% Rust)..."
	./bin/liquid simulate

doctor:
	@echo "==> Running Liquid Harness micro-kernel doctor..."
	./bin/liquid doctor

guide:
	@echo "==> Running Liquid Harness workspace onboarding..."
	./bin/liquid guide

check:
	@echo "==> Checking Rust workspace..."
	cargo check --all-targets

test:
	@echo "==> Testing 100% Rust workspace..."
	cargo test

build:
	@echo "==> Building release binary..."
	cargo build --release
	mkdir -p bin
	cp target/release/liquid bin/liquid
	chmod +x bin/liquid

clean:
	@echo "==> Cleaning build artifacts..."
	cargo clean
	rm -f bin/liquid

help:
	@echo "Liquid Harness Workspace Commands (100% Rust):"
	@echo "  make check         - Run static analysis & type checks across Rust crates"
	@echo "  make test          - Run unit and integration tests"
	@echo "  make build         - Build optimized release binary in bin/liquid"
	@echo "  make simulate      - Execute 5-phase lifecycle simulation"
	@echo "  make doctor        - Audit micro-kernel health and adapters"
	@echo "  make guide         - Initialize .liquid workspace directories"

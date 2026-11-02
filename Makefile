.PHONY: all check check-rust check-go check-python check-bun check-elixir test build clean help

all: check test

check: check-rust check-go check-python check-bun check-elixir
	@echo "==> All polyglot workspace checks passed successfully."

check-rust:
	@echo "==> Checking Rust core workspace..."
	cd core && cargo check --all-targets

check-go:
	@echo "==> Checking Go orchestrator..."
	cd orchestrator && go vet ./...

check-python:
	@echo "==> Checking Python research workspace..."
	cd research && uv run python -m py_compile main.py

check-bun:
	@echo "==> Checking Bun runtime..."
	cd runtime && bun check

check-elixir:
	@echo "==> Checking Elixir supervision..."
	cd supervision && mix compile --warnings-as-errors

test: test-rust test-go test-python test-bun test-elixir
	@echo "==> All test suites executed."

test-rust:
	@echo "==> Testing Rust crates..."
	cd core && cargo test

test-go:
	@echo "==> Testing Go orchestrator..."
	cd orchestrator && go test ./...

test-python:
	@echo "==> Testing Python research workspace..."
	cd research && uv run python -m unittest discover -s . -p "test_*.py"

test-bun:
	@echo "==> Testing Bun runtime..."
	cd runtime && bun test

test-elixir:
	@echo "==> Testing Elixir supervision..."
	cd supervision && mix test

build:
	@echo "==> Building Rust workspace..."
	cd core && cargo build
	@echo "==> Building Go orchestrator..."
	cd orchestrator && go build -o bin/liquid-d ./cmd/liquid-d
	@echo "==> Building Elixir OTP application..."
	cd supervision && mix compile

clean:
	@echo "==> Cleaning build artifacts..."
	cd core && cargo clean
	rm -rf orchestrator/bin
	cd supervision && mix clean
	rm -rf runtime/node_modules
	rm -rf research/.venv

help:
	@echo "Liquid Harness Workspace Commands:"
	@echo "  make check         - Run static analysis / compiler checks across all 5 stacks"
	@echo "  make test          - Run unit tests across all 5 stacks"
	@echo "  make build         - Build core binaries and modules"
	@echo "  make clean         - Clean temporary build outputs"

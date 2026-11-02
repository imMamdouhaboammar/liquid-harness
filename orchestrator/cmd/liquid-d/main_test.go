package main

import (
	"context"
	"testing"

	"github.com/liquid-harness/liquid-harness/orchestrator/pkg/registry"
	"go.uber.org/zap"
)

func TestRegistryBootstrapDiscovery(t *testing.T) {
	logger := zap.NewNop()
	reg := registry.NewRegistry(logger)

	// Phase 1 verification: baseline capability exists
	tools := reg.Discover(context.Background(), "bootstrap", []string{"bootstrap"}, 10)
	if len(tools) == 0 {
		t.Fatal("expected discover_harness_capabilities to be present in baseline")
	}
	if tools[0].Id != "discover_harness_capabilities" {
		t.Fatalf("unexpected baseline tool id: %s", tools[0].Id)
	}

	// Phase 2 verification: query external catalog for initial capabilities
	allTools := reg.Discover(context.Background(), "assemble", nil, 10)
	if len(allTools) < 4 {
		t.Fatalf("expected at least 4 capabilities in catalog for Day-1 assembly, got %d", len(allTools))
	}

	// Phase 3 verification: 24h scout detects candidate upgrade
	count, candidates := reg.ScoutUpdates(context.Background(), []string{"prompt_compressor"})
	if count != 1 || len(candidates) != 1 {
		t.Fatalf("expected 1 update candidate from 24h scout, got %d", count)
	}
	if candidates[0].Version != "2.0.0" {
		t.Fatalf("expected version 2.0.0 for candidate upgrade, got %s", candidates[0].Version)
	}
}

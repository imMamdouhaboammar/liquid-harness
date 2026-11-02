package registry

import (
	"context"
	"testing"

	"go.uber.org/zap"
)

func TestRegistryDiscoveryAndScout(t *testing.T) {
	logger := zap.NewNop()
	reg := NewRegistry(logger)

	tools := reg.Discover(context.Background(), "general", []string{"kernel"}, 10)
	if len(tools) == 0 {
		t.Fatalf("expected at least 1 tool with tag 'kernel', got 0")
	}

	count, candidates := reg.ScoutUpdates(context.Background(), []string{"prompt_compressor"})
	if count != 1 || len(candidates) != 1 {
		t.Fatalf("expected 1 candidate for prompt_compressor, got %d", count)
	}

	if candidates[0].Version != "2.0.0" {
		t.Fatalf("expected version 2.0.0, got %s", candidates[0].Version)
	}
}

func TestTrackRegistryCampaignLifecycle(t *testing.T) {
	logger := zap.NewNop()
	tr := NewTrackRegistry(logger)

	tracks := tr.ListTracks()
	if len(tracks) < 3 {
		t.Fatalf("expected at least 3 default tracks, got %d", len(tracks))
	}

	track, err := tr.GetTrack("prompt_optimization")
	if err != nil || track == nil {
		t.Fatalf("failed to find prompt_optimization track: %v", err)
	}

	campaign, err := tr.StartCampaign("prompt_optimization", 5)
	if err != nil || campaign == nil {
		t.Fatalf("failed to start campaign: %v", err)
	}

	if campaign.Status != CampaignStatusRunning {
		t.Fatalf("expected campaign status running, got %s", campaign.Status)
	}

	updated, err := tr.UpdateCampaign(campaign.ID, 5, 0.95, "2.0.0")
	if err != nil {
		t.Fatalf("failed to update campaign: %v", err)
	}

	if updated.Status != CampaignStatusCompleted {
		t.Fatalf("expected campaign to complete at iteration 5, got %s", updated.Status)
	}

	if updated.BestScore != 0.95 {
		t.Fatalf("expected best score 0.95, got %f", updated.BestScore)
	}
}

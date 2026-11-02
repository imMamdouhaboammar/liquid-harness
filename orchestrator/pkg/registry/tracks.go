package registry

import (
	"fmt"
	"sync"
	"time"

	"go.uber.org/zap"
)

type Track struct {
	ID              string    `json:"id"`
	Name            string    `json:"name"`
	TargetComponent string    `json:"target_component"` // prompt | tool | middleware | guardrail
	BenchmarkAdapter string   `json:"benchmark_adapter"` // generic_command | bun_test | agent_eval
	Description     string    `json:"description"`
	CreatedAt       time.Time `json:"created_at"`
}

type CampaignStatus string

const (
	CampaignStatusPending   CampaignStatus = "pending"
	CampaignStatusRunning   CampaignStatus = "running"
	CampaignStatusCompleted CampaignStatus = "completed"
	CampaignStatusFailed    CampaignStatus = "failed"
)

type Campaign struct {
	ID               string         `json:"id"`
	TrackID          string         `json:"track_id"`
	Status           CampaignStatus `json:"status"`
	CurrentIteration int            `json:"current_iteration"`
	MaxIterations    int            `json:"max_iterations"`
	ChampionVersion  string         `json:"champion_version"`
	BestScore        float64        `json:"best_score"`
	StartedAt        time.Time      `json:"started_at"`
	UpdatedAt        time.Time      `json:"updated_at"`
}

type TrackRegistry struct {
	mu        sync.RWMutex
	tracks    map[string]*Track
	campaigns map[string]*Campaign
	logger    *zap.Logger
}

func NewTrackRegistry(logger *zap.Logger) *TrackRegistry {
	tr := &TrackRegistry{
		tracks:    make(map[string]*Track),
		campaigns: make(map[string]*Campaign),
		logger:    logger,
	}
	tr.seedDefaultTracks()
	return tr
}

func (tr *TrackRegistry) seedDefaultTracks() {
	tr.mu.Lock()
	defer tr.mu.Unlock()

	defaultTracks := []*Track{
		{
			ID:               "prompt_optimization",
			Name:             "Prompt Footprint Optimization",
			TargetComponent: "prompt",
			BenchmarkAdapter: "agent_eval",
			Description:      "Continuously compacts prompts while preserving benchmark accuracy.",
			CreatedAt:       time.Now(),
		},
		{
			ID:               "tool_synthesis",
			Name:             "Dynamic Tool Evolution",
			TargetComponent: "tool",
			BenchmarkAdapter: "bun_test",
			Description:      "Evolves and hot-swaps active tool implementations.",
			CreatedAt:       time.Now(),
		},
		{
			ID:               "security_hardening",
			Name:             "Zero-Trust Security Barrier",
			TargetComponent: "guardrail",
			BenchmarkAdapter: "generic_command",
			Description:      "Tightens AST validation rules and denylists against new attack vectors.",
			CreatedAt:       time.Now(),
		},
	}

	for _, t := range defaultTracks {
		tr.tracks[t.ID] = t
	}
}

func (tr *TrackRegistry) RegisterTrack(track *Track) error {
	tr.mu.Lock()
	defer tr.mu.Unlock()

	if track.ID == "" {
		return fmt.Errorf("track ID cannot be empty")
	}

	track.CreatedAt = time.Now()
	tr.tracks[track.ID] = track
	tr.logger.Info("Track registered", zap.String("track_id", track.ID))
	return nil
}

func (tr *TrackRegistry) GetTrack(id string) (*Track, error) {
	tr.mu.RLock()
	defer tr.mu.RUnlock()

	t, exists := tr.tracks[id]
	if !exists {
		return nil, fmt.Errorf("track '%s' not found", id)
	}
	return t, nil
}

func (tr *TrackRegistry) ListTracks() []*Track {
	tr.mu.RLock()
	defer tr.mu.RUnlock()

	res := make([]*Track, 0, len(tr.tracks))
	for _, t := range tr.tracks {
		res = append(res, t)
	}
	return res
}

func (tr *TrackRegistry) StartCampaign(trackID string, maxIterations int) (*Campaign, error) {
	tr.mu.Lock()
	defer tr.mu.Unlock()

	if _, exists := tr.tracks[trackID]; !exists {
		return nil, fmt.Errorf("cannot start campaign for non-existent track '%s'", trackID)
	}

	campaignID := fmt.Sprintf("camp-%s-%d", trackID, time.Now().UnixNano()%100000)
	c := &Campaign{
		ID:               campaignID,
		TrackID:          trackID,
		Status:           CampaignStatusRunning,
		CurrentIteration: 0,
		MaxIterations:    maxIterations,
		ChampionVersion:  "1.0.0",
		BestScore:        0.0,
		StartedAt:        time.Now(),
		UpdatedAt:        time.Now(),
	}

	tr.campaigns[campaignID] = c
	tr.logger.Info("Started optimization campaign",
		zap.String("campaign_id", campaignID),
		zap.String("track_id", trackID),
		zap.Int("max_iterations", maxIterations))

	return c, nil
}

func (tr *TrackRegistry) UpdateCampaign(campaignID string, iteration int, score float64, championVersion string) (*Campaign, error) {
	tr.mu.Lock()
	defer tr.mu.Unlock()

	c, exists := tr.campaigns[campaignID]
	if !exists {
		return nil, fmt.Errorf("campaign '%s' not found", campaignID)
	}

	c.CurrentIteration = iteration
	c.UpdatedAt = time.Now()
	if score > c.BestScore {
		c.BestScore = score
		c.ChampionVersion = championVersion
	}
	if iteration >= c.MaxIterations {
		c.Status = CampaignStatusCompleted
	}

	return c, nil
}

func (tr *TrackRegistry) ListCampaigns() []*Campaign {
	tr.mu.RLock()
	defer tr.mu.RUnlock()

	res := make([]*Campaign, 0, len(tr.campaigns))
	for _, c := range tr.campaigns {
		res = append(res, c)
	}
	return res
}

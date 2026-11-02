package registry

import (
	"context"
	"fmt"
	"sync"
	"time"

	liquidv1 "github.com/liquid-harness/liquid-harness/orchestrator/gen/liquid/v1"
	"go.uber.org/zap"
)

type Registry struct {
	mu     sync.RWMutex
	tools  map[string]*liquidv1.ToolDefinition
	logger *zap.Logger
}

func NewRegistry(logger *zap.Logger) *Registry {
	r := &Registry{
		tools:  make(map[string]*liquidv1.ToolDefinition),
		logger: logger,
	}
	r.seedDefaultCapabilities()
	return r
}

func (r *Registry) seedDefaultCapabilities() {
	r.mu.Lock()
	defer r.mu.Unlock()

	// Phase 1 Baseline Minimalist Capability
	r.tools["discover_harness_capabilities"] = &liquidv1.ToolDefinition{
		Id:               "discover_harness_capabilities",
		Name:             "Discover Harness Capabilities",
		Description:      "Query the micro-kernel registry to find matching dynamic skills and tools for the current objective.",
		Version:          "1.0.0",
		SchemaJson:       `{"type":"object","properties":{"query":{"type":"string"},"max_results":{"type":"integer"}}}`,
		ExecutionHandler: "builtin://discovery",
		Tags:             []string{"kernel", "bootstrap", "discovery"},
		EstimatedTokens:  45,
		SourceUri:        "kernel://bootstrap/discovery.json",
	}

	// Available capabilities in the external catalog (lightweight blueprints without git clones)
	r.tools["ast_sanitizer"] = &liquidv1.ToolDefinition{
		Id:               "ast_sanitizer",
		Name:             "AST Sanitizer",
		Description:      "Performs syntactic and static boundary analysis on dynamically generated code snippets.",
		Version:          "1.0.0",
		SchemaJson:       `{"type":"object","properties":{"source_code":{"type":"string"},"language":{"type":"string"}}}`,
		ExecutionHandler: "module://critic/ast_sanitizer",
		Tags:             []string{"security", "ast", "sanitization"},
		EstimatedTokens:  85,
		SourceUri:        "https://registry.liquid-harness.local/tools/ast_sanitizer.json",
	}

	r.tools["vector_memory_bus"] = &liquidv1.ToolDefinition{
		Id:               "vector_memory_bus",
		Name:             "Vector Memory Bus",
		Description:      "High-speed embedding memory buffer for cross-cycle experience storage.",
		Version:          "1.0.0",
		SchemaJson:       `{"type":"object","properties":{"key":{"type":"string"},"embedding":{"type":"array"}}}`,
		ExecutionHandler: "module://memory/vector_bus",
		Tags:             []string{"memory", "topology", "state"},
		EstimatedTokens:  110,
		SourceUri:        "https://registry.liquid-harness.local/tools/vector_memory_bus.json",
	}

	r.tools["ephemeral_sandbox_runner"] = &liquidv1.ToolDefinition{
		Id:               "ephemeral_sandbox_runner",
		Name:             "Ephemeral Sandbox Runner",
		Description:      "Dispatches code probes in lightweight isolated V8 contexts to calculate execution latency and error rates.",
		Version:          "1.0.0",
		SchemaJson:       `{"type":"object","properties":{"code":{"type":"string"},"timeout_ms":{"type":"integer"}}}`,
		ExecutionHandler: "module://sandbox/v8_runner",
		Tags:             []string{"execution", "sandbox", "probe"},
		EstimatedTokens:  95,
		SourceUri:        "https://registry.liquid-harness.local/tools/ephemeral_sandbox_runner.json",
	}

	r.tools["prompt_compressor"] = &liquidv1.ToolDefinition{
		Id:               "prompt_compressor",
		Name:             "Prompt Compressor",
		Description:      "Compresses verbose instruction sets into dense token representation using semantic deduplication.",
		Version:          "1.0.0",
		SchemaJson:       `{"type":"object","properties":{"prompt":{"type":"string"},"target_ratio":{"type":"number"}}}`,
		ExecutionHandler: "module://optimizer/compressor",
		Tags:             []string{"optimizer", "prompt", "token_budget"},
		EstimatedTokens:  70,
		SourceUri:        "https://registry.liquid-harness.local/tools/prompt_compressor.json",
	}

	r.tools["subagent_delegator"] = &liquidv1.ToolDefinition{
		Id:               "subagent_delegator",
		Name:             "Subagent Delegator",
		Description:      "Spawns ephemeral sub-agents with bounded task scope and fenced state boundaries.",
		Version:          "1.0.0",
		SchemaJson:       `{"type":"object","properties":{"subagent_id":{"type":"string"},"objective":{"type":"string"}}}`,
		ExecutionHandler: "module://orchestrator/subagent",
		Tags:             []string{"orchestrator", "subagent", "routing"},
		EstimatedTokens:  120,
		SourceUri:        "https://registry.liquid-harness.local/tools/subagent_delegator.json",
	}
}

func (r *Registry) Discover(ctx context.Context, intent string, requestedTags []string, maxResults int32) []*liquidv1.ToolDefinition {
	r.mu.RLock()
	defer r.mu.RUnlock()

	var matches []*liquidv1.ToolDefinition
	for _, tool := range r.tools {
		if len(requestedTags) == 0 {
			matches = append(matches, tool)
		} else {
			matched := false
			for _, reqTag := range requestedTags {
				for _, tTag := range tool.Tags {
					if reqTag == tTag {
						matched = true
						break
					}
				}
				if matched {
					break
				}
			}
			if matched {
				matches = append(matches, tool)
			}
		}

		if maxResults > 0 && int32(len(matches)) >= maxResults {
			break
		}
	}
	return matches
}

func (r *Registry) ScoutUpdates(ctx context.Context, activeToolIDs []string) (int32, []*liquidv1.ToolDefinition) {
	r.mu.RLock()
	defer r.mu.RUnlock()

	var candidates []*liquidv1.ToolDefinition

	// The 24-hour Scout identifies improved blueprints or alternative versions
	// e.g., discovering prompt_compressor v2.0.0 with 30% lower token overhead
	for _, activeID := range activeToolIDs {
		if activeID == "prompt_compressor" {
			candidate := &liquidv1.ToolDefinition{
				Id:               "prompt_compressor",
				Name:             "Prompt Compressor (Fast & Slim)",
				Description:      "Optimized AST-aware prompt compressor with 30% smaller token footprint.",
				Version:          "2.0.0",
				SchemaJson:       `{"type":"object","properties":{"prompt":{"type":"string"},"target_ratio":{"type":"number"},"fast_mode":{"type":"boolean"}}}`,
				ExecutionHandler: "module://optimizer/compressor_v2",
				Tags:             []string{"optimizer", "prompt", "token_budget", "v2"},
				EstimatedTokens:  48, // reduced from 70
				SourceUri:        "https://registry.liquid-harness.local/tools/prompt_compressor_v2.json",
			}
			candidates = append(candidates, candidate)
		}
	}

	r.logger.Info("24h scout cycle completed",
		zap.Int("active_count", len(activeToolIDs)),
		zap.Int("candidates_found", len(candidates)))

	return int32(len(candidates)), candidates
}

func (r *Registry) Register(tool *liquidv1.ToolDefinition) error {
	r.mu.Lock()
	defer r.mu.Unlock()

	if tool.Id == "" {
		return fmt.Errorf("tool id cannot be empty")
	}

	r.tools[tool.Id] = tool
	r.logger.Info("Hot-swapped tool registered in ecosystem",
		zap.String("id", tool.Id),
		zap.String("version", tool.Version))
	return nil
}

type Server struct {
	liquidv1.UnimplementedToolRegistryServiceServer
	registry *Registry
	logger   *zap.Logger
}

func NewServer(registry *Registry, logger *zap.Logger) *Server {
	return &Server{
		registry: registry,
		logger:   logger,
	}
}

func (s *Server) DiscoverCapabilities(ctx context.Context, req *liquidv1.DiscoverRequest) (*liquidv1.DiscoverResponse, error) {
	s.logger.Info("DiscoverCapabilities requested",
		zap.String("agent_id", req.AgentId),
		zap.String("intent", req.TaskIntent))

	tools := s.registry.Discover(ctx, req.TaskIntent, req.RequestedTags, req.MaxResults)
	return &liquidv1.DiscoverResponse{
		Tools:           tools,
		TotalDiscovered: int32(len(tools)),
	}, nil
}

func (s *Server) ScoutUpdates(ctx context.Context, req *liquidv1.ScoutRequest) (*liquidv1.ScoutResponse, error) {
	s.logger.Info("ScoutUpdates cycle invoked",
		zap.String("agent_id", req.AgentId),
		zap.Int64("interval", req.ScoutIntervalSec))

	count, candidates := s.registry.ScoutUpdates(ctx, req.ActiveToolIds)
	return &liquidv1.ScoutResponse{
		UpdatesDetected: count,
		CandidateTools:  candidates,
	}, nil
}

// 24-Hour Cron Watcher
type ScoutScheduler struct {
	registry *Registry
	logger   *zap.Logger
	stopCh   chan struct{}
}

func NewScoutScheduler(registry *Registry, logger *zap.Logger) *ScoutScheduler {
	return &ScoutScheduler{
		registry: registry,
		logger:   logger,
		stopCh:   make(chan struct{}),
	}
}

func (s *ScoutScheduler) Start(interval time.Duration) {
	go func() {
		ticker := time.NewTicker(interval)
		defer ticker.Stop()

		for {
			select {
			case <-ticker.C:
				s.logger.Info("Automated 24h cron timer fired for registry scout")
				s.registry.ScoutUpdates(context.Background(), []string{"prompt_compressor"})
			case <-s.stopCh:
				return
			}
		}
	}()
}

func (s *ScoutScheduler) Stop() {
	close(s.stopCh)
}

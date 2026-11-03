use crate::types::{ToolDefinition, TrackDefinition, TrackPolicy};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

#[derive(Clone)]
pub struct Registry {
    tools: Arc<RwLock<HashMap<String, ToolDefinition>>>,
    tracks: Arc<RwLock<HashMap<String, TrackDefinition>>>,
}

impl Default for Registry {
    fn default() -> Self {
        Self::new()
    }
}

impl Registry {
    pub fn new() -> Self {
        let registry = Self {
            tools: Arc::new(RwLock::new(HashMap::new())),
            tracks: Arc::new(RwLock::new(HashMap::new())),
        };
        registry.seed_defaults();
        registry
    }

    fn seed_defaults(&self) {
        let mut tools = self.tools.write().unwrap();

        // Phase 1 Baseline Minimalist Capability
        tools.insert(
            "discover_harness_capabilities".to_string(),
            ToolDefinition {
                id: "discover_harness_capabilities".to_string(),
                name: "Discover Harness Capabilities".to_string(),
                version: "1.0.0".to_string(),
                description: "Query the micro-kernel registry to find matching dynamic skills and tools.".to_string(),
                execution_handler: "builtin://discovery".to_string(),
                schema_json: r#"{"type":"object","properties":{"query":{"type":"string"},"max_results":{"type":"integer"}}}"#.to_string(),
                tags: vec!["kernel".to_string(), "bootstrap".to_string(), "discovery".to_string()],
                estimated_tokens: 45,
                source_uri: "kernel://bootstrap/discovery.json".to_string(),
            },
        );

        // Day-1 Capabilities
        let day1 = vec![
            ToolDefinition {
                id: "ast_sanitizer".to_string(),
                name: "AST Sanitizer".to_string(),
                version: "1.0.0".to_string(),
                description: "Syntactic boundary and denylist analysis.".to_string(),
                execution_handler: "module://critic/ast_sanitizer".to_string(),
                schema_json: r#"{"type":"object","properties":{"source_code":{"type":"string"}}}"#.to_string(),
                tags: vec!["security".to_string(), "ast".to_string()],
                estimated_tokens: 85,
                source_uri: "https://registry.liquid-harness.local/tools/ast_sanitizer.json".to_string(),
            },
            ToolDefinition {
                id: "vector_memory_bus".to_string(),
                name: "Vector Memory Bus".to_string(),
                version: "1.0.0".to_string(),
                description: "Dynamic embedding memory buffer.".to_string(),
                execution_handler: "module://memory/vector_bus".to_string(),
                schema_json: r#"{"type":"object","properties":{"query_vector":{"type":"array"}}}"#.to_string(),
                tags: vec!["memory".to_string(), "embeddings".to_string()],
                estimated_tokens: 60,
                source_uri: "https://registry.liquid-harness.local/tools/vector_memory_bus.json".to_string(),
            },
            ToolDefinition {
                id: "ephemeral_sandbox_runner".to_string(),
                name: "Ephemeral Sandbox Runner".to_string(),
                version: "1.0.0".to_string(),
                description: "Lightweight isolated probe execution.".to_string(),
                execution_handler: "module://sandbox/runner".to_string(),
                schema_json: r#"{"type":"object","properties":{"code":{"type":"string"},"timeout_ms":{"type":"integer"}}}"#.to_string(),
                tags: vec!["sandbox".to_string(), "staging".to_string()],
                estimated_tokens: 70,
                source_uri: "https://registry.liquid-harness.local/tools/ephemeral_sandbox_runner.json".to_string(),
            },
            ToolDefinition {
                id: "prompt_compressor".to_string(),
                name: "Prompt Compressor".to_string(),
                version: "1.0.0".to_string(),
                description: "Semantic token compression unit.".to_string(),
                execution_handler: "module://optimizer/prompt_compressor".to_string(),
                schema_json: r#"{"type":"object","properties":{"prompt":{"type":"string"}}}"#.to_string(),
                tags: vec!["optimizer".to_string(), "prompt".to_string()],
                estimated_tokens: 50,
                source_uri: "https://registry.liquid-harness.local/tools/prompt_compressor.json".to_string(),
            },
        ];

        for tool in day1 {
            tools.insert(tool.id.clone(), tool);
        }

        // Seed Tracks
        let mut tracks = self.tracks.write().unwrap();
        tracks.insert(
            "prompt_optimization".to_string(),
            TrackDefinition {
                id: "prompt_optimization".to_string(),
                name: "Prompt Footprint Optimization".to_string(),
                target_component: "prompt".to_string(),
                benchmark_adapter: "agent_eval".to_string(),
                generator: "failure_summary".to_string(),
                policy: TrackPolicy {
                    screening_min_score: 1.0,
                    validation_min_win: 0.05,
                    min_stability_threshold: 0.85,
                    confidence_level: 0.95,
                },
            },
        );

        tracks.insert(
            "tool_synthesis".to_string(),
            TrackDefinition {
                id: "tool_synthesis".to_string(),
                name: "Dynamic Tool Evolution".to_string(),
                target_component: "tool".to_string(),
                benchmark_adapter: "cargo_test".to_string(),
                generator: "template".to_string(),
                policy: TrackPolicy {
                    screening_min_score: 1.0,
                    validation_min_win: 0.02,
                    min_stability_threshold: 0.80,
                    confidence_level: 0.95,
                },
            },
        );

        tracks.insert(
            "security_hardening".to_string(),
            TrackDefinition {
                id: "security_hardening".to_string(),
                name: "Zero-Trust Security Barrier".to_string(),
                target_component: "guardrail".to_string(),
                benchmark_adapter: "generic_command".to_string(),
                generator: "template".to_string(),
                policy: TrackPolicy {
                    screening_min_score: 1.0,
                    validation_min_win: 0.0,
                    min_stability_threshold: 0.90,
                    confidence_level: 0.95,
                },
            },
        );
    }

    pub fn get_tool(&self, id: &str) -> Option<ToolDefinition> {
        let tools = self.tools.read().unwrap();
        tools.get(id).cloned()
    }

    pub fn list_tools(&self) -> Vec<ToolDefinition> {
        let tools = self.tools.read().unwrap();
        tools.values().cloned().collect()
    }

    pub fn register_tool(&self, tool: ToolDefinition) {
        let mut tools = self.tools.write().unwrap();
        tools.insert(tool.id.clone(), tool);
    }

    pub fn get_track(&self, id: &str) -> Option<TrackDefinition> {
        let tracks = self.tracks.read().unwrap();
        tracks.get(id).cloned()
    }

    pub fn list_tracks(&self) -> Vec<TrackDefinition> {
        let tracks = self.tracks.read().unwrap();
        tracks.values().cloned().collect()
    }

    pub fn scout_candidate(&self, track_id: &str) -> Option<ToolDefinition> {
        match track_id {
            "prompt_optimization" => Some(ToolDefinition {
                id: "prompt_compressor".to_string(),
                name: "Prompt Compressor (Fast & Slim)".to_string(),
                version: "2.0.0".to_string(),
                description: "Optimized AST-aware prompt compressor with 30% smaller token footprint.".to_string(),
                execution_handler: "(x) => x.replace(/\\s+/g, ' ').trim()".to_string(),
                schema_json: r#"{"type":"object"}"#.to_string(),
                tags: vec!["optimizer".to_string(), "v2".to_string()],
                estimated_tokens: 48,
                source_uri: "registry://scout/prompt_compressor_v2.json".to_string(),
            }),
            _ => None,
        }
    }
}

use crate::critic::SecurityCritic;
use crate::optimizer::PromptOptimizer;
use crate::plugin::{LiquidPlugin, ManifestPlugin, PluginExecutionContext, SlotRegistry};
use crate::registry::Registry;
use crate::stages::StagePipelineRunner;
use crate::types::{
    AgentPhase, AgentState, AgentStateSnapshot, AuditVerdict, ChampionRecord, PipelineConfig,
    PluginManifest, PluginStatus, SlotKind, ToolDefinition,
};
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;

pub struct AgentKernel {
    state: AgentState,
    registry: Registry,
    critic: SecurityCritic,
    pipeline: StagePipelineRunner,
    optimizer: PromptOptimizer,
    slots: SlotRegistry,
}

impl AgentKernel {
    pub fn boot(agent_id: &str, registry: Registry) -> Self {
        let baseline_tool = registry
            .get_tool("discover_harness_capabilities")
            .unwrap_or_else(|| ToolDefinition {
                id: "discover_harness_capabilities".to_string(),
                name: "Discover Harness Capabilities".to_string(),
                version: "1.0.0".to_string(),
                description: "Query the micro-kernel registry to find dynamic skills.".to_string(),
                execution_handler: "builtin://discovery".to_string(),
                schema_json: "{}".to_string(),
                tags: vec!["kernel".to_string()],
                estimated_tokens: 45,
                source_uri: "kernel://bootstrap/discovery.json".to_string(),
            });

        let mut components = HashMap::new();
        components.insert("system_prompt".to_string(), "Minimalist Liquid Kernel Baseline".to_string());
        components.insert("guardrails".to_string(), "Strict zero-trust execution".to_string());
        components.insert("memory".to_string(), "ephemeral-local".to_string());

        let state = AgentState {
            agent_id: agent_id.to_string(),
            phase: AgentPhase::Boot,
            generation: 1,
            active_tools: vec![baseline_tool],
            components,
            champion: None,
            rollback_stack: Vec::new(),
            created_at: Utc::now(),
        };

        Self {
            state,
            registry,
            critic: SecurityCritic::new(),
            pipeline: StagePipelineRunner::default(),
            optimizer: PromptOptimizer::new(),
            slots: SlotRegistry::new(),
        }
    }

    pub fn state(&self) -> &AgentState {
        &self.state
    }

    pub fn slots(&self) -> &SlotRegistry {
        &self.slots
    }

    pub fn inject_plugin(&mut self, plugin: Arc<dyn LiquidPlugin>) -> Result<PluginStatus, String> {
        let status = self.slots.register_plugin(plugin.clone(), &self.critic)?;
        if status == PluginStatus::Active && plugin.manifest().slot == SlotKind::Tool {
            let m = plugin.manifest();
            let tool = ToolDefinition {
                id: m.id.clone(),
                name: m.name.clone(),
                version: m.version.clone(),
                description: m.description.clone(),
                execution_handler: format!("plugin://{}", m.id),
                schema_json: m.schema_json.clone(),
                tags: m.tags.clone(),
                estimated_tokens: m.estimated_tokens,
                source_uri: format!("slot://capabilities/{}", m.id),
            };
            if !self.state.active_tools.iter().any(|t| t.id == tool.id) {
                self.state.active_tools.push(tool);
            }
        }
        Ok(status)
    }

    pub fn inject_manifest(&mut self, manifest: PluginManifest) -> Result<PluginStatus, String> {
        let plugin = Arc::new(ManifestPlugin::new(manifest));
        self.inject_plugin(plugin)
    }

    pub fn execute_plugin(&self, plugin_id: &str, payload: &str) -> Result<String, String> {
        let ctx = PluginExecutionContext {
            agent_id: self.state.agent_id.clone(),
            generation: self.state.generation,
            working_dir: ".".to_string(),
        };
        self.slots.execute(plugin_id, payload, &ctx)
    }

    /// Phase 2: Self-Assembling Day-1 Harness
    pub fn assemble_day1(&mut self) -> &[ToolDefinition] {
        let day1_ids = [
            "ast_sanitizer",
            "vector_memory_bus",
            "ephemeral_sandbox_runner",
            "prompt_compressor",
        ];

        for id in day1_ids {
            if let Some(tool) = self.registry.get_tool(id) {
                if !self.state.active_tools.iter().any(|t| t.id == tool.id) {
                    self.state.active_tools.push(tool);
                }
            }
        }

        self.state.phase = AgentPhase::Assembled;
        self.state.generation = 2;
        &self.state.active_tools
    }

    /// Phase 3: The 24h Scout
    pub fn trigger_scout(&mut self, track_id: &str) -> Option<ToolDefinition> {
        self.state.phase = AgentPhase::Scouting;
        self.registry.scout_candidate(track_id)
    }

    /// Phase 4 & 5: RRSI Multi-Stage Auditing and Hot-Swap Rollout
    pub fn audit_and_hotswap(
        &mut self,
        candidate_tool: ToolDefinition,
        proposal_diff: &str,
    ) -> Result<ChampionRecord, String> {
        self.state.phase = AgentPhase::Auditing;

        // Gate 1: Rust Security Critic
        let audit_report = self.critic.audit(
            &format!("prop-{}", candidate_tool.id),
            &candidate_tool.description,
            &candidate_tool.schema_json,
        );

        if audit_report.verdict == AuditVerdict::Rejected {
            return Err(format!(
                "Security Critic rejected candidate: {:?}",
                audit_report.violations
            ));
        }

        let diff_report = self
            .critic
            .audit_mutation_diff(&format!("diff-{}", candidate_tool.id), proposal_diff);
        if !diff_report.safe_to_stage {
            return Err(format!(
                "Security Critic diff risk level {:?} exceeds safety barrier",
                diff_report.risk_level
            ));
        }

        // Gate 2: AutoHarness 3-Stage Pipeline
        let baseline_tool = self
            .state
            .active_tools
            .iter()
            .find(|t| t.id == candidate_tool.id)
            .cloned()
            .unwrap_or_else(|| self.state.active_tools[0].clone());

        let pipeline_result = self.pipeline.run_pipeline(&PipelineConfig {
            proposal_id: format!("audit-{}", candidate_tool.id),
            baseline_tool,
            candidate_tool: candidate_tool.clone(),
            screening_input: Some("   test   stream  ".to_string()),
            validation_inputs: Some(vec![
                "  hello   liquid  ".to_string(),
                "   alpha   beta  ".to_string(),
                "   gamma   delta  ".to_string(),
            ]),
            holdout_inputs: Some(vec![
                " holdout task 1 ".to_string(),
                " holdout task 2 ".to_string(),
            ]),
            min_validation_win: Some(0.04),
            min_stability_threshold: Some(0.85),
        });

        if !pipeline_result.passed {
            return Err(format!(
                "Multi-stage pipeline rejected candidate: {}",
                pipeline_result.verdict_reason
            ));
        }

        // Gate 3: Token Budget Evaluation
        let baseline_tokens = 70;
        let candidate_tokens = candidate_tool.estimated_tokens;
        let (token_delta, budget_passed) =
            self.optimizer.evaluate_budget(baseline_tokens, candidate_tokens, 100);

        if !budget_passed && token_delta > 0 {
            return Err(format!(
                "Token budget exceeded: delta=+{} tokens",
                token_delta
            ));
        }

        // Gate 4: Hot-Swap Rollout with Snapshot Push
        let snapshot = AgentStateSnapshot {
            generation: self.state.generation,
            tools: self.state.active_tools.clone(),
            score: self.state.champion.as_ref().map(|c| c.score).unwrap_or(0.85),
            timestamp: Utc::now(),
        };
        self.state.rollback_stack.push(snapshot);

        // Atomically replace the tool
        let mut replaced = false;
        for t in self.state.active_tools.iter_mut() {
            if t.id == candidate_tool.id {
                *t = candidate_tool.clone();
                replaced = true;
                break;
            }
        }
        if !replaced {
            self.state.active_tools.push(candidate_tool.clone());
        }

        self.state.generation += 1;
        self.state.phase = AgentPhase::Active;

        let champion = ChampionRecord {
            generation: self.state.generation,
            proposal_id: format!("champ-gen-{}", self.state.generation),
            track_id: "prompt_optimization".to_string(),
            score: 0.96,
            token_footprint: candidate_tokens,
            promoted_at: Utc::now(),
        };

        self.state.champion = Some(champion.clone());
        Ok(champion)
    }

    /// Rollback champion atomically if regression observed
    pub fn rollback_champion(&mut self, reason: &str) -> Result<usize, String> {
        if let Some(prev) = self.state.rollback_stack.pop() {
            self.state.active_tools = prev.tools;
            self.state.generation = prev.generation;
            self.state.phase = AgentPhase::RolledBack;
            tracing::warn!(
                "Rolled back agent state to generation {} (Reason: {})",
                prev.generation,
                reason
            );
            Ok(prev.generation)
        } else {
            Err("No previous state in rollback stack to revert to".to_string())
        }
    }
}

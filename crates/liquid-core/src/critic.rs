use crate::types::{AuditReport, AuditVerdict, MutationRiskLevel, MutationRiskReport};
use regex::Regex;
use std::time::Instant;

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
        } else if violations.iter().any(|v| {
            v.contains("Direct process") || v.contains("Destructive") || v.contains("Credential")
        }) {
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

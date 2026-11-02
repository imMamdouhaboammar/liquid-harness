use regex::Regex;
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuditVerdict {
    Passed,
    Rejected,
    RequiresSanitization,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditReport {
    pub proposal_id: String,
    pub verdict: AuditVerdict,
    pub violations: Vec<String>,
    pub sanitized_content: String,
    pub latency_us: u64,
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
            max_payload_bytes: 65_536, // 64 KB memory boundary
            max_json_depth: 8,
        }
    }

    pub fn audit(&self, proposal_id: &str, content: &str, schema_json: &str) -> AuditReport {
        let start = Instant::now();
        let mut violations = Vec::new();

        // 1. Memory Boundary Check
        if content.len() > self.max_payload_bytes {
            violations.push(format!(
                "Payload size ({} bytes) exceeds maximum memory limit of {} bytes",
                content.len(),
                self.max_payload_bytes
            ));
        }

        // 2. Static Regex Denylist Checks
        for (re, desc) in &self.denylist_patterns {
            if re.is_match(content) {
                violations.push(desc.to_string());
            }
        }

        // 3. Schema AST & Nesting Depth Sanitization
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

        // 4. Determine Verdict & Sanitized Output
        let verdict = if violations.is_empty() {
            AuditVerdict::Passed
        } else if violations.iter().any(|v| v.contains("Direct process") || v.contains("Destructive") || v.contains("Credential")) {
            AuditVerdict::Rejected
        } else {
            // Apply defensive sanitization: strip dangerous control characters
            sanitized = sanitized.replace("\0", "").replace("\r", "");
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
}

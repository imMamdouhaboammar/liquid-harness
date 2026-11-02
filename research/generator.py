"""
Proposal Generator Engine inspired by AutoHarness.
Includes Failure-Driven and Template-Driven mutation generators.
"""

from __future__ import annotations
import hashlib
import re
from typing import Any, Dict, List, Optional
from pydantic import BaseModel, Field

class GeneratedProposal(BaseModel):
    proposal_id: str
    track_id: str
    generator_type: str
    target_component: str
    rationale: str
    diff: str
    proposed_content: str
    estimated_tokens: int
    metadata: Dict[str, Any] = Field(default_factory=dict)

class ProposalRequest(BaseModel):
    track_id: str
    target_component: str = "prompt"  # prompt | tool | middleware | guardrail
    failure_log: Optional[str] = None
    current_content: Optional[str] = None
    template_name: Optional[str] = None

class FailureDrivenGenerator:
    """
    Parses benchmark error logs and stack traces to synthesize targeted harness mutations.
    """

    ERROR_PATTERNS = [
        ("timeout", re.compile(r"(?i)(timeout|timed\s*out|deadline\s*exceeded)")),
        ("syntax_error", re.compile(r"(?i)(syntaxerror|parse\s*error|invalid\s*syntax)")),
        ("token_bloat", re.compile(r"(?i)(token.*limit|context.*length|maximum.*token|token.*bloat)")),
        ("security_violation", re.compile(r"(?i)(security|denylist|traversal|exec|system)")),
        ("assertion_failure", re.compile(r"(?i)(assertionerror|expected\s+.*,\s+got\s+.*|failed\s+assertion)")),
    ]

    def classify_failure(self, failure_log: str) -> str:
        for error_type, pattern in self.ERROR_PATTERNS:
            if pattern.search(failure_log):
                return error_type
        return "general_regression"

    def generate(self, track_id: str, target_component: str, failure_log: str, current_content: str = "") -> GeneratedProposal:
        error_type = self.classify_failure(failure_log)
        proposal_hash = hashlib.sha256(f"{track_id}:{error_type}:{failure_log[:100]}".encode()).hexdigest()[:8]
        proposal_id = f"prop-fail-{error_type}-{proposal_hash}"

        if error_type == "timeout":
            rationale = "Benchmark timed out. Injecting fast-exit short-circuit and bounded loop guards."
            diff = "--- a/harness\n+++ b/harness\n@@ -1 +1,2 @@\n+timeout_ms: 1500\n+short_circuit_on_empty: true\n"
            proposed_content = current_content + "\n# Guard: Fast exit on idle\nif not input_payload: return ''"
        elif error_type == "token_bloat":
            rationale = "Context token limit exceeded. Applying aggressive prompt compression and removing conversational fluff."
            diff = "--- a/prompt\n+++ b/prompt\n@@ -1,5 +1,2 @@\n-You are a polite AI assistant...\n-Please remember that...\n+# Direct Payload Instructions\n"
            cleaned = re.sub(r"(?i)\b(you are an advanced ai|please note that|remember to be polite)\b", "", current_content)
            proposed_content = "# Instructions\n" + cleaned.strip()
        elif error_type == "security_violation":
            rationale = "Security Critic rejected unvalidated input. Injecting AST boundary validator and regex sanitizer."
            diff = "--- a/middleware\n+++ b/middleware\n@@ -1 +1,3 @@\n+validate_ast_depth(max_depth=8)\n+strip_control_characters()\n"
            proposed_content = current_content + "\n# Security Barrier: AST depth check\nassert calculate_depth(payload) <= 8"
        else:
            rationale = f"Detected {error_type}. Synthesizing defensive error boundary and retry policy."
            diff = f"--- a/adapter\n+++ b/adapter\n@@ -1 +1,2 @@\n+# Fix for {error_type}\n+retry_with_backoff(max_retries=2)\n"
            proposed_content = current_content + f"\n# Automated patch for {error_type}"

        words = len(proposed_content.split())
        estimated_tokens = max(1, int(words * 1.25))

        return GeneratedProposal(
            proposal_id=proposal_id,
            track_id=track_id,
            generator_type="failure_driven",
            target_component=target_component,
            rationale=rationale,
            diff=diff,
            proposed_content=proposed_content,
            estimated_tokens=estimated_tokens,
            metadata={"classified_error": error_type, "original_log_snippet": failure_log[:150]},
        )

class TemplateProposalGenerator:
    """
    Applies deterministic, verified transformation templates.
    """

    TEMPLATES = {
        "compress_prompt": {
            "rationale": "Enforce Zero-Obviousness & Direct Payload doctrine to reduce token footprint.",
            "diff": "--- a/prompt\n+++ b/prompt\n-Conversational fluff\n+Compact imperative directives",
            "content": "# Core Instructions\nDirect execution only. Return compact structured output.",
            "component": "prompt",
        },
        "circuit_breaker": {
            "rationale": "Add Fable Circuit Breaker: halt speculative mutations after 2 consecutive failures.",
            "diff": "--- a/circuit_breaker\n+++ b/circuit_breaker\n+if failure_streak >= 2: break",
            "content": "failure_streak_limit = 2\nfence_cancelled_workers = True",
            "component": "middleware",
        },
        "ast_guard": {
            "rationale": "Inject strict Rust-compatible AST payload memory and recursion bounds.",
            "diff": "--- a/guardrails\n+++ b/guardrails\n+max_payload_bytes: 65536\n+max_depth: 8",
            "content": "denylist = ['exec', 'system', 'rm -rf', 'BEGIN PRIVATE KEY']",
            "component": "guardrail",
        },
    }

    def generate(self, track_id: str, template_name: str) -> GeneratedProposal:
        tpl = self.TEMPLATES.get(template_name, self.TEMPLATES["compress_prompt"])
        proposal_id = f"prop-tpl-{template_name}-{hashlib.md5(track_id.encode()).hexdigest()[:6]}"
        words = len(tpl["content"].split())
        estimated_tokens = max(1, int(words * 1.25))

        return GeneratedProposal(
            proposal_id=proposal_id,
            track_id=track_id,
            generator_type="template_driven",
            target_component=tpl["component"],
            rationale=tpl["rationale"],
            diff=tpl["diff"],
            proposed_content=tpl["content"],
            estimated_tokens=estimated_tokens,
            metadata={"template_name": template_name},
        )

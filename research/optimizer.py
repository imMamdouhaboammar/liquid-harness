import re
from typing import Dict, List, Optional
from pydantic import BaseModel

class ToolDef(BaseModel):
    id: str
    name: str
    description: str
    version: str
    estimated_tokens: int

class PromptOptimizeRequest(BaseModel):
    agent_id: str
    current_prompt: str
    active_tools: List[ToolDef] = []
    max_target_tokens: Optional[int] = 1000

class PromptOptimizeResponse(BaseModel):
    optimized_prompt: str
    original_tokens: int
    optimized_tokens: int
    compression_ratio: float

class TokenBudgetRequest(BaseModel):
    baseline_tokens: int
    proposed_tokens: int
    max_allowed_budget: int

class TokenBudgetResponse(BaseModel):
    budget_passed: bool
    token_delta: int
    reason: str

class PromptOptimizer:
    """
    AST-aware and semantic prompt compressor.
    Enforces the Zero-Obviousness & Direct Payload doctrine from anti-slop-engineering:
    Removes conversational preambles, tautological phrasing, and compresses instructions.
    """

    FLUFF_PATTERNS = [
        re.compile(r"(?i)\b(as an ai agent,?\s*|you are an advanced language model,?\s*)"),
        re.compile(r"(?i)\b(please note that\s*|it is important to remember that\s*)"),
        re.compile(r"(?i)\b(the following section describes\s*|in order to achieve this,\s*)"),
        re.compile(r"\n{3,}"),
    ]

    @staticmethod
    def estimate_tokens(text: str) -> int:
        # Approximate 1 token ~= 4 characters or word count * 1.3
        words = len(text.split())
        return max(1, int(words * 1.25))

    def optimize(self, current_prompt: str, active_tools: List[ToolDef]) -> PromptOptimizeResponse:
        original_tokens = self.estimate_tokens(current_prompt)

        # 1. Clean conversational fluff
        cleaned = current_prompt
        for pat in self.FLUFF_PATTERNS:
            cleaned = pat.sub("", cleaned)

        # 2. Re-structure into structured minimal payload
        lines = [line.strip() for line in cleaned.splitlines() if line.strip()]
        
        # Deduplicate consecutive identical directives
        seen = set()
        deduped_lines = []
        for line in lines:
            if line not in seen:
                seen.add(line)
                deduped_lines.append(line)

        # Append compact tool manifest
        tool_manifest = []
        for tool in active_tools:
            tool_manifest.append(f"- {tool.id} (v{tool.version}): {tool.description}")

        sections = []
        sections.append("# Core Instructions\n" + "\n".join(deduped_lines))
        if tool_manifest:
            sections.append("# Mounted Capabilities\n" + "\n".join(tool_manifest))

        optimized_prompt = "\n\n".join(sections)
        optimized_tokens = self.estimate_tokens(optimized_prompt)

        ratio = round(optimized_tokens / max(1, original_tokens), 3)

        return PromptOptimizeResponse(
            optimized_prompt=optimized_prompt,
            original_tokens=original_tokens,
            optimized_tokens=optimized_tokens,
            compression_ratio=ratio,
        )

    def evaluate_budget(self, baseline: int, proposed: int, max_budget: int) -> TokenBudgetResponse:
        delta = proposed - baseline

        if proposed > max_budget:
            return TokenBudgetResponse(
                budget_passed=False,
                token_delta=delta,
                reason=f"Proposed token footprint ({proposed}) exceeds maximum allowed budget ({max_budget})",
            )

        # RRSI Constraint: If proposal increases tokens by > 40% without proven gain, reject
        if baseline > 0 and (delta / baseline) > 0.40:
            return TokenBudgetResponse(
                budget_passed=False,
                token_delta=delta,
                reason=f"Token bloat hazard: proposal adds {delta} tokens (+{int((delta/baseline)*100)}%), violating RRSI anti-bloat rule",
            )

        return TokenBudgetResponse(
            budget_passed=True,
            token_delta=delta,
            reason="Token budget criteria passed with optimal footprint",
        )

"""
Root Memory & Transfer Learning Manager inspired by AutoHarness.
Maintains persistent cross-campaign memory of failure signatures, winning interventions,
and transferable evolutionary strategies.
"""

from __future__ import annotations
import json
import os
import time
from typing import Any, Dict, List, Optional
from pydantic import BaseModel, Field

class InterventionRecord(BaseModel):
    proposal_id: str
    track_id: str
    target_component: str
    generator_type: str
    rationale: str
    score_delta: float
    token_delta: int
    promoted_at: float
    transferability_score: float = 1.0

class FailureFingerprint(BaseModel):
    error_signature: str
    attempt_count: int = 1
    rejected_proposals: List[str] = Field(default_factory=list)
    reasons: List[str] = Field(default_factory=list)
    last_seen_at: float = Field(default_factory=time.time)

class RootMemoryState(BaseModel):
    version: str = "1.0.0"
    total_campaigns: int = 0
    champions_promoted: int = 0
    winning_interventions: List[InterventionRecord] = Field(default_factory=list)
    failure_fingerprints: Dict[str, FailureFingerprint] = Field(default_factory=dict)
    metadata: Dict[str, Any] = Field(default_factory=dict)

class RootMemoryManager:
    def __init__(self, storage_path: Optional[str] = None):
        self.storage_path = storage_path or ".liquid/root_memory.json"
        self.state = RootMemoryState()
        self.load()

    def record_champion(
        self,
        proposal_id: str,
        track_id: str,
        target_component: str,
        generator_type: str,
        rationale: str,
        score_delta: float,
        token_delta: int,
    ) -> InterventionRecord:
        record = InterventionRecord(
            proposal_id=proposal_id,
            track_id=track_id,
            target_component=target_component,
            generator_type=generator_type,
            rationale=rationale,
            score_delta=score_delta,
            token_delta=token_delta,
            promoted_at=time.time(),
            transferability_score=min(1.0, max(0.5, score_delta * 2.0)),
        )
        self.state.winning_interventions.append(record)
        self.state.champions_promoted += 1
        self.save()
        return record

    def record_failure(self, error_signature: str, proposal_id: str, reason: str) -> FailureFingerprint:
        if error_signature in self.state.failure_fingerprints:
            fp = self.state.failure_fingerprints[error_signature]
            fp.attempt_count += 1
            if proposal_id not in fp.rejected_proposals:
                fp.rejected_proposals.append(proposal_id)
            if reason not in fp.reasons:
                fp.reasons.append(reason)
            fp.last_seen_at = time.time()
        else:
            fp = FailureFingerprint(
                error_signature=error_signature,
                attempt_count=1,
                rejected_proposals=[proposal_id],
                reasons=[reason],
                last_seen_at=time.time(),
            )
            self.state.failure_fingerprints[error_signature] = fp

        self.save()
        return fp

    def get_transfer_suggestions(self, track_id: str) -> List[Dict[str, Any]]:
        suggestions = []
        for winner in sorted(self.state.winning_interventions, key=lambda w: w.transferability_score, reverse=True):
            suggestions.append({
                "strategy": f"Transfer proven champion from track '{winner.track_id}'",
                "target_component": winner.target_component,
                "rationale": winner.rationale,
                "expected_score_gain": winner.score_delta,
                "token_savings": abs(winner.token_delta) if winner.token_delta < 0 else 0,
                "confidence": winner.transferability_score,
            })
        if not suggestions:
            suggestions.append({
                "strategy": "Apply Zero-Obviousness prompt compression & AST security fence",
                "target_component": "prompt",
                "rationale": "Default high-leverage baseline optimization template",
                "expected_score_gain": 0.15,
                "token_savings": 25,
                "confidence": 0.90,
            })
        return suggestions

    def get_summary(self) -> Dict[str, Any]:
        return {
            "total_campaigns": self.state.total_campaigns,
            "champions_promoted": self.state.champions_promoted,
            "failure_clusters": len(self.state.failure_fingerprints),
            "recent_winners": [w.model_dump() for w in self.state.winning_interventions[-5:]],
        }

    def save(self) -> None:
        try:
            os.makedirs(os.path.dirname(self.storage_path) or ".", exist_ok=True)
            with open(self.storage_path, "w", encoding="utf-8") as f:
                f.write(self.state.model_dump_json(indent=2))
        except Exception:
            pass

    def load(self) -> None:
        if os.path.exists(self.storage_path):
            try:
                with open(self.storage_path, "r", encoding="utf-8") as f:
                    data = json.load(f)
                    self.state = RootMemoryState(**data)
            except Exception:
                pass

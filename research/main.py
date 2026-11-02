import uvicorn
from fastapi import FastAPI, HTTPException
from optimizer import (
    PromptOptimizer,
    PromptOptimizeRequest,
    PromptOptimizeResponse,
    TokenBudgetRequest,
    TokenBudgetResponse,
)
from generator import (
    FailureDrivenGenerator,
    TemplateProposalGenerator,
    ProposalRequest,
    GeneratedProposal,
)
from root_memory import RootMemoryManager, InterventionRecord

app = FastAPI(title="Liquid Harness - Inference & Optimizer Engine", version="1.1.0")

optimizer = PromptOptimizer()
failure_generator = FailureDrivenGenerator()
template_generator = TemplateProposalGenerator()
memory_manager = RootMemoryManager()

@app.get("/health")
def health():
    return {"status": "ok", "service": "liquid-optimizer-python", "version": "1.1.0"}

@app.post("/api/v1/optimize-prompt", response_model=PromptOptimizeResponse)
def optimize_prompt_endpoint(req: PromptOptimizeRequest):
    return optimizer.optimize(req.current_prompt, req.active_tools)

@app.post("/api/v1/evaluate-token-budget", response_model=TokenBudgetResponse)
def evaluate_budget_endpoint(req: TokenBudgetRequest):
    return optimizer.evaluate_budget(req.baseline_tokens, req.proposed_tokens, req.max_allowed_budget)

@app.post("/api/v1/proposals/generate", response_model=GeneratedProposal)
def generate_proposal_endpoint(req: ProposalRequest):
    if req.failure_log:
        return failure_generator.generate(
            track_id=req.track_id,
            target_component=req.target_component,
            failure_log=req.failure_log,
            current_content=req.current_content or "",
        )
    elif req.template_name:
        return template_generator.generate(
            track_id=req.track_id,
            template_name=req.template_name,
        )
    else:
        return template_generator.generate(
            track_id=req.track_id,
            template_name="compress_prompt",
        )

@app.post("/api/v1/memory/record-champion")
def record_champion_endpoint(record: InterventionRecord):
    return memory_manager.record_champion(
        proposal_id=record.proposal_id,
        track_id=record.track_id,
        target_component=record.target_component,
        generator_type=record.generator_type,
        rationale=record.rationale,
        score_delta=record.score_delta,
        token_delta=record.token_delta,
    )

@app.get("/api/v1/memory/summary")
def memory_summary_endpoint():
    return memory_manager.get_summary()

@app.get("/api/v1/memory/transfer-suggestions")
def transfer_suggestions_endpoint(track_id: str = "general"):
    return memory_manager.get_transfer_suggestions(track_id)

if __name__ == "__main__":
    uvicorn.run(app, host="127.0.0.1", port=50054, log_level="warning")

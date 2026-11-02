import uvicorn
from fastapi import FastAPI
from optimizer import (
    PromptOptimizer,
    PromptOptimizeRequest,
    PromptOptimizeResponse,
    TokenBudgetRequest,
    TokenBudgetResponse,
)

app = FastAPI(title="Liquid Harness - Inference & Optimizer Engine", version="1.0.0")
optimizer = PromptOptimizer()

@app.get("/health")
def health():
    return {"status": "ok", "service": "liquid-optimizer-python"}

@app.post("/api/v1/optimize-prompt", response_model=PromptOptimizeResponse)
def optimize_prompt_endpoint(req: PromptOptimizeRequest):
    return optimizer.optimize(req.current_prompt, req.active_tools)

@app.post("/api/v1/evaluate-token-budget", response_model=TokenBudgetResponse)
def evaluate_budget_endpoint(req: TokenBudgetRequest):
    return optimizer.evaluate_budget(req.baseline_tokens, req.proposed_tokens, req.max_allowed_budget)

if __name__ == "__main__":
    uvicorn.run(app, host="127.0.0.1", port=50054, log_level="warning")

import unittest
from optimizer import PromptOptimizer, ToolDef

class TestPromptOptimizer(unittest.TestCase):
    def setUp(self):
        self.optimizer = PromptOptimizer()

    def test_prompt_optimization_removes_fluff(self):
        raw_prompt = (
            "As an AI agent, please note that you must solve math problems.\n"
            "It is important to remember that you should be accurate.\n"
            "In order to achieve this, check your calculations twice.\n"
            "In order to achieve this, check your calculations twice."
        )
        tools = [
            ToolDef(
                id="math_calc",
                name="Math Calc",
                description="High precision calculation unit",
                version="1.0.0",
                estimated_tokens=50,
            )
        ]

        res = self.optimizer.optimize(raw_prompt, tools)
        self.assertNotIn("As an AI agent", res.optimized_prompt)
        self.assertIn("# Core Instructions", res.optimized_prompt)
        self.assertIn("math_calc", res.optimized_prompt)
        self.assertLessEqual(res.optimized_tokens, res.original_tokens + 25)

    def test_token_budget_rrsi_bloat_rejection(self):
        # Baseline = 100 tokens, Proposed = 200 tokens (+100%), max budget = 500
        res = self.optimizer.evaluate_budget(baseline=100, proposed=200, max_budget=500)
        self.assertFalse(res.budget_passed)
        self.assertIn("Token bloat hazard", res.reason)

    def test_token_budget_passed(self):
        res = self.optimizer.evaluate_budget(baseline=100, proposed=110, max_budget=500)
        self.assertTrue(res.budget_passed)

if __name__ == "__main__":
    unittest.main()

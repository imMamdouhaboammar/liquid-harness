import os
import shutil
import tempfile
import unittest
from optimizer import PromptOptimizer, ToolDef
from generator import FailureDrivenGenerator, TemplateProposalGenerator
from root_memory import RootMemoryManager

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

class TestProposalGenerators(unittest.TestCase):
    def setUp(self):
        self.failure_gen = FailureDrivenGenerator()
        self.template_gen = TemplateProposalGenerator()

    def test_failure_driven_generator_timeout(self):
        failure_log = "Error: execution timed out after 5000ms in worker process"
        proposal = self.failure_gen.generate(
            track_id="track-latency",
            target_component="middleware",
            failure_log=failure_log,
            current_content="def execute(): pass"
        )
        self.assertEqual(proposal.generator_type, "failure_driven")
        self.assertIn("timeout", proposal.proposal_id)
        self.assertIn("timeout_ms", proposal.diff)
        self.assertGreater(proposal.estimated_tokens, 0)

    def test_failure_driven_generator_token_bloat(self):
        failure_log = "Error: maximum context tokens limit exceeded (8192 tokens)"
        proposal = self.failure_gen.generate(
            track_id="track-prompt",
            target_component="prompt",
            failure_log=failure_log,
            current_content="You are an advanced AI assistant. Please note that you must follow rules."
        )
        self.assertIn("token_bloat", proposal.proposal_id)
        self.assertIn("prompt compression", proposal.rationale)

    def test_template_generator(self):
        proposal = self.template_gen.generate(
            track_id="track-general",
            template_name="circuit_breaker"
        )
        self.assertEqual(proposal.generator_type, "template_driven")
        self.assertIn("circuit_breaker", proposal.proposal_id)
        self.assertIn("failure_streak", proposal.diff)

class TestRootMemoryManager(unittest.TestCase):
    def setUp(self):
        self.test_dir = tempfile.mkdtemp()
        self.mem_file = os.path.join(self.test_dir, "root_memory.json")
        self.memory = RootMemoryManager(storage_path=self.mem_file)

    def tearDown(self):
        shutil.rmtree(self.test_dir, ignore_errors=True)

    def test_record_champion_and_persistence(self):
        rec = self.memory.record_champion(
            proposal_id="prop-win-1",
            track_id="track-prompt",
            target_component="prompt",
            generator_type="failure_driven",
            rationale="Saved 40 tokens with zero accuracy loss",
            score_delta=0.12,
            token_delta=-40,
        )
        self.assertEqual(rec.proposal_id, "prop-win-1")
        self.assertTrue(os.path.exists(self.mem_file))

        # Re-load in fresh manager
        reloaded = RootMemoryManager(storage_path=self.mem_file)
        self.assertEqual(reloaded.state.champions_promoted, 1)
        self.assertEqual(len(reloaded.state.winning_interventions), 1)

    def test_record_failure_clusters(self):
        fp1 = self.memory.record_failure("timeout_err_sig", "prop-1", "Stage 1 probe timeout")
        fp2 = self.memory.record_failure("timeout_err_sig", "prop-2", "Stage 1 probe timeout")

        self.assertEqual(fp2.attempt_count, 2)
        self.assertEqual(len(fp2.rejected_proposals), 2)

    def test_transfer_suggestions(self):
        self.memory.record_champion(
            proposal_id="prop-win-best",
            track_id="track-speed",
            target_component="middleware",
            generator_type="template",
            rationale="Circuit breaker",
            score_delta=0.25,
            token_delta=-10,
        )
        suggestions = self.memory.get_transfer_suggestions("new-track")
        self.assertGreater(len(suggestions), 0)
        self.assertEqual(suggestions[0]["target_component"], "middleware")

if __name__ == "__main__":
    unittest.main()

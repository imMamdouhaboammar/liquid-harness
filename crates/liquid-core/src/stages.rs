use crate::evaluator::StatisticalEvaluator;
use crate::sandbox::EphemeralSandbox;
use crate::types::{PipelineConfig, PipelineResult, StageName, StageResult};
use std::time::Instant;

pub struct StagePipelineRunner {
    sandbox: EphemeralSandbox,
}

impl Default for StagePipelineRunner {
    fn default() -> Self {
        Self::new(EphemeralSandbox::default())
    }
}

impl StagePipelineRunner {
    pub fn new(sandbox: EphemeralSandbox) -> Self {
        Self { sandbox }
    }

    pub fn run_pipeline(&self, config: &PipelineConfig) -> PipelineResult {
        let pipeline_start = Instant::now();

        // --- Stage 1: Screening ---
        let screening_start = Instant::now();
        let screen_input = config
            .screening_input
            .as_deref()
            .unwrap_or("screening_sample_payload");
        let screen_probe = self.sandbox.execute_probe(
            &format!("screen-{}", config.proposal_id),
            &config.candidate_tool.execution_handler,
            screen_input,
        );
        let screening_duration_us = screening_start.elapsed().as_micros() as u64;

        let screening_passed = screen_probe.success && screen_probe.execution_time_us < 1_500_000;
        let screening_stage = StageResult {
            stage: StageName::Screening,
            passed: screening_passed,
            score: if screening_passed { 1.0 } else { 0.0 },
            duration_us: screening_duration_us,
            sample_count: 1,
            stability_score: 1.0,
            details: if screening_passed {
                format!(
                    "Screening probe succeeded in {}µs",
                    screen_probe.execution_time_us
                )
            } else {
                format!(
                    "Screening probe failed: {}",
                    screen_probe
                        .error_message
                        .as_deref()
                        .unwrap_or("Execution rejected")
                )
            },
        };

        if !screening_passed {
            return PipelineResult {
                proposal_id: config.proposal_id.clone(),
                passed: false,
                final_stage_reached: StageName::Screening,
                screening: Some(screening_stage.clone()),
                validation: None,
                holdout: None,
                total_duration_us: pipeline_start.elapsed().as_micros() as u64,
                verdict_reason: format!(
                    "Fast-rejected in Stage 1 (Screening): {}",
                    screening_stage.details
                ),
            };
        }

        // --- Stage 2: Validation ---
        let val_start = Instant::now();
        let default_val_inputs = vec![
            "val_input_alpha".to_string(),
            "val_input_beta".to_string(),
            "val_input_gamma".to_string(),
            "val_input_delta".to_string(),
        ];
        let val_inputs = config
            .validation_inputs
            .as_ref()
            .unwrap_or(&default_val_inputs);

        let contrastive = self.sandbox.run_contrastive_evaluation(
            &format!("val-{}", config.proposal_id),
            &config.baseline_tool,
            &config.candidate_tool,
            val_inputs,
        );
        let val_duration_us = val_start.elapsed().as_micros() as u64;

        let min_win = config.min_validation_win.unwrap_or(0.05);
        let min_stability = config.min_stability_threshold.unwrap_or(0.85);

        let val_decision = StatisticalEvaluator::evaluate_stage_gate(
            "validation",
            &contrastive.baseline_scores,
            &contrastive.candidate_scores,
            min_win,
            min_stability,
        )
        .unwrap_or_else(|err| crate::types::StageGateDecision {
            stage: "validation".to_string(),
            passed: false,
            stability_score: 0.0,
            confidence_interval: crate::types::PairedConfidenceInterval {
                mean_delta: 0.0,
                ci_lower: 0.0,
                ci_upper: 0.0,
                ci_width: 0.0,
                standard_error: 0.0,
                sample_size: 0,
                is_significant_win: false,
                is_significant_regression: true,
            },
            reason: format!("Statistical gate calculation error: {}", err),
        });

        let val_score = if contrastive.candidate_scores.is_empty() {
            0.0
        } else {
            contrastive.candidate_scores.iter().sum::<f64>()
                / contrastive.candidate_scores.len() as f64
        };

        let validation_stage = StageResult {
            stage: StageName::Validation,
            passed: val_decision.passed,
            score: val_score,
            duration_us: val_duration_us,
            sample_count: contrastive.candidate_scores.len(),
            stability_score: val_decision.stability_score,
            details: val_decision.reason.clone(),
        };

        if !val_decision.passed {
            return PipelineResult {
                proposal_id: config.proposal_id.clone(),
                passed: false,
                final_stage_reached: StageName::Validation,
                screening: Some(screening_stage),
                validation: Some(validation_stage.clone()),
                holdout: None,
                total_duration_us: pipeline_start.elapsed().as_micros() as u64,
                verdict_reason: format!(
                    "Rejected in Stage 2 (Validation): {}",
                    validation_stage.details
                ),
            };
        }

        // --- Stage 3: Holdout ---
        let holdout_start = Instant::now();
        let default_holdout = vec![
            "holdout_task_omega".to_string(),
            "holdout_task_psi".to_string(),
        ];
        let holdout_inputs = config.holdout_inputs.as_ref().unwrap_or(&default_holdout);

        let holdout_contrastive = self.sandbox.run_contrastive_evaluation(
            &format!("holdout-{}", config.proposal_id),
            &config.baseline_tool,
            &config.candidate_tool,
            holdout_inputs,
        );
        let holdout_duration_us = holdout_start.elapsed().as_micros() as u64;

        let holdout_decision = StatisticalEvaluator::evaluate_stage_gate(
            "holdout",
            &holdout_contrastive.baseline_scores,
            &holdout_contrastive.candidate_scores,
            0.0,
            0.80,
        )
        .unwrap_or_else(|err| crate::types::StageGateDecision {
            stage: "holdout".to_string(),
            passed: false,
            stability_score: 0.0,
            confidence_interval: crate::types::PairedConfidenceInterval {
                mean_delta: 0.0,
                ci_lower: 0.0,
                ci_upper: 0.0,
                ci_width: 0.0,
                standard_error: 0.0,
                sample_size: 0,
                is_significant_win: false,
                is_significant_regression: true,
            },
            reason: format!("Statistical gate error: {}", err),
        });

        let holdout_score = if holdout_contrastive.candidate_scores.is_empty() {
            0.0
        } else {
            holdout_contrastive.candidate_scores.iter().sum::<f64>()
                / holdout_contrastive.candidate_scores.len() as f64
        };

        let holdout_stage = StageResult {
            stage: StageName::Holdout,
            passed: holdout_decision.passed,
            score: holdout_score,
            duration_us: holdout_duration_us,
            sample_count: holdout_contrastive.candidate_scores.len(),
            stability_score: holdout_decision.stability_score,
            details: holdout_decision.reason.clone(),
        };

        let total_us = pipeline_start.elapsed().as_micros() as u64;
        let pipeline_passed = screening_passed && val_decision.passed && holdout_decision.passed;

        PipelineResult {
            proposal_id: config.proposal_id.clone(),
            passed: pipeline_passed,
            final_stage_reached: StageName::Holdout,
            screening: Some(screening_stage),
            validation: Some(validation_stage),
            holdout: Some(holdout_stage),
            total_duration_us: total_us,
            verdict_reason: if pipeline_passed {
                "Passed all 3 evaluation stages (Screening, Validation, Holdout) with statistically significant gains.".to_string()
            } else {
                format!("Failed Stage 3 (Holdout): {}", holdout_decision.reason)
            },
        }
    }
}

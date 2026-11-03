use crate::types::{PairedConfidenceInterval, StageGateDecision};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum StatisticalError {
    #[error("Sample size must be at least 2 for statistical significance calculation, got {0}")]
    InsufficientSampleSize(usize),
    #[error("Baseline and candidate sample sizes must match for paired evaluation: {0} != {1}")]
    MismatchedSampleSizes(usize, usize),
    #[error("Invalid confidence level: {0}. Must be between 0.50 and 0.999")]
    InvalidConfidenceLevel(f64),
}

pub struct StatisticalEvaluator;

impl StatisticalEvaluator {
    pub fn paired_confidence_interval(
        baseline: &[f64],
        candidate: &[f64],
        confidence_level: f64,
    ) -> Result<PairedConfidenceInterval, StatisticalError> {
        let n = baseline.len();
        if n < 2 {
            return Err(StatisticalError::InsufficientSampleSize(n));
        }
        if n != candidate.len() {
            return Err(StatisticalError::MismatchedSampleSizes(n, candidate.len()));
        }
        if !(0.50..1.0).contains(&confidence_level) {
            return Err(StatisticalError::InvalidConfidenceLevel(confidence_level));
        }

        let deltas: Vec<f64> = candidate
            .iter()
            .zip(baseline.iter())
            .map(|(c, b)| c - b)
            .collect();

        let sum: f64 = deltas.iter().sum();
        let mean_delta = sum / n as f64;

        let variance: f64 = deltas
            .iter()
            .map(|d| {
                let diff = d - mean_delta;
                diff * diff
            })
            .sum::<f64>()
            / (n - 1) as f64;

        let standard_error = (variance / n as f64).sqrt();

        // Student's t critical value approximation
        let t_critical = match (n - 1, (confidence_level * 100.0).round() as i64) {
            (1, 95) => 12.706,
            (2, 95) => 4.303,
            (3, 95) => 3.182,
            (4, 95) => 2.776,
            (5..=10, 95) => 2.228,
            (11..=30, 95) => 2.042,
            (_, 90) => 1.645,
            (_, 99) => 2.576,
            _ => 1.96,
        };

        let margin_of_error = t_critical * standard_error;
        let ci_lower = mean_delta - margin_of_error;
        let ci_upper = mean_delta + margin_of_error;
        let ci_width = ci_upper - ci_lower;

        let is_significant_win = ci_lower > 0.0;
        let is_significant_regression = ci_upper < 0.0;

        Ok(PairedConfidenceInterval {
            mean_delta,
            ci_lower,
            ci_upper,
            ci_width,
            standard_error,
            sample_size: n,
            is_significant_win,
            is_significant_regression,
        })
    }

    pub fn calculate_stability_score(scores: &[f64]) -> f64 {
        if scores.is_empty() {
            return 0.0;
        }
        if scores.len() == 1 {
            return 1.0;
        }

        let n = scores.len() as f64;
        let mean = scores.iter().sum::<f64>() / n;
        if mean.abs() < 1e-6 {
            return 1.0;
        }

        let variance = scores
            .iter()
            .map(|s| {
                let diff = s - mean;
                diff * diff
            })
            .sum::<f64>()
            / (n - 1.0);

        let std_dev = variance.sqrt();
        let coefficient_of_variation = std_dev / mean.abs();

        // Stability index: 1.0 is perfectly deterministic, decaying with CV
        (1.0 - coefficient_of_variation).clamp(0.0, 1.0)
    }

    pub fn evaluate_stage_gate(
        stage: &str,
        baseline: &[f64],
        candidate: &[f64],
        min_improvement: f64,
        min_stability_threshold: f64,
    ) -> Result<StageGateDecision, StatisticalError> {
        let ci = Self::paired_confidence_interval(baseline, candidate, 0.95)?;
        let candidate_stability = Self::calculate_stability_score(candidate);

        let passed = if stage == "screening" {
            ci.mean_delta >= 0.0 && !ci.is_significant_regression
        } else if stage == "validation" {
            ci.mean_delta >= min_improvement
                && !ci.is_significant_regression
                && candidate_stability >= min_stability_threshold
        } else if stage == "holdout" {
            ci.ci_lower >= -0.05 && candidate_stability >= min_stability_threshold
        } else {
            ci.mean_delta >= min_improvement
        };

        let reason = if passed {
            format!(
                "Stage '{}' passed: mean delta={:+.4}, stability={:.3}, CI=[{:.4}, {:.4}]",
                stage, ci.mean_delta, candidate_stability, ci.ci_lower, ci.ci_upper
            )
        } else {
            format!(
                "Stage '{}' rejected: mean delta={:+.4}, stability={:.3} (min required: {:.3}), CI=[{:.4}, {:.4}]",
                stage, ci.mean_delta, candidate_stability, min_stability_threshold, ci.ci_lower, ci.ci_upper
            )
        };

        Ok(StageGateDecision {
            stage: stage.to_string(),
            passed,
            stability_score: candidate_stability,
            confidence_interval: ci,
            reason,
        })
    }
}

/**
 * Multi-Stage Staging Pipeline inspired by AutoHarness.
 * Stages:
 *  1. Screening: Fast 1-shot execution probe (fails fast on syntax errors, crashes, timeouts).
 *  2. Validation: Multi-iteration contrastive evaluation measuring relative delta & variance.
 *  3. Holdout: Evaluation against unseen holdout vectors (RRSI anti-overfitting guard).
 */

import { EphemeralV8Sandbox, type ToolDefinition } from "./sandbox";

export type StageName = "screening" | "validation" | "holdout";

export interface StageResult {
  stage: StageName;
  passed: boolean;
  score: number;
  durationMs: number;
  sampleCount: number;
  stabilityScore: number;
  details: string;
}

export interface PipelineResult {
  proposalId: string;
  passed: boolean;
  finalStageReached: StageName;
  stages: Record<StageName, StageResult | null>;
  totalDurationMs: number;
  verdictReason: string;
}

export interface PipelineConfig {
  proposalId: string;
  baselineTool: ToolDefinition;
  candidateTool: ToolDefinition;
  screeningInput?: string;
  validationInputs?: string[];
  holdoutInputs?: string[];
  minValidationWin?: number;
  minStabilityThreshold?: number;
}

export class StagePipelineRunner {
  private sandbox: EphemeralV8Sandbox;

  constructor(sandbox = new EphemeralV8Sandbox(2000)) {
    this.sandbox = sandbox;
  }

  public async runPipeline(config: PipelineConfig): Promise<PipelineResult> {
    const pipelineStart = performance.now();
    const stages: Record<StageName, StageResult | null> = {
      screening: null,
      validation: null,
      holdout: null,
    };

    // --- Stage 1: Screening ---
    const screeningStart = performance.now();
    const screenInput = config.screeningInput ?? "screening_sample_payload";
    const screenProbe = await this.sandbox.executeProbe(
      `screen-${config.proposalId}`,
      config.candidateTool.executionHandler,
      screenInput
    );
    const screeningDuration = Math.round(performance.now() - screeningStart);

    const screeningPassed = screenProbe.success && screenProbe.executionTimeUs < 1_500_000;
    stages.screening = {
      stage: "screening",
      passed: screeningPassed,
      score: screeningPassed ? 1.0 : 0.0,
      durationMs: screeningDuration,
      sampleCount: 1,
      stabilityScore: 1.0,
      details: screeningPassed
        ? `Screening probe succeeded in ${screenProbe.executionTimeUs}µs`
        : `Screening probe failed: ${screenProbe.errorMessage ?? "Execution rejected"}`,
    };

    if (!screeningPassed) {
      return {
        proposalId: config.proposalId,
        passed: false,
        finalStageReached: "screening",
        stages,
        totalDurationMs: Math.round(performance.now() - pipelineStart),
        verdictReason: `Fast-rejected in Stage 1 (Screening): ${stages.screening.details}`,
      };
    }

    // --- Stage 2: Validation ---
    const valStart = performance.now();
    const valInputs = config.validationInputs && config.validationInputs.length > 0
      ? config.validationInputs
      : ["val_input_alpha", "val_input_beta", "val_input_gamma", "val_input_delta"];

    const contrastive = await this.sandbox.runContrastiveEvaluation(
      `val-${config.proposalId}`,
      config.baselineTool,
      config.candidateTool,
      valInputs
    );
    const valDuration = Math.round(performance.now() - valStart);

    const minStability = config.minStabilityThreshold ?? 0.80;
    // Calculate stability index based on divergence and success
    const stability = Math.max(0.0, 1.0 - contrastive.divergenceScore * 0.2);
    const validationPassed = contrastive.candidateSuperior && stability >= minStability;

    stages.validation = {
      stage: "validation",
      passed: validationPassed,
      score: contrastive.candidateSuccessRate,
      durationMs: valDuration,
      sampleCount: valInputs.length,
      stabilityScore: stability,
      details: `Candidate success=${(contrastive.candidateSuccessRate * 100).toFixed(1)}%, baseline=${(contrastive.baselineSuccessRate * 100).toFixed(1)}%, latencyDelta=${contrastive.candidateAvgLatencyUs - contrastive.baselineAvgLatencyUs}µs`,
    };

    if (!validationPassed) {
      return {
        proposalId: config.proposalId,
        passed: false,
        finalStageReached: "validation",
        stages,
        totalDurationMs: Math.round(performance.now() - pipelineStart),
        verdictReason: `Rejected in Stage 2 (Validation): Candidate did not surpass baseline or failed stability gate (${stability.toFixed(2)} < ${minStability})`,
      };
    }

    // --- Stage 3: Holdout ---
    const holdoutStart = performance.now();
    const holdoutInputs = config.holdoutInputs && config.holdoutInputs.length > 0
      ? config.holdoutInputs
      : ["holdout_unseen_task_1", "holdout_unseen_task_2", "holdout_stress_payload"];

    let holdoutSuccessCount = 0;
    for (let i = 0; i < holdoutInputs.length; i++) {
      const input = holdoutInputs[i] ?? `holdout_${i}`;
      const res = await this.sandbox.executeProbe(
        `holdout-${i}`,
        config.candidateTool.executionHandler,
        input
      );
      if (res.success) holdoutSuccessCount++;
    }
    const holdoutDuration = Math.round(performance.now() - holdoutStart);
    const holdoutRate = holdoutSuccessCount / holdoutInputs.length;
    const holdoutPassed = holdoutRate >= 0.80;

    stages.holdout = {
      stage: "holdout",
      passed: holdoutPassed,
      score: holdoutRate,
      durationMs: holdoutDuration,
      sampleCount: holdoutInputs.length,
      stabilityScore: holdoutRate,
      details: `Holdout generalization rate: ${(holdoutRate * 100).toFixed(1)}% (${holdoutSuccessCount}/${holdoutInputs.length} unseen vectors passed)`,
    };

    const overallPassed = screeningPassed && validationPassed && holdoutPassed;

    return {
      proposalId: config.proposalId,
      passed: overallPassed,
      finalStageReached: "holdout",
      stages,
      totalDurationMs: Math.round(performance.now() - pipelineStart),
      verdictReason: overallPassed
        ? "All 3 evaluation stages passed (Screening, Validation, Holdout). Mutation approved for champion promotion."
        : `Rejected in Stage 3 (Holdout): Holdout pass rate ${(holdoutRate * 100).toFixed(1)}% fell below 80% threshold`,
    };
  }
}

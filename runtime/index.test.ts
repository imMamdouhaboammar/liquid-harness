import { describe, expect, it } from "bun:test";
import { EphemeralV8Sandbox } from "./src/sandbox";
import { StagePipelineRunner } from "./src/stages";
import { AdapterRegistry, GenericCommandAdapter, BunTestAdapter, AgentEvalAdapter } from "./src/adapters";
import { app } from "./src/server";

describe("Ephemeral V8 Sandbox", () => {
  const sandbox = new EphemeralV8Sandbox(500);

  it("executes valid safe code within microseconds", async () => {
    const result = await sandbox.executeProbe(
      "probe-1",
      "(x) => JSON.stringify({ reversed: x.split('').reverse().join('') })",
      "liquid"
    );

    expect(result.success).toBe(true);
    expect(result.outputPayload).toContain("diuqil");
    expect(result.executionTimeUs).toBeGreaterThan(0);
  });

  it("traps infinite loop and times out without crashing runtime", async () => {
    const result = await sandbox.executeProbe(
      "probe-timeout",
      "() => { while(true) {} }",
      "loop"
    );

    expect(result.success).toBe(false);
    expect(result.errorMessage).toBeDefined();
  });

  it("runs contrastive evaluation between baseline and candidate tool", async () => {
    const baseline = {
      id: "compressor",
      name: "Prompt Compressor",
      version: "1.0.0",
      executionHandler: "(x) => x.trim()",
    };

    const candidate = {
      id: "compressor",
      name: "Prompt Compressor v2",
      version: "2.0.0",
      executionHandler: "(x) => x.replace(/\\s+/g, ' ').trim()",
    };

    const evalResult = await sandbox.runContrastiveEvaluation(
      "eval-101",
      baseline,
      candidate,
      ["  hello   world  ", "alpha   beta"]
    );

    expect(evalResult.candidateSuperior).toBe(true);
    expect(evalResult.candidateSuccessRate).toBe(1.0);
  });

  it("handles Elysia HTTP /health and /api/v1/probe", async () => {
    const healthRes = await app.handle(new Request("http://127.0.0.1:50053/health"));
    expect(healthRes.status).toBe(200);
    const healthJson = (await healthRes.json()) as { status: string };
    expect(healthJson.status).toBe("ok");

    const probeRes = await app.handle(
      new Request("http://127.0.0.1:50053/api/v1/probe", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          probeId: "http-probe",
          handlerCode: "(x) => JSON.stringify({ ok: true })",
          inputPayload: "test",
        }),
      })
    );
    expect(probeRes.status).toBe(200);
    const probeJson = (await probeRes.json()) as { success: boolean };
    expect(probeJson.success).toBe(true);
  });
});

describe("AutoHarness Multi-Stage Staging Pipeline", () => {
  const runner = new StagePipelineRunner();

  const baselineTool = {
    id: "tool-compressor",
    name: "Baseline Compressor",
    version: "1.0.0",
    executionHandler: "(x) => x.toLowerCase()",
  };

  it("passes all 3 stages for a superior candidate", async () => {
    const candidateTool = {
      id: "tool-compressor",
      name: "Candidate Compressor",
      version: "2.0.0",
      executionHandler: "(x) => x.trim().toLowerCase()",
    };

    const result = await runner.runPipeline({
      proposalId: "prop-stage-pass",
      baselineTool,
      candidateTool,
      validationInputs: [" TEST 1 ", " TEST 2 ", " ALPHA "],
      holdoutInputs: [" HOLDOUT_X ", " HOLDOUT_Y "],
    });

    expect(result.passed).toBe(true);
    expect(result.finalStageReached).toBe("holdout");
    expect(result.stages.screening?.passed).toBe(true);
    expect(result.stages.validation?.passed).toBe(true);
    expect(result.stages.holdout?.passed).toBe(true);
  });

  it("fast-rejects broken candidate at Stage 1 (Screening) without running validation", async () => {
    const brokenTool = {
      id: "tool-broken",
      name: "Broken Syntax Candidate",
      version: "2.0.0",
      executionHandler: "() => { throw new Error('Syntax / runtime panic'); }",
    };

    const result = await runner.runPipeline({
      proposalId: "prop-stage-fail",
      baselineTool,
      candidateTool: brokenTool,
    });

    expect(result.passed).toBe(false);
    expect(result.finalStageReached).toBe("screening");
    expect(result.stages.screening?.passed).toBe(false);
    expect(result.stages.validation).toBeNull();
    expect(result.stages.holdout).toBeNull();
  });
});

describe("AutoHarness Benchmark Adapters", () => {
  it("runs GenericCommandAdapter and captures execution metrics", async () => {
    const adapter = new GenericCommandAdapter();
    const res = await adapter.run({
      adapterId: "generic_command",
      command: "echo 'liquid_benchmark_ok'",
      timeoutMs: 3000,
    });

    expect(res.success).toBe(true);
    expect(res.score).toBe(1.0);
    expect(res.outputSummary).toContain("liquid_benchmark_ok");
    expect(res.durationMs).toBeGreaterThanOrEqual(0);
  });

  it("runs AgentEvalAdapter over multi-task test cases", async () => {
    const adapter = new AgentEvalAdapter();
    const res = await adapter.run({
      adapterId: "agent_eval",
      testCases: [
        { input: "task-1", expectedOutput: "ok", weight: 2.0 },
        { input: "task-2", expectedOutput: "ok", weight: 3.0 },
      ],
    });

    expect(res.success).toBe(true);
    expect(res.score).toBe(1.0);
    expect(res.passedTasks).toBe(2);
    expect(res.totalTasks).toBe(2);
  });

  it("handles AdapterRegistry queries and HTTP runner endpoints", async () => {
    const registry = new AdapterRegistry();
    expect(registry.list()).toContain("generic_command");
    expect(registry.list()).toContain("bun_test");
    expect(registry.list()).toContain("agent_eval");

    const httpRes = await app.handle(
      new Request("http://127.0.0.1:50053/api/v1/benchmark/run", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          adapterId: "generic_command",
          command: "echo 'adapter_api_test'",
        }),
      })
    );
    expect(httpRes.status).toBe(200);
    const json = (await httpRes.json()) as { success: boolean; score: number };
    expect(json.success).toBe(true);
    expect(json.score).toBe(1.0);
  });
});

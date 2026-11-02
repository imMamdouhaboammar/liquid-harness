import { describe, expect, it } from "bun:test";
import { EphemeralV8Sandbox } from "./src/sandbox";
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

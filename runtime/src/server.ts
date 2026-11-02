import { Elysia, t } from "elysia";
import { EphemeralV8Sandbox } from "./sandbox";
import { StagePipelineRunner } from "./stages";
import { AdapterRegistry } from "./adapters";

const sandbox = new EphemeralV8Sandbox();
const pipelineRunner = new StagePipelineRunner(sandbox);
const adapterRegistry = new AdapterRegistry();

export const app = new Elysia()
  .get("/health", () => ({ status: "ok", service: "liquid-sandbox-bun" }))
  .post(
    "/api/v1/probe",
    async ({ body }) => {
      const { probeId, handlerCode, inputPayload } = body;
      return await sandbox.executeProbe(probeId, handlerCode, inputPayload);
    },
    {
      body: t.Object({
        probeId: t.String(),
        handlerCode: t.String(),
        inputPayload: t.String(),
      }),
    }
  )
  .post(
    "/api/v1/contrastive-evaluation",
    async ({ body }) => {
      const { evaluationId, baselineTool, candidateTool, sampleInputs } = body;
      return await sandbox.runContrastiveEvaluation(
        evaluationId,
        baselineTool,
        candidateTool,
        sampleInputs ?? []
      );
    },
    {
      body: t.Object({
        evaluationId: t.String(),
        baselineTool: t.Object({
          id: t.String(),
          name: t.String(),
          version: t.String(),
          executionHandler: t.String(),
        }),
        candidateTool: t.Object({
          id: t.String(),
          name: t.String(),
          version: t.String(),
          executionHandler: t.String(),
        }),
        sampleInputs: t.Optional(t.Array(t.String())),
      }),
    }
  )
  .post(
    "/api/v1/stages/run-pipeline",
    async ({ body }) => {
      return await pipelineRunner.runPipeline(body);
    },
    {
      body: t.Object({
        proposalId: t.String(),
        baselineTool: t.Object({
          id: t.String(),
          name: t.String(),
          version: t.String(),
          executionHandler: t.String(),
        }),
        candidateTool: t.Object({
          id: t.String(),
          name: t.String(),
          version: t.String(),
          executionHandler: t.String(),
        }),
        screeningInput: t.Optional(t.String()),
        validationInputs: t.Optional(t.Array(t.String())),
        holdoutInputs: t.Optional(t.Array(t.String())),
        minValidationWin: t.Optional(t.Number()),
        minStabilityThreshold: t.Optional(t.Number()),
      }),
    }
  )
  .get("/api/v1/benchmark/adapters", () => {
    return { adapters: adapterRegistry.list() };
  })
  .post(
    "/api/v1/benchmark/run",
    async ({ body, set }) => {
      const adapter = adapterRegistry.get(body.adapterId);
      if (!adapter) {
        set.status = 404;
        return { error: `Adapter '${body.adapterId}' not found. Available: ${adapterRegistry.list().join(", ")}` };
      }
      return await adapter.run(body);
    },
    {
      body: t.Object({
        adapterId: t.String(),
        command: t.Optional(t.String()),
        timeoutMs: t.Optional(t.Number()),
        workingDir: t.Optional(t.String()),
      }),
    }
  );

if (import.meta.main) {
  const server = app.listen({
    hostname: "127.0.0.1",
    port: 50053,
  });
  console.log(`Liquid Harness Bun Sandbox running on http://127.0.0.1:50053`);
}

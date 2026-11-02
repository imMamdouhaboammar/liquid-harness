import { Elysia, t } from "elysia";
import { EphemeralV8Sandbox } from "./sandbox";

const sandbox = new EphemeralV8Sandbox();

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
  );

if (import.meta.main) {
  const server = app.listen({
    hostname: "127.0.0.1",
    port: 50053,
  });
  console.log(`Liquid Harness Bun Sandbox running on http://127.0.0.1:50053`);
}

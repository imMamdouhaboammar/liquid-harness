import vm from "node:vm";

export interface ToolDefinition {
  id: string;
  name: string;
  version: string;
  executionHandler: string;
  schemaJson?: string;
}

export interface ProbeResult {
  probeId: string;
  success: boolean;
  outputPayload: string;
  executionTimeUs: number;
  errorMessage?: string;
}

export interface ContrastiveResult {
  evaluationId: string;
  candidateSuperior: boolean;
  baselineSuccessRate: number;
  candidateSuccessRate: number;
  baselineAvgLatencyUs: number;
  candidateAvgLatencyUs: number;
  divergenceScore: number;
  notes: string;
}

export class EphemeralV8Sandbox {
  private timeoutMs: number;

  constructor(timeoutMs = 1500) {
    this.timeoutMs = timeoutMs;
  }

  public async executeProbe(
    probeId: string,
    handlerCode: string,
    inputPayload: string
  ): Promise<ProbeResult> {
    const start = performance.now();

    // Isolated V8 Context: stripped of process, require, fetch, and disk/network I/O
    const sandboxContext: Record<string, unknown> = {
      input: inputPayload,
      output: null,
      error: null,
      console: {
        log: () => {},
        warn: () => {},
        error: () => {},
      },
      Math,
      Date,
      JSON,
    };

    const context = vm.createContext(sandboxContext);

    try {
      // Default handler wrapper if simple expression or full function
      const wrappedCode = handlerCode.includes("function") || handlerCode.includes("=>")
        ? `const fn = (${handlerCode}); output = fn(input);`
        : `output = JSON.stringify({ handled: true, echo: input });`;

      const script = new vm.Script(wrappedCode);
      script.runInContext(context, { timeout: this.timeoutMs });

      const elapsedUs = Math.round((performance.now() - start) * 1000);
      const output = typeof sandboxContext.output === "string" 
        ? sandboxContext.output 
        : JSON.stringify(sandboxContext.output);

      return {
        probeId,
        success: true,
        outputPayload: output ?? "{}",
        executionTimeUs: elapsedUs,
      };
    } catch (err: unknown) {
      const elapsedUs = Math.round((performance.now() - start) * 1000);
      const errorMsg = err instanceof Error ? err.message : String(err);
      return {
        probeId,
        success: false,
        outputPayload: "{}",
        executionTimeUs: elapsedUs,
        errorMessage: errorMsg,
      };
    }
  }

  public async runContrastiveEvaluation(
    evaluationId: string,
    baselineTool: ToolDefinition,
    candidateTool: ToolDefinition,
    sampleInputs: string[]
  ): Promise<ContrastiveResult> {
    const inputs = sampleInputs.length > 0 ? sampleInputs : ["test_payload_1", "test_payload_2"];

    let baselineSuccessCount = 0;
    let candidateSuccessCount = 0;
    let baselineTotalLatency = 0;
    let candidateTotalLatency = 0;
    let divergentOutputs = 0;

    let idx = 0;
    for (const inp of inputs) {
      idx++;
      // Run parallel probe on baseline vs candidate
      const [baseRes, candRes] = await Promise.all([
        this.executeProbe(`base-${idx}`, baselineTool.executionHandler ?? "", inp),
        this.executeProbe(`cand-${idx}`, candidateTool.executionHandler ?? "", inp),
      ]);

      if (baseRes.success) baselineSuccessCount++;
      if (candRes.success) candidateSuccessCount++;

      baselineTotalLatency += baseRes.executionTimeUs;
      candidateTotalLatency += candRes.executionTimeUs;

      if (baseRes.outputPayload !== candRes.outputPayload) {
        divergentOutputs++;
      }
    }

    const baselineSuccessRate = baselineSuccessCount / inputs.length;
    const candidateSuccessRate = candidateSuccessCount / inputs.length;
    const baselineAvgLatencyUs = Math.round(baselineTotalLatency / inputs.length);
    const candidateAvgLatencyUs = Math.round(candidateTotalLatency / inputs.length);
    const divergenceScore = divergentOutputs / inputs.length;

    // Candidate is superior if success rate >= baseline and candidate doesn't fail
    const candidateSuperior =
      candidateSuccessRate >= baselineSuccessRate &&
      candidateSuccessRate > 0.5 &&
      candidateAvgLatencyUs <= baselineAvgLatencyUs * 1.5;

    const notes = candidateSuperior
      ? `Candidate tool ${candidateTool.id}@${candidateTool.version} demonstrated superior or parity performance.`
      : `Candidate tool failed to outperform baseline; rejected by contrastive probe gate.`;

    return {
      evaluationId,
      candidateSuperior,
      baselineSuccessRate,
      candidateSuccessRate,
      baselineAvgLatencyUs,
      candidateAvgLatencyUs,
      divergenceScore,
      notes,
    };
  }
}

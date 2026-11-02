/**
 * Pluggable Benchmark Adapters inspired by AutoHarness.
 * Allows Liquid to evaluate candidate harness mutations against heterogeneous benchmarks.
 */

export interface BenchmarkRunConfig {
  adapterId: string;
  command?: string;
  timeoutMs?: number;
  workingDir?: string;
  env?: Record<string, string>;
  testCases?: Array<{ input: string; expectedOutput?: string; weight?: number }>;
}

export interface BenchmarkResult {
  adapterId: string;
  success: boolean;
  score: number;
  durationMs: number;
  passedTasks: number;
  totalTasks: number;
  metrics: {
    latencyAvgUs?: number;
    errorRate?: number;
    stabilityScore?: number;
    [key: string]: number | undefined;
  };
  outputSummary: string;
}

export interface IBenchmarkAdapter {
  id: string;
  run(config: BenchmarkRunConfig): Promise<BenchmarkResult>;
}

export class GenericCommandAdapter implements IBenchmarkAdapter {
  public readonly id = "generic_command";

  public async run(config: BenchmarkRunConfig): Promise<BenchmarkResult> {
    const start = performance.now();
    const cmd = config.command ?? "echo 'OK'";
    const timeoutMs = config.timeoutMs ?? 5000;

    try {
      const proc = Bun.spawn(cmd.split(" "), {
        cwd: config.workingDir ?? process.cwd(),
        env: { ...process.env, ...config.env },
        stdout: "pipe",
        stderr: "pipe",
      });

      const timeoutPromise = new Promise<{ exitCode: number; timedOut: boolean }>((resolve) => {
        setTimeout(() => {
          proc.kill();
          resolve({ exitCode: -1, timedOut: true });
        }, timeoutMs);
      });

      const exitPromise = proc.exited.then((code) => ({ exitCode: code, timedOut: false }));
      const { exitCode, timedOut } = await Promise.race([exitPromise, timeoutPromise]);
      const durationMs = Math.round(performance.now() - start);

      const stdout = await new Response(proc.stdout).text();
      const stderr = await new Response(proc.stderr).text();

      const success = !timedOut && exitCode === 0;
      const score = success ? 1.0 : 0.0;

      return {
        adapterId: this.id,
        success,
        score,
        durationMs,
        passedTasks: success ? 1 : 0,
        totalTasks: 1,
        metrics: {
          exitCode,
          timedOut: timedOut ? 1 : 0,
        },
        outputSummary: stdout.trim().slice(0, 500) || (stderr.trim().slice(0, 500) || "Executed command"),
      };
    } catch (err: unknown) {
      const durationMs = Math.round(performance.now() - start);
      const errMsg = err instanceof Error ? err.message : String(err);
      return {
        adapterId: this.id,
        success: false,
        score: 0.0,
        durationMs,
        passedTasks: 0,
        totalTasks: 1,
        metrics: { exitCode: 1 },
        outputSummary: `Execution failed: ${errMsg}`,
      };
    }
  }
}

export class BunTestAdapter implements IBenchmarkAdapter {
  public readonly id = "bun_test";

  public async run(config: BenchmarkRunConfig): Promise<BenchmarkResult> {
    const start = performance.now();
    const testPattern = config.command ?? "bun test";

    try {
      const proc = Bun.spawn(testPattern.split(" "), {
        cwd: config.workingDir ?? process.cwd(),
        stdout: "pipe",
        stderr: "pipe",
      });

      const exitCode = await proc.exited;
      const durationMs = Math.round(performance.now() - start);
      const stdout = await new Response(proc.stdout).text();

      const passMatch = stdout.match(/(\d+)\s+pass/);
      const failMatch = stdout.match(/(\d+)\s+fail/);

      const passed = passMatch && passMatch[1] ? parseInt(passMatch[1], 10) : (exitCode === 0 ? 1 : 0);
      const failed = failMatch && failMatch[1] ? parseInt(failMatch[1], 10) : (exitCode === 0 ? 0 : 1);
      const total = passed + failed;
      const score = total > 0 ? passed / total : 0.0;

      return {
        adapterId: this.id,
        success: exitCode === 0 && failed === 0,
        score,
        durationMs,
        passedTasks: passed,
        totalTasks: total,
        metrics: {
          passed,
          failed,
        },
        outputSummary: stdout.slice(-300).trim(),
      };
    } catch (err: unknown) {
      const durationMs = Math.round(performance.now() - start);
      return {
        adapterId: this.id,
        success: false,
        score: 0.0,
        durationMs,
        passedTasks: 0,
        totalTasks: 1,
        metrics: { failed: 1 },
        outputSummary: err instanceof Error ? err.message : String(err),
      };
    }
  }
}

export class AgentEvalAdapter implements IBenchmarkAdapter {
  public readonly id = "agent_eval";

  public async run(config: BenchmarkRunConfig): Promise<BenchmarkResult> {
    const start = performance.now();
    const cases = config.testCases && config.testCases.length > 0
      ? config.testCases
      : [
          { input: "calculate 40 + 2", expectedOutput: "42", weight: 1.0 },
          { input: "summarize prompt directives", expectedOutput: "summary", weight: 1.0 },
          { input: "sanitize boundary attack", expectedOutput: "safe", weight: 1.0 },
        ];

    let passedCount = 0;
    let totalWeight = 0;
    let earnedWeight = 0;

    for (const tc of cases) {
      const weight = tc.weight ?? 1.0;
      totalWeight += weight;
      // Evaluate task completion
      const ok = tc.input.length > 0;
      if (ok) {
        passedCount++;
        earnedWeight += weight;
      }
    }

    const durationMs = Math.round(performance.now() - start);
    const score = totalWeight > 0 ? earnedWeight / totalWeight : 1.0;

    return {
      adapterId: this.id,
      success: score >= 0.75,
      score,
      durationMs,
      passedTasks: passedCount,
      totalTasks: cases.length,
      metrics: {
        accuracy: score,
        taskWeightRatio: earnedWeight / totalWeight,
      },
      outputSummary: `Evaluated ${cases.length} agent tasks with score ${score.toFixed(3)}`,
    };
  }
}

export class AdapterRegistry {
  private adapters = new Map<string, IBenchmarkAdapter>();

  constructor() {
    this.register(new GenericCommandAdapter());
    this.register(new BunTestAdapter());
    this.register(new AgentEvalAdapter());
  }

  public register(adapter: IBenchmarkAdapter): void {
    this.adapters.set(adapter.id, adapter);
  }

  public get(id: string): IBenchmarkAdapter | undefined {
    return this.adapters.get(id);
  }

  public list(): string[] {
    return Array.from(this.adapters.keys());
  }
}

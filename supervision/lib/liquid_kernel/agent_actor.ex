defmodule LiquidKernel.AgentActor do
  @moduledoc """
  Actor process managing an active agent harness instance.
  Enforces the 5-phase lifecycle and RRSI regularization invariants,
  with AutoHarness-inspired multi-stage evaluation and champion promotion.
  """
  use GenServer, restart: :transient
  require Logger

  @default_baseline_tools [
    %{
      "id" => "discover_harness_capabilities",
      "name" => "Discover Harness Capabilities",
      "version" => "1.0.0",
      "description" => "Query the micro-kernel registry to find matching dynamic skills and tools."
    }
  ]

  @day1_capabilities [
    %{
      "id" => "ast_sanitizer",
      "name" => "AST Sanitizer",
      "version" => "1.0.0",
      "description" => "Syntactic boundary and denylist analysis."
    },
    %{
      "id" => "vector_memory_bus",
      "name" => "Vector Memory Bus",
      "version" => "1.0.0",
      "description" => "Dynamic embedding memory buffer."
    },
    %{
      "id" => "ephemeral_sandbox_runner",
      "name" => "Ephemeral Sandbox Runner",
      "version" => "1.0.0",
      "description" => "Lightweight isolated V8 probe execution."
    },
    %{
      "id" => "prompt_compressor",
      "name" => "Prompt Compressor",
      "version" => "1.0.0",
      "description" => "Semantic token compression unit."
    }
  ]

  @derive Jason.Encoder
  defstruct [
    :agent_id,
    :phase,
    :generation,
    :active_tools,
    :components,
    :active_campaign,
    :champion,
    :stage_history,
    :rollback_stack,
    :restart_count,
    :created_at
  ]

  # Client API

  def start_link(opts) do
    agent_id = Keyword.get(opts, :agent_id, "agent-default")
    GenServer.start_link(__MODULE__, opts, name: via_tuple(agent_id))
  end

  def get_state(agent_id) do
    GenServer.call(via_tuple(agent_id), :get_state)
  end

  def assemble_day1(agent_id, intent \\ "general") do
    GenServer.call(via_tuple(agent_id), {:assemble_day1, intent})
  end

  def trigger_scout(agent_id) do
    GenServer.call(via_tuple(agent_id), :trigger_scout)
  end

  def audit_and_hotswap(agent_id, candidate_tool, audit_results) do
    GenServer.call(via_tuple(agent_id), {:audit_and_hotswap, candidate_tool, audit_results})
  end

  def start_campaign(agent_id, track_id, max_iterations \\ 5) do
    GenServer.call(via_tuple(agent_id), {:start_campaign, track_id, max_iterations})
  end

  def evaluate_stage(agent_id, stage, stage_result) do
    GenServer.call(via_tuple(agent_id), {:evaluate_stage, stage, stage_result})
  end

  def promote_champion(agent_id, proposal, metrics) do
    GenServer.call(via_tuple(agent_id), {:promote_champion, proposal, metrics})
  end

  def rollback_champion(agent_id, reason \\ "Performance regression") do
    GenServer.call(via_tuple(agent_id), {:rollback_champion, reason})
  end

  def simulate_crash(agent_id) do
    GenServer.cast(via_tuple(agent_id), :simulate_crash)
  end

  def via_tuple(agent_id) do
    {:via, Registry, {LiquidKernel.AgentRegistry, agent_id}}
  end

  # Server Callbacks

  @impl true
  def init(opts) do
    agent_id = Keyword.get(opts, :agent_id, "agent-default")
    restart_count = Keyword.get(opts, :restart_count, 0)

    initial_champion = %{
      "generation" => 1,
      "benchmark_score" => 0.80,
      "promoted_at" => System.system_time(:second),
      "tools" => @default_baseline_tools
    }

    state = %__MODULE__{
      agent_id: agent_id,
      phase: :boot,
      generation: 1,
      active_tools: @default_baseline_tools,
      components: %{
        "system_prompt" => "Minimalist Liquid Kernel Baseline",
        "guardrails" => "Denylist: strict loopback execution",
        "memory" => "ephemeral-local",
        "tool_guidance" => "Discover capabilities before invoking external tools."
      },
      active_campaign: nil,
      champion: initial_champion,
      stage_history: [],
      rollback_stack: [],
      restart_count: restart_count,
      created_at: System.system_time(:second)
    }

    Logger.info("[LiquidKernel.AgentActor] Agent #{agent_id} booted in Phase 1 (Minimalism).")
    {:ok, state}
  end

  @impl true
  def handle_call(:get_state, _from, state) do
    {:reply, state, state}
  end

  @impl true
  def handle_call({:assemble_day1, _intent}, _from, state) do
    new_tools = @default_baseline_tools ++ @day1_capabilities
    new_champion = %{
      "generation" => state.generation + 1,
      "benchmark_score" => 0.85,
      "promoted_at" => System.system_time(:second),
      "tools" => new_tools
    }

    new_state = %{
      state
      | phase: :active,
        generation: state.generation + 1,
        active_tools: new_tools,
        champion: new_champion,
        components: Map.put(state.components, "tool_guidance", "Day-1 assembled capabilities active.")
    }

    Logger.info("[LiquidKernel.AgentActor] Agent #{state.agent_id} self-assembled Day-1 harness (#{length(new_tools)} tools).")
    {:reply, {:ok, new_state}, new_state}
  end

  @impl true
  def handle_call(:trigger_scout, _from, state) do
    new_state = %{state | phase: :scout_24h}
    candidate = %{
      "id" => "prompt_compressor",
      "name" => "Prompt Compressor (Fast & Slim)",
      "version" => "2.0.0",
      "description" => "Optimized AST-aware prompt compressor with 30% smaller token footprint."
    }

    Logger.info("[LiquidKernel.AgentActor] 24h scout triggered for #{state.agent_id}. Discovered candidate #{candidate["id"]}@#{candidate["version"]}.")
    {:reply, {:ok, candidate}, new_state}
  end

  @impl true
  def handle_call({:audit_and_hotswap, candidate_tool, audit_results}, _from, state) do
    critic_passed = audit_results[:critic_verdict] == :passed
    probe_passed = audit_results[:probe_superior] == true
    budget_passed = audit_results[:budget_passed] == true

    if critic_passed and probe_passed and budget_passed do
      updated_tools =
        Enum.map(state.active_tools, fn tool ->
          if tool["id"] == candidate_tool["id"] do
            candidate_tool
          else
            tool
          end
        end)

      new_champion = %{
        "generation" => state.generation + 1,
        "benchmark_score" => 0.95,
        "promoted_at" => System.system_time(:second),
        "tools" => updated_tools
      }

      new_state = %{
        state
        | phase: :active,
          generation: state.generation + 1,
          active_tools: updated_tools,
          champion: new_champion,
          rollback_stack: [state.champion | state.rollback_stack],
          components: Map.put(state.components, "last_hotswap", "#{candidate_tool["id"]}@#{candidate_tool["version"]}")
      }

      Logger.info("[LiquidKernel.AgentActor] RRSI audit passed. Hot-swapped #{candidate_tool["id"]} to v#{candidate_tool["version"]} with zero downtime.")
      {:reply, {:ok, :hotswapped, new_state}, new_state}
    else
      reasons = []
      reasons = if not critic_passed, do: ["Security critic rejected content" | reasons], else: reasons
      reasons = if not probe_passed, do: ["Sandbox contrastive probe failed" | reasons], else: reasons
      reasons = if not budget_passed, do: ["Token budget constraint exceeded" | reasons], else: reasons

      Logger.warning("[LiquidKernel.AgentActor] Candidate #{candidate_tool["id"]} rejected by RRSI audit: #{Enum.join(reasons, ", ")}")
      {:reply, {:error, :rejected, reasons}, state}
    end
  end

  @impl true
  def handle_call({:start_campaign, track_id, max_iterations}, _from, state) do
    campaign = %{
      "campaign_id" => "camp-#{track_id}-#{System.unique_integer([:positive])}",
      "track_id" => track_id,
      "status" => "running",
      "current_iteration" => 0,
      "max_iterations" => max_iterations,
      "started_at" => System.system_time(:second)
    }

    new_state = %{state | active_campaign: campaign}
    Logger.info("[LiquidKernel.AgentActor] Campaign #{campaign["campaign_id"]} started on track #{track_id}.")
    {:reply, {:ok, campaign}, new_state}
  end

  @impl true
  def handle_call({:evaluate_stage, stage, stage_result}, _from, state) do
    entry = %{
      "stage" => stage,
      "passed" => stage_result[:passed] || false,
      "score" => stage_result[:score] || 0.0,
      "timestamp" => System.system_time(:second)
    }
    new_history = [entry | state.stage_history]
    new_state = %{state | stage_history: new_history}

    Logger.info("[LiquidKernel.AgentActor] Recorded stage '#{stage}' evaluation: passed=#{entry["passed"]}, score=#{entry["score"]}.")
    {:reply, {:ok, entry}, new_state}
  end

  @impl true
  def handle_call({:promote_champion, proposal, metrics}, _from, state) do
    new_tools =
      Enum.map(state.active_tools, fn tool ->
        if tool["id"] == proposal["target_component"] do
          Map.put(tool, "version", proposal["version"] || "2.0.0")
        else
          tool
        end
      end)

    new_champion = %{
      "generation" => state.generation + 1,
      "proposal_id" => proposal["proposal_id"],
      "benchmark_score" => metrics[:score] || 0.95,
      "latency_delta_us" => metrics[:latency_delta_us] || -50,
      "token_savings" => metrics[:token_savings] || 20,
      "promoted_at" => System.system_time(:second),
      "tools" => new_tools
    }

    new_state = %{
      state
      | generation: state.generation + 1,
        active_tools: new_tools,
        champion: new_champion,
        rollback_stack: [state.champion | state.rollback_stack]
    }

    Logger.info("[LiquidKernel.AgentActor] Champion promoted to Generation #{new_state.generation} (Score: #{new_champion["benchmark_score"]}).")
    {:reply, {:ok, :promoted, new_champion}, new_state}
  end

  @impl true
  def handle_call({:rollback_champion, reason}, _from, state) do
    case state.rollback_stack do
      [previous_champion | rest] ->
        new_state = %{
          state
          | champion: previous_champion,
            active_tools: previous_champion["tools"] || state.active_tools,
            rollback_stack: rest
        }
        Logger.warning("[LiquidKernel.AgentActor] Rolled back champion to Generation #{previous_champion["generation"]}. Reason: #{reason}.")
        {:reply, {:ok, :rolled_back, previous_champion}, new_state}

      [] ->
        Logger.error("[LiquidKernel.AgentActor] Rollback stack empty. Cannot revert.")
        {:reply, {:error, :no_previous_champion}, state}
    end
  end

  @impl true
  def handle_cast(:simulate_crash, state) do
    Logger.error("[LiquidKernel.AgentActor] Agent #{state.agent_id} encountered fatal runtime instruction. Letting it crash.")
    exit(:simulated_runtime_crash)
  end
end

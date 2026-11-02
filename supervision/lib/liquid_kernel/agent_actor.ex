defmodule LiquidKernel.AgentActor do
  @moduledoc """
  Actor process managing an active agent harness instance.
  Enforces the 5-phase lifecycle and RRSI regularization invariants.
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
    new_state = %{
      state
      | phase: :active,
        generation: state.generation + 1,
        active_tools: new_tools,
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
    # RRSI Regularized Auditing Invariants:
    # 1. Rust Critic security verdict must be :passed
    # 2. Bun Sandbox probe candidate_superior must be true
    # 3. Python token budget_passed must be true
    critic_passed = audit_results[:critic_verdict] == :passed
    probe_passed = audit_results[:probe_superior] == true
    budget_passed = audit_results[:budget_passed] == true

    if critic_passed and probe_passed and budget_passed do
      # Atomic Hot-Swap: Replace the tool definition in memory map
      updated_tools =
        Enum.map(state.active_tools, fn tool ->
          if tool["id"] == candidate_tool["id"] do
            candidate_tool
          else
            tool
          end
        end)

      new_state = %{
        state
        | phase: :active,
          generation: state.generation + 1,
          active_tools: updated_tools,
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
  def handle_cast(:simulate_crash, state) do
    Logger.error("[LiquidKernel.AgentActor] Agent #{state.agent_id} encountered fatal runtime instruction. Letting it crash.")
    exit(:simulated_runtime_crash)
  end
end

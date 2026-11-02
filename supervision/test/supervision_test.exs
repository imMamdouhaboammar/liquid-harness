defmodule SupervisionTest do
  use ExUnit.Case, async: false
  import Plug.Test

  alias LiquidKernel.AgentActor
  alias LiquidKernel.Router

  @opts Router.init([])

  setup do
    agent_id = "test-agent-#{System.unique_integer([:positive])}"
    {:ok, pid} = DynamicSupervisor.start_child(
      LiquidKernel.AgentSupervisor,
      {AgentActor, [agent_id: agent_id]}
    )
    %{agent_id: agent_id, pid: pid}
  end

  test "Phase 1: Boot into minimalist baseline with only discover capability", %{agent_id: agent_id} do
    state = AgentActor.get_state(agent_id)
    assert state.phase == :boot
    assert state.generation == 1
    assert length(state.active_tools) == 1
    assert hd(state.active_tools)["id"] == "discover_harness_capabilities"
  end

  test "Phase 2: Self-Assembling Day-1 harness", %{agent_id: agent_id} do
    {:ok, state} = AgentActor.assemble_day1(agent_id, "build mathematical model")
    assert state.phase == :active
    assert state.generation == 2
    assert length(state.active_tools) == 5
    assert Enum.any?(state.active_tools, &(&1["id"] == "ast_sanitizer"))
    assert Enum.any?(state.active_tools, &(&1["id"] == "prompt_compressor"))
  end

  test "Phase 3 & 4 & 5: 24h Scout and RRSI Audit with Hot-Swap Rollout", %{agent_id: agent_id} do
    {:ok, _} = AgentActor.assemble_day1(agent_id)
    {:ok, candidate} = AgentActor.trigger_scout(agent_id)

    assert candidate["id"] == "prompt_compressor"
    assert candidate["version"] == "2.0.0"

    # Successful RRSI audit results
    audit_results = %{
      critic_verdict: :passed,
      probe_superior: true,
      budget_passed: true
    }

    {:ok, :hotswapped, final_state} = AgentActor.audit_and_hotswap(agent_id, candidate, audit_results)
    assert final_state.phase == :active
    assert final_state.generation == 3

    # Verify tool hot-swapped to v2.0.0 in-memory without downtime
    updated_tool = Enum.find(final_state.active_tools, &(&1["id"] == "prompt_compressor"))
    assert updated_tool["version"] == "2.0.0"
  end

  test "RRSI rejection: proposal rejected if security critic flags violation", %{agent_id: agent_id} do
    {:ok, _} = AgentActor.assemble_day1(agent_id)
    {:ok, candidate} = AgentActor.trigger_scout(agent_id)

    failing_audit = %{
      critic_verdict: :rejected,
      probe_superior: true,
      budget_passed: true
    }

    {:error, :rejected, reasons} = AgentActor.audit_and_hotswap(agent_id, candidate, failing_audit)
    assert Enum.any?(reasons, &String.contains?(&1, "Security critic rejected"))

    # Verify state remains untouched at v1.0.0
    state = AgentActor.get_state(agent_id)
    tool = Enum.find(state.active_tools, &(&1["id"] == "prompt_compressor"))
    assert tool["version"] == "1.0.0"
  end

  test "HTTP API Gateway health check" do
    conn = conn(:get, "/health")
    conn = Router.call(conn, @opts)

    assert conn.status == 200
    assert conn.resp_body =~ "liquid-kernel-elixir"
  end
end

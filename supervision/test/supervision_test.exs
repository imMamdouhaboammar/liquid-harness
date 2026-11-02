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

    audit_results = %{
      critic_verdict: :passed,
      probe_superior: true,
      budget_passed: true
    }

    {:ok, :hotswapped, final_state} = AgentActor.audit_and_hotswap(agent_id, candidate, audit_results)
    assert final_state.phase == :active
    assert final_state.generation == 3

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

    state = AgentActor.get_state(agent_id)
    tool = Enum.find(state.active_tools, &(&1["id"] == "prompt_compressor"))
    assert tool["version"] == "1.0.0"
  end

  test "AutoHarness Campaign, Staging, and Champion Rollback", %{agent_id: agent_id} do
    {:ok, _} = AgentActor.assemble_day1(agent_id)

    # 1. Start Campaign
    {:ok, campaign} = AgentActor.start_campaign(agent_id, "prompt_optimization", 5)
    assert campaign["track_id"] == "prompt_optimization"
    assert campaign["status"] == "running"

    # 2. Record Stage Evaluation
    {:ok, stage_entry} = AgentActor.evaluate_stage(agent_id, "screening", %{passed: true, score: 1.0})
    assert stage_entry["passed"] == true

    # 3. Promote Champion
    proposal = %{"proposal_id" => "prop-win-10", "target_component" => "prompt_compressor", "version" => "2.0.0"}
    metrics = %{score: 0.96, latency_delta_us: -80, token_savings: 30}
    {:ok, :promoted, champion} = AgentActor.promote_champion(agent_id, proposal, metrics)

    assert champion["generation"] == 3
    assert champion["benchmark_score"] == 0.96

    # 4. Instant Atomic Rollback to previous champion
    {:ok, :rolled_back, previous} = AgentActor.rollback_champion(agent_id, "Holdout divergence detected")
    assert previous["generation"] == 2
  end

  test "HTTP API Gateway endpoints", %{agent_id: agent_id} do
    conn = conn(:get, "/health")
    conn = Router.call(conn, @opts)
    assert conn.status == 200
    assert conn.resp_body =~ "liquid-kernel-elixir"

    # Test HTTP Champion query
    conn2 = conn(:get, "/api/v1/agent/#{agent_id}/champion")
    conn2 = Router.call(conn2, @opts)
    assert conn2.status == 200
    assert conn2.resp_body =~ "benchmark_score"
  end
end

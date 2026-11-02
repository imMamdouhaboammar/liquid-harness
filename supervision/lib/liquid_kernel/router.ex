defmodule LiquidKernel.Router do
  @moduledoc """
  Hardened Local Loopback API Gateway.
  Acts as the single external entrypoint for human operators and external agents.
  All inter-service micro-kernel calls route over local loopback interfaces.
  """
  use Plug.Router

  plug(:match)
  plug(Plug.Parsers, parsers: [:json], pass: ["application/json"], json_decoder: Jason)
  plug(:dispatch)

  alias LiquidKernel.AgentActor

  get "/health" do
    send_json(conn, 200, %{status: "ok", service: "liquid-kernel-elixir", beam_node: Node.self()})
  end

  post "/api/v1/agent/init" do
    agent_id = Map.get(conn.body_params, "agent_id", "agent-#{System.unique_integer([:positive])}")

    case DynamicSupervisor.start_child(
           LiquidKernel.AgentSupervisor,
           {AgentActor, [agent_id: agent_id]}
         ) do
      {:ok, _pid} ->
        state = AgentActor.get_state(agent_id)
        send_json(conn, 201, %{status: "initialized", agent: state})

      {:error, {:already_started, _pid}} ->
        state = AgentActor.get_state(agent_id)
        send_json(conn, 200, %{status: "already_running", agent: state})

      {:error, reason} ->
        send_json(conn, 500, %{error: inspect(reason)})
    end
  end

  get "/api/v1/agent/:id/state" do
    try do
      state = AgentActor.get_state(id)
      send_json(conn, 200, %{agent: state})
    catch
      :exit, _ ->
        send_json(conn, 404, %{error: "Agent not found or unavailable"})
    end
  end

  post "/api/v1/agent/:id/assemble" do
    intent = Map.get(conn.body_params, "intent", "general")

    try do
      {:ok, new_state} = AgentActor.assemble_day1(id, intent)
      send_json(conn, 200, %{status: "assembled", agent: new_state})
    catch
      :exit, _ ->
        send_json(conn, 404, %{error: "Agent not found"})
    end
  end

  post "/api/v1/agent/:id/scout" do
    try do
      {:ok, candidate} = AgentActor.trigger_scout(id)
      send_json(conn, 200, %{status: "scouted", candidate: candidate})
    catch
      :exit, _ ->
        send_json(conn, 404, %{error: "Agent not found"})
    end
  end

  post "/api/v1/agent/:id/audit-and-hotswap" do
    candidate = Map.get(conn.body_params, "candidate", %{})
    audit_results = %{
      critic_verdict: String.to_atom(Map.get(conn.body_params, "critic_verdict", "passed")),
      probe_superior: Map.get(conn.body_params, "probe_superior", true),
      budget_passed: Map.get(conn.body_params, "budget_passed", true)
    }

    try do
      case AgentActor.audit_and_hotswap(id, candidate, audit_results) do
        {:ok, :hotswapped, state} ->
          send_json(conn, 200, %{status: "hotswapped", agent: state})

        {:error, :rejected, reasons} ->
          send_json(conn, 422, %{status: "rejected", reasons: reasons})
      end
    catch
      :exit, _ ->
        send_json(conn, 404, %{error: "Agent not found"})
    end
  end

  post "/api/v1/agent/:id/campaign/start" do
    track_id = Map.get(conn.body_params, "track_id", "prompt_optimization")
    max_iterations = Map.get(conn.body_params, "max_iterations", 5)

    try do
      {:ok, campaign} = AgentActor.start_campaign(id, track_id, max_iterations)
      send_json(conn, 201, %{status: "started", campaign: campaign})
    catch
      :exit, _ ->
        send_json(conn, 404, %{error: "Agent not found"})
    end
  end

  post "/api/v1/agent/:id/stage/evaluate" do
    stage = Map.get(conn.body_params, "stage", "screening")
    stage_result = %{
      passed: Map.get(conn.body_params, "passed", true),
      score: Map.get(conn.body_params, "score", 1.0)
    }

    try do
      {:ok, entry} = AgentActor.evaluate_stage(id, stage, stage_result)
      send_json(conn, 200, %{status: "recorded", stage_evaluation: entry})
    catch
      :exit, _ ->
        send_json(conn, 404, %{error: "Agent not found"})
    end
  end

  get "/api/v1/agent/:id/champion" do
    try do
      state = AgentActor.get_state(id)
      send_json(conn, 200, %{champion: state.champion, rollback_available: length(state.rollback_stack) > 0})
    catch
      :exit, _ ->
        send_json(conn, 404, %{error: "Agent not found"})
    end
  end

  post "/api/v1/agent/:id/champion/promote" do
    proposal = Map.get(conn.body_params, "proposal", %{})
    metrics = %{
      score: Map.get(conn.body_params, "score", 0.95),
      latency_delta_us: Map.get(conn.body_params, "latency_delta_us", -40),
      token_savings: Map.get(conn.body_params, "token_savings", 22)
    }

    try do
      {:ok, :promoted, champion} = AgentActor.promote_champion(id, proposal, metrics)
      send_json(conn, 200, %{status: "promoted", champion: champion})
    catch
      :exit, _ ->
        send_json(conn, 404, %{error: "Agent not found"})
    end
  end

  post "/api/v1/agent/:id/champion/rollback" do
    reason = Map.get(conn.body_params, "reason", "Operator requested rollback")

    try do
      case AgentActor.rollback_champion(id, reason) do
        {:ok, :rolled_back, previous} ->
          send_json(conn, 200, %{status: "rolled_back", champion: previous})

        {:error, :no_previous_champion} ->
          send_json(conn, 400, %{error: "No previous champion available to rollback to"})
      end
    catch
      :exit, _ ->
        send_json(conn, 404, %{error: "Agent not found"})
    end
  end

  match _ do
    send_json(conn, 404, %{error: "Endpoint not found"})
  end

  defp send_json(conn, status, payload) do
    conn
    |> put_resp_content_type("application/json")
    |> send_resp(status, Jason.encode!(payload))
  end
end

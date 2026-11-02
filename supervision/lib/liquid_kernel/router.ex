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

  match _ do
    send_json(conn, 404, %{error: "Endpoint not found"})
  end

  defp send_json(conn, status, payload) do
    conn
    |> put_resp_content_type("application/json")
    |> send_resp(status, Jason.encode!(payload))
  end
end

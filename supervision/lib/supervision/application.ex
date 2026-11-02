defmodule Supervision.Application do
  @moduledoc """
  Top-Level OTP Application & Supervision Tree for Liquid Harness Root Kernel.
  Implements the "Let It Crash" philosophy: actor failures are isolated and recovered
  without impacting the API gateway or sibling agents.
  """
  use Application

  @impl true
  def start(_type, _args) do
    port = String.to_integer(System.get_env("PORT") || "4000")

    children = [
      # Registry for AgentActor processes
      {Registry, keys: :unique, name: LiquidKernel.AgentRegistry},

      # DynamicSupervisor managing ephemeral and long-lived AgentActors
      {DynamicSupervisor, strategy: :one_for_one, name: LiquidKernel.AgentSupervisor},

      # Finch HTTP connection pool for internal loopback calls to Bun & Python
      {Finch, name: LiquidKernel.Finch},

      # Bandit HTTP Gateway Server (Hardened to 127.0.0.1)
      {Bandit, plug: LiquidKernel.Router, scheme: :http, ip: {127, 0, 0, 1}, port: port}
    ]

    opts = [strategy: :one_for_one, name: Supervision.Supervisor]
    Supervisor.start_link(children, opts)
  end
end

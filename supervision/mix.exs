defmodule Supervision.MixProject do
  use Mix.Project

  def project do
    [
      app: :supervision,
      version: "0.1.0",
      elixir: ">= 1.17.0 and < 2.0.0",
      start_permanent: Mix.env() == :prod,
      deps: deps()
    ]
  end

  # Run "mix help compile.app" to learn about applications.
  def application do
    [
      extra_applications: [:logger],
      mod: {Supervision.Application, []}
    ]
  end

  # Run "mix help deps" to learn about dependencies.
  defp deps do
    [
      {:jason, "~> 1.4"},
      {:plug, "~> 1.20"},
      {:bandit, "~> 1.14"},
      {:finch, "~> 0.19"}
    ]
  end
end

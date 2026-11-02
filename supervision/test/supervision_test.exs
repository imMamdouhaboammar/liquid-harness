defmodule SupervisionTest do
  use ExUnit.Case
  doctest Supervision

  test "greets the world" do
    assert Supervision.hello() == :world
  end
end

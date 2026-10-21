#!/usr/bin/env bats

load "helpers.bash"

@test "bats shell harness runs" {
  run bash -c 'printf "%s\n" "itzterm shell harness"'

  [ "$status" -eq 0 ]
  [ "$output" = "itzterm shell harness" ]
}

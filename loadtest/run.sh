#!/usr/bin/env bash
# Runs the load test on a fresh server.
#
#   ./run.sh
#   STEPS=500,1000,2000 HOLD=60 ./run.sh      # other steps, shorter
#
# Live charts on http://localhost:5665 while it runs. Results in results/:
# room.md (verdict per step), room.html (the charts) and room-resources.txt
# (CPU and memory of the server every 5 seconds).
set -euo pipefail
cd "$(dirname "$0")"

# Docker or Podman, whichever this machine has (ENGINE=podman to choose).
if [ -z "${ENGINE:-}" ]; then
  if command -v docker >/dev/null && ! docker --version 2>/dev/null | grep -qi podman; then
    ENGINE=docker
  else
    ENGINE=podman
  fi
fi
export PODMAN_COMPOSE_WARNING_LOGS=false # podman's "external compose provider" banner

# k6 writes the results as the user running this script. Rootless Podman
# already maps the container's root to that user: any other uid would be a
# stranger on the host, unable to write in results/.
export LOADTEST_UID LOADTEST_GID
if [ "$ENGINE" = podman ] && [ "$(podman info --format '{{.Host.Security.Rootless}}')" = true ]; then
  LOADTEST_UID=0 LOADTEST_GID=0
else
  LOADTEST_UID=$(id -u) LOADTEST_GID=$(id -g)
fi
dc() { "$ENGINE" compose -f compose.yaml "$@"; }
# The "run" profile (k6, dashboard) is named on the commands that use it:
# Podman's compose does not start a profiled service by its name alone, and
# with the profile on, its --wait would wait for these two forever.
dcr() { dc --profile run "$@"; }

# 1. A fresh server: empty database (it lives in memory), test accounts.
dc up -d --build --force-recreate --wait postgres mock-provider backend
timeout 120 bash -c 'until curl -sf -o /dev/null http://127.0.0.1:3100/api-docs/openapi.json; do sleep 1; done'
dc exec -T postgres psql -q -U loadtest -d musicroom < seed.sql
rm -f results/room.md results/room.html

# 2. CPU and memory of the server every 5 seconds, while the test runs.
# Podman's --no-stream gives the average since the container started, not
# the current use: Podman streams instead, every 5 seconds.
start=$(date +%s)
stamp() { while read -r line; do echo "$(( $(date +%s) - start ))s $line"; done; }
format='{{.Name}} {{.CPUPerc}} {{.MemUsage}}'
if [ "$ENGINE" = podman ]; then
  podman stats --interval 5 --no-reset --format "$format" loadtest-backend loadtest-postgres \
    > >(stamp > results/room-resources.txt) &
else
  while sleep 5; do
    docker stats --no-stream --format "$format" loadtest-backend loadtest-postgres
  done > >(stamp > results/room-resources.txt) &
fi
sampler=$!
relay=
trap 'kill $sampler $relay 2>/dev/null; dcr stop -t 1 dashboard >/dev/null 2>&1' EXIT

# 3. The live dashboard. k6 writes room.html as soon as the test is over:
# that is when the relay is cut, so an open page cannot keep k6 waiting.
# (Fallback: the planned duration of the test, plus a margin.)
steps=$(( $(tr -cd ',' <<< "${STEPS:-500,1000,2000,4000,6000,8000}" | wc -c) + 1 ))
deadline=$(( steps * (20 + ${HOLD:-120}) + 120 ))
dcr up -d --force-recreate dashboard
( for _ in $(seq "$deadline"); do [ -f results/room.html ] && break; sleep 1; done
  dcr stop -t 1 dashboard ) >/dev/null 2>&1 &
relay=$!
echo "Live dashboard: http://localhost:5665"

# 4. The test.
# (-e NAME=value: Podman's compose does not accept a bare -e NAME.)
env=()
for var in STEPS HOLD; do [ -n "${!var:-}" ] && env+=(-e "$var=${!var}"); done
dcr run --rm --name loadtest-k6 "${env[@]}" -e K6_WEB_DASHBOARD_EXPORT=results/room.html \
  k6 run --quiet room.js

#!/usr/bin/env bash
# Runs one load test on a fresh server.
#
#   ./run.sh playlist
#   ./run.sh delegation
#   STEPS=500,1000,2000 HOLD=60 ./run.sh playlist     # other steps / duration
#
# Results in results/: <scenario>.md (verdict per step), <scenario>.html
# (k6's charts) and <scenario>-resources.txt (CPU and memory of the server).
set -euo pipefail
cd "$(dirname "$0")"
scenario=${1:?usage: ./run.sh playlist|delegation}

# k6 writes the results as the user running this script.
export LOADTEST_UID LOADTEST_GID
LOADTEST_UID=$(id -u)
LOADTEST_GID=$(id -g)
dc() { docker compose -f compose.yaml "$@"; }

# 1. A fresh server: empty database (it lives in memory), test accounts.
dc up -d --build --force-recreate postgres mock-provider backend
timeout 120 bash -c 'until curl -sf -o /dev/null http://127.0.0.1:3100/api-docs/openapi.json; do sleep 1; done'
dc exec -T postgres psql -q -U loadtest -d musicroom < seed.sql

# 2. CPU and memory of the server every 5 seconds, while the test runs.
resources="results/$scenario-resources.txt"
start=$(date +%s)
while sleep 5; do
  docker stats --no-stream --format "$(( $(date +%s) - start ))s {{.Name}} {{.CPUPerc}} {{.MemUsage}}" \
    music-room-loadtest-backend-1 music-room-loadtest-postgres-1
done > "$resources" &
trap 'kill $!' EXIT

# 3. The test.
dc run --rm -e STEPS -e HOLD -e K6_WEB_DASHBOARD_EXPORT="results/$scenario.html" \
  k6 run --quiet "$scenario.js"

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

# k6 writes the results as the user running this script.
export LOADTEST_UID LOADTEST_GID
LOADTEST_UID=$(id -u)
LOADTEST_GID=$(id -g)
dc() { docker compose -f compose.yaml "$@"; }

# 1. A fresh server: empty database (it lives in memory), test accounts.
dc up -d --build --force-recreate --wait postgres mock-provider backend
timeout 120 bash -c 'until curl -sf -o /dev/null http://127.0.0.1:3100/api-docs/openapi.json; do sleep 1; done'
dc exec -T postgres psql -q -U loadtest -d musicroom < seed.sql
rm -f results/room.md results/room.html

# 2. CPU and memory of the server every 5 seconds, while the test runs.
start=$(date +%s)
while sleep 5; do
  docker stats --no-stream --format "$(( $(date +%s) - start ))s {{.Name}} {{.CPUPerc}} {{.MemUsage}}" \
    music-room-loadtest-backend-1 music-room-loadtest-postgres-1
done > results/room-resources.txt &
sampler=$!
relay=
trap 'kill $sampler $relay 2>/dev/null; dc stop -t 1 dashboard >/dev/null 2>&1' EXIT

# 3. The live dashboard. k6 writes room.html as soon as the test is over:
# that is when the relay is cut, so an open page cannot keep k6 waiting.
# (Fallback: the planned duration of the test, plus a margin.)
steps=$(( $(tr -cd ',' <<< "${STEPS:-500,1000,2000,4000,6000,8000}" | wc -c) + 1 ))
deadline=$(( steps * (20 + ${HOLD:-120}) + 120 ))
dc up -d --force-recreate dashboard
( for _ in $(seq "$deadline"); do [ -f results/room.html ] && break; sleep 1; done
  dc stop -t 1 dashboard ) >/dev/null 2>&1 &
relay=$!
echo "Live dashboard: http://localhost:5665"

# 4. The test.
dc run --rm --name loadtest-k6 -e STEPS -e HOLD -e K6_WEB_DASHBOARD_EXPORT=results/room.html \
  k6 run --quiet room.js

// Helpers shared by playlist.js and delegation.js (read those first).
//
// The model: one k6 VU = one room. The VU creates the room, connects its
// members (one WebSocket each, like the app) and makes them act. The load
// grows in steps: more and more rooms, so more and more simultaneous users.

import http from 'k6/http';
import crypto from 'k6/crypto';
import encoding from 'k6/encoding';
import exec from 'k6/execution';
import { WebSocket } from 'k6/websockets';
import { setTimeout } from 'k6/timers';
import { Rate, Trend } from 'k6/metrics';

const BASE_URL = __ENV.BASE_URL || 'http://localhost:3100';
const WS_URL = BASE_URL.replace(/^http/, 'ws');
const JWT_SECRET = __ENV.JWT_SECRET || 'loadtest-secret';

// ------------------------------------------------------------ load profile

// Number of simultaneous users of each step, e.g. STEPS=500,1000,2000.
const STEPS = (__ENV.STEPS || '2000,4000,8000,12000,16000').split(',').map(Number);
const RAMP_SECONDS = 20; // time for a step's new users to join
const HOLD_SECONDS = Number(__ENV.HOLD || 60); // time the step is measured
const STEP_SECONDS = RAMP_SECONDS + HOLD_SECONDS;

// What every user must experience for a step to pass.
const CRITERIA = {
  ws_connect_ok: ['rate>0.99'], // gets into its room
  action_errors: ['rate<0.01'], // requests failing (5xx, timeout)
  http_req_duration: ['p(95)<500'], // API response time
  state_delivery_ms: ['p(95)<300'], // room update reaching every member
};

// `extraCriteria` may target one kind of request: 'http_req_duration{op:edit}'.
export function loadProfile(roomSize, extraCriteria = {}) {
  const stages = [];
  const thresholds = {};
  for (const users of STEPS) {
    const rooms = Math.ceil(users / roomSize);
    stages.push({ duration: `${RAMP_SECONDS}s`, target: rooms });
    stages.push({ duration: `${HOLD_SECONDS}s`, target: rooms });
    for (const [key, rules] of Object.entries({ ...CRITERIA, ...extraCriteria })) {
      thresholds[withStep(key, users)] = rules;
    }
  }
  return {
    scenarios: { rooms: { executor: 'ramping-vus', stages, gracefulStop: '10s' } },
    thresholds,
    summaryTrendStats: ['avg', 'med', 'p(95)', 'p(99)', 'max'],
  };
}

// 'http_req_duration{op:edit}' -> 'http_req_duration{step:2000,op:edit}'
export function withStep(key, users) {
  const [metric, tags] = key.split('{');
  return tags ? `${metric}{step:${users},${tags}` : `${metric}{step:${users}}`;
}

function elapsedSeconds() {
  return exec.instance.currentTestRunDuration / 1000;
}

export function testIsOver() {
  return elapsedSeconds() >= STEPS.length * STEP_SECONDS;
}

// Tag of every measurement: the number of users of the step being held, or
// "ramp" while users are joining (not judged).
export function currentStep() {
  const t = elapsedSeconds();
  const index = Math.min(Math.floor(t / STEP_SECONDS), STEPS.length - 1);
  return t % STEP_SECONDS < RAMP_SECONDS ? 'ramp' : String(STEPS[index]);
}

// ------------------------------------------------------------ metrics

const connected = new Rate('ws_connect_ok');
const actionErrors = new Rate('action_errors');
const stateDelivery = new Trend('state_delivery_ms', true);

// ------------------------------------------------------------ users

// Test accounts lt_0, lt_1... are created by seed.sql with the id
// md5('lt-user-<n>'). k6 signs their access token itself, as /auth/login
// would: logging thousands of users in would only measure password hashing.
function userId(n) {
  const h = crypto.md5(`lt-user-${n}`, 'hex');
  return `${h.slice(0, 8)}-${h.slice(8, 12)}-${h.slice(12, 16)}-${h.slice(16, 20)}-${h.slice(20)}`;
}

function accessToken(n) {
  const encode = (value) => encoding.b64encode(JSON.stringify(value), 'rawurl');
  const header = encode({ alg: 'HS256', typ: 'JWT' });
  const claims = encode({
    username: `lt_${n}`,
    user_id: userId(n),
    exp: Math.floor(Date.now() / 1000) + 4 * 3600,
  });
  const signature = crypto.hmac('sha256', JWT_SECRET, `${header}.${claims}`, 'base64rawurl');
  return `${header}.${claims}.${signature}`;
}

// The users of this VU's room; the first one owns it.
export function roomMembers(roomSize) {
  const first = (exec.vu.idInTest - 1) * roomSize;
  return Array.from({ length: roomSize }, (_, k) => {
    const n = first + k;
    return { n, id: userId(n), token: accessToken(n) };
  });
}

// ------------------------------------------------------------ actions

// Calls the API as `user`. Asynchronous, so the other members of the room
// keep receiving their messages meanwhile. `op` names the kind of request
// in the results (edit, delegate...).
export async function api(user, method, path, body, op = 'other') {
  const step = currentStep();
  const res = await http.asyncRequest(method, `${BASE_URL}${path}`,
    body === undefined ? null : JSON.stringify(body), {
      headers: { Authorization: `Bearer ${user.token}`, 'Content-Type': 'application/json' },
      tags: { step, op },
      // 404/409/422: another user changed the same thing first. Expected
      // with concurrent users, and not a failure of the server.
      responseCallback: http.expectedStatuses({ min: 200, max: 299 }, 404, 409, 422),
    });
  actionErrors.add(res.status === 0 || res.status >= 500, { step });
  return res;
}

export async function createRoom(owner) {
  const res = await api(owner, 'POST', '/rooms');
  return res.json('id');
}

// chrono writes nanoseconds; Date.parse only reads milliseconds.
function parseTime(value) {
  return Date.parse(String(value).replace(/(\.\d{3})\d+/, '$1'));
}

// Opens the app of `user` in the room: one WebSocket, kept until leave().
// Resolves once the first room state has arrived. `onState` is called with
// every room state the user receives.
export function join(user, roomId, onState) {
  return new Promise((resolve) => {
    const ws = new WebSocket(`${WS_URL}/rooms/${roomId}/ws`, null, {
      headers: { Authorization: `Bearer ${user.token}` },
    });
    user.send = (type, payload) => ws.send(JSON.stringify({ type, payload }));
    user.leave = () => ws.close();

    ws.onmessage = (message) => {
      const { type, payload } = JSON.parse(message.data);
      if (type === 'Welcome') user.connectionId = payload.connection_id;
      if (type !== 'RoomState') return;

      stateDelivery.add(Date.now() - parseTime(payload.server_time), { step: currentStep() });
      user.state = payload;
      if (onState) onState(user, payload);
      resolve(true);
    };
    ws.onclose = () => resolve(false);
  }).then((ok) => {
    connected.add(ok, { step: currentStep() });
    return ok;
  });
}

// Waits a random time, but never past the end of the test.
export function pause(minSeconds, maxSeconds) {
  const seconds = minSeconds + Math.random() * (maxSeconds - minSeconds);
  const left = STEPS.length * STEP_SECONDS - elapsedSeconds();
  return new Promise((resolve) => setTimeout(resolve, Math.max(0, Math.min(seconds, left)) * 1000));
}

export function pick(items) {
  return items[Math.floor(Math.random() * items.length)];
}

// ------------------------------------------------------------ summary

// One line per step: did it pass, and the key figures behind the verdict.
// Written to results/<scenario>.md and printed (in colour) at the end.
export function stepReport(scenario, data, columns) {
  const value = (metric, stat) => data.metrics[metric] && data.metrics[metric].values[stat];
  const failures = (users) => Object.entries(data.metrics)
    .filter(([name]) => name.includes(`{step:${users},`) || name.includes(`{step:${users}}`))
    .flatMap(([name, metric]) => Object.entries(metric.thresholds || {})
      .filter(([, result]) => !result.ok)
      .map(([rule]) => {
        // 'http_req_duration{step:8000,op:edit}' -> 'http_req_duration{op:edit}',
        // then the column's title when there is one: 'Edit response p95 (p(95)<500)'
        const metric = name.replace(/step:\d+,?/, '').replace(',}', '}').replace('{}', '');
        const column = columns.find((c) => c.metric === metric);
        return column ? `${column.title} (${rule})` : `${metric} ${rule}`;
      }));

  const steps = STEPS.map((users) => ({ users, failed: failures(users) }));
  const header = ['Users', 'Verdict', ...columns.map((c) => c.title)];
  const rows = steps.map(({ users, failed }) => [
    String(users), failed.length ? 'FAIL' : 'PASS',
    ...columns.map((c) => c.format(value(withStep(c.metric, users), c.stat))),
  ]);

  // The highest step such that it and every step below it passed.
  const firstFail = steps.findIndex((s) => s.failed.length);
  const held = firstFail === -1 ? steps.length : firstFail;
  const verdict = held === 0
    ? `**Not even ${steps[0].users} users**: the first step fails.`
    : held === steps.length
      ? `**${steps[held - 1].users} users supported**: every step passes.`
      : `**${steps[held - 1].users} users supported**: ${steps[held].users} fails.`;
  const notes = steps.filter((s) => s.failed.length)
    .map((s) => `- ${s.users} users fail on: ${s.failed.join(', ')}`);

  const render = (paint) => [
    `# ${scenario}`, '',
    paint(verdict, held === steps.length ? GREEN : RED), '',
    `Each step: ${RAMP_SECONDS} s for the new users to join, then ${HOLD_SECONDS} s measured.`, '',
    markdownTable(header, rows, (padded, cell) => (cell === 'PASS' ? paint(padded, GREEN)
      : cell === 'FAIL' ? paint(padded, RED) : padded)),
    ...(notes.length ? ['', ...notes] : []), '',
  ].join('\n');

  return {
    [`results/${scenario}.md`]: render((text) => text),
    stdout: `\n${render((text, colour) => `${colour}${text.replace(/\*\*/g, '')}${RESET}`)}`,
  };
}

const GREEN = '\x1b[32m';
const RED = '\x1b[31m';
const RESET = '\x1b[0m';

// Columns padded to the same width (still a valid Markdown table), figures
// aligned right. `paint(padded, cell)` may colour a cell once it is padded.
export function markdownTable(header, rows, paint = (cell) => cell) {
  const widths = header.map((title, i) => Math.max(title.length, ...rows.map((r) => r[i].length)));
  const numeric = header.map((_, i) => rows.every((r) => /^[\d.<>-]/.test(r[i])));
  const pad = (cell, i) => (numeric[i] ? cell.padStart(widths[i]) : cell.padEnd(widths[i]));
  const line = (cells) => `| ${cells.join(' | ')} |`;
  return [
    line(header.map(pad)),
    line(widths.map((w, i) => (numeric[i] ? `${'-'.repeat(w - 1)}:` : '-'.repeat(w)))),
    ...rows.map((row) => line(row.map((cell, i) => paint(pad(cell, i), cell)))),
  ].join('\n');
}

export const ms = (v) => (v === undefined ? '-' : `${Math.round(v)} ms`);
// 1 -> '100 %', 0.99953 -> '99.95 %', 0 -> '0 %'
export const percent = (v) => (v === undefined ? '-' : `${+(v * 100).toFixed(2)} %`);

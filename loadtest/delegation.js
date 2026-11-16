// Load test of the Music Control Delegation.
//
// Each room has 10 people. Every 15 to 30 seconds the owner hands playback
// control to one member's device (or, one time in five, takes it back). The
// device in control then moves the playback every 2 to 4 seconds, and every
// member must receive it.
//
// The device in control sends its own clock as the position, so each member
// can tell how long the command took to reach it: control_latency_ms.

import { Trend } from 'k6/metrics';
import {
  api, createRoom, currentStep, join, loadProfile, ms, pause, percent, pick, roomMembers,
  stepReport, testIsOver,
} from './lib.js';

const ROOM_SIZE = 10;
const controlLatency = new Trend('control_latency_ms', true);

export const options = loadProfile(ROOM_SIZE, { control_latency_ms: ['p(95)<300'] });

export default async function () {
  if (testIsOver()) return;

  // This VU's room: the owner creates it and opens it, then the others join.
  const people = roomMembers(ROOM_SIZE);
  const [owner, ...members] = people;
  const roomId = await createRoom(owner);
  await join(owner, roomId, measureControl);
  await Promise.all(members.map((member) => join(member, roomId, measureControl)));
  const connected = members.filter((member) => member.connectionId);

  while (!testIsOver()) {
    // Choose who drives: a member's device, or the owner taking control back.
    let driver;
    if (connected.length === 0 || Math.random() < 0.2) {
      driver = owner;
      await api(owner, 'DELETE', `/rooms/${roomId}/delegate`);
    } else {
      driver = pick(connected);
      await api(owner, 'POST', `/rooms/${roomId}/delegate`, {
        user_id: driver.id,
        connection_id: driver.connectionId,
      });
    }

    // The driver moves the playback for 15 to 30 seconds.
    const until = Date.now() + (15 + Math.random() * 15) * 1000;
    while (Date.now() < until && !testIsOver()) {
      await pause(2, 4);
      driver.send('SeekTo', { position: clock() });
    }
  }
  people.forEach((person) => person.leave());
}

// Positions are 32-bit integers: the clock is sent modulo a billion ms.
function clock() {
  return Date.now() % 1e9;
}

// Called with every room state a member receives.
function measureControl(person, state) {
  if (state.current_position === person.lastPosition) return;
  person.lastPosition = state.current_position;
  const latency = clock() - state.current_position;
  // The very first state (position 0) is not a command.
  if (latency >= 0 && latency < 60000) controlLatency.add(latency, { step: currentStep() });
}

export function handleSummary(data) {
  return stepReport('delegation', data, [
    { title: 'In the room', metric: 'ws_connect_ok', stat: 'rate', format: percent },
    { title: 'Delegation response p95', metric: 'http_req_duration', stat: 'p(95)', format: ms },
    { title: 'Command reaching members p95', metric: 'control_latency_ms', stat: 'p(95)', format: ms },
    { title: 'Errors', metric: 'action_errors', stat: 'rate', format: percent },
  ]);
}

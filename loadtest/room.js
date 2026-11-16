// Load test of both services at once: Music Playlist Editor and Music
// Control Delegation, used together in every room as people really would.
//
// Each room has 20 people who keep it open (one WebSocket each):
//   - every member edits the shared queue every 10 to 30 seconds: adds a
//     track, removes one or moves one;
//   - every 15 to 30 seconds the owner hands playback control to a member's
//     device (or, one time in five, takes it back), and whoever holds it
//     moves the playback every 2 to 4 seconds.
// Every change is sent to the whole room.
//
// Measured per service: the response time of edits and of delegations (the
// server broadcasts a change before it answers, so this includes the
// broadcast), the time a room update takes to reach each member, and the
// time a playback command takes to reach each member.

import { Trend } from 'k6/metrics';
import {
  api, createRoom, currentStep, join, loadProfile, ms, pause, percent, pick, roomMembers,
  stepReport, testIsOver,
} from './lib.js';

const ROOM_SIZE = 20;
const commandLatency = new Trend('command_latency_ms', true);

export const options = loadProfile(ROOM_SIZE, {
  'http_req_duration{op:edit}': ['p(95)<500'],
  'http_req_duration{op:delegate}': ['p(95)<500'],
  command_latency_ms: ['p(95)<300'],
});

export default async function () {
  if (testIsOver()) return;

  // This VU's room: the owner creates it and opens it, then the others join.
  const people = roomMembers(ROOM_SIZE);
  const [owner, ...members] = people;
  const roomId = await createRoom(owner);
  await join(owner, roomId, measureCommand);
  await Promise.all(members.map((member) => join(member, roomId, measureCommand)));

  // Both services run side by side until the end of the test.
  await Promise.all([
    ...people.map((person) => editQueue(person, roomId)),
    delegateControl(owner, members, roomId),
  ]);
  people.forEach((person) => person.leave());
}

// ------------------------------------------------- Music Playlist Editor

async function editQueue(person, roomId) {
  const path = `/rooms/${roomId}/queue`;
  while (!testIsOver()) {
    await pause(10, 30);
    if (!person.state) continue; // never got into the room

    const queue = person.state.queue;
    if (queue.length < 10) {
      // A track among 300, like the popular tracks of a real audience.
      await api(person, 'POST', path, { track_id: 1 + Math.floor(Math.random() * 300) }, 'edit');
    } else if (Math.random() < 0.5) {
      await api(person, 'DELETE', path, { id: pick(queue).id }, 'edit');
    } else {
      await api(person, 'PATCH', path, { id: pick(queue).id, new_position: Math.random() * 1000 }, 'edit');
    }
  }
}

// ------------------------------------------------- Music Control Delegation

async function delegateControl(owner, members, roomId) {
  const path = `/rooms/${roomId}/delegate`;
  const connected = members.filter((member) => member.connectionId);

  while (!testIsOver()) {
    // Choose who drives: a member's device, or the owner taking control back.
    let driver;
    if (connected.length === 0 || Math.random() < 0.2) {
      driver = owner;
      await api(owner, 'DELETE', path, undefined, 'delegate');
    } else {
      driver = pick(connected);
      await api(owner, 'POST', path, {
        user_id: driver.id,
        connection_id: driver.connectionId,
      }, 'delegate');
    }

    // The driver moves the playback for 15 to 30 seconds.
    const until = Date.now() + (15 + Math.random() * 15) * 1000;
    while (Date.now() < until && !testIsOver()) {
      await pause(2, 4);
      driver.send('SeekTo', { position: clock() });
    }
  }
}

// The driver sends its clock as the position (modulo a billion ms: positions
// are 32-bit integers), so each member can tell how long the command took.
function clock() {
  return Date.now() % 1e9;
}

// Called with every room state a member receives.
function measureCommand(person, state) {
  if (state.current_position === person.lastPosition) return;
  person.lastPosition = state.current_position;
  const latency = clock() - state.current_position;
  // Other position changes (a new track starts at 0) are not commands.
  if (latency >= 0 && latency < 60000) commandLatency.add(latency, { step: currentStep() });
}

export function handleSummary(data) {
  return stepReport('room', data, [
    { title: 'In the room', metric: 'ws_connect_ok', stat: 'rate', format: percent },
    { title: 'Edit response p95', metric: 'http_req_duration{op:edit}', stat: 'p(95)', format: ms },
    { title: 'Delegation response p95', metric: 'http_req_duration{op:delegate}', stat: 'p(95)', format: ms },
    { title: 'Update delivery p95', metric: 'state_delivery_ms', stat: 'p(95)', format: ms },
    { title: 'Command delivery p95', metric: 'command_latency_ms', stat: 'p(95)', format: ms },
    { title: 'Errors', metric: 'action_errors', stat: 'rate', format: percent },
  ]);
}

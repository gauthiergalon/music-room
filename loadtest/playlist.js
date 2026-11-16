// Load test of the Music Playlist Editor.
//
// Each room has 25 people. Everyone keeps the room open (one WebSocket) and,
// every 10 to 30 seconds, edits the shared queue: adds a track, removes one
// or moves one. Every edit is sent to the whole room.
//
// The server broadcasts the new queue before it answers the request, so the
// API response time already includes the broadcast; state_delivery_ms adds
// the time for it to reach each member.

import {
  api, createRoom, join, loadProfile, ms, pause, percent, pick, roomMembers,
  stepReport, testIsOver,
} from './lib.js';

const ROOM_SIZE = 25;

export const options = loadProfile(ROOM_SIZE);

export default async function () {
  if (testIsOver()) return;

  // This VU's room: the owner creates it and opens it, then the others join.
  const people = roomMembers(ROOM_SIZE);
  const roomId = await createRoom(people[0]);
  await join(people[0], roomId);
  await Promise.all(people.slice(1).map((person) => join(person, roomId)));

  // Everyone edits the queue until the end of the test.
  await Promise.all(people.map((person) => editQueue(person, roomId)));
  people.forEach((person) => person.leave());
}

async function editQueue(person, roomId) {
  const path = `/rooms/${roomId}/queue`;
  while (!testIsOver()) {
    await pause(10, 30);
    if (!person.state) continue; // never got into the room

    const queue = person.state.queue;
    if (queue.length < 10) {
      // A track among 300, like the popular tracks of a real audience.
      await api(person, 'POST', path, { track_id: 1 + Math.floor(Math.random() * 300) });
    } else if (Math.random() < 0.5) {
      await api(person, 'DELETE', path, { id: pick(queue).id });
    } else {
      await api(person, 'PATCH', path, { id: pick(queue).id, new_position: Math.random() * 1000 });
    }
  }
}

export function handleSummary(data) {
  return stepReport('playlist', data, [
    { title: 'In the room', metric: 'ws_connect_ok', stat: 'rate', format: percent },
    { title: 'Edit response p95', metric: 'http_req_duration', stat: 'p(95)', format: ms },
    { title: 'Update delivery p95', metric: 'state_delivery_ms', stat: 'p(95)', format: ms },
    { title: 'Errors', metric: 'action_errors', stat: 'rate', format: percent },
  ]);
}

# Load test: how many simultaneous users?

This measures how many people can use the two services of Music Room at the
same time — **Music Playlist Editor** and **Music Control Delegation** — on
a server sized like a low-end VPS, and whether they all still get a
real-time experience.

## Server

The backend and its database run in Docker with hard CPU and memory limits,
together the equivalent of a **low-end VPS with 2 vCPU and 2 GB of RAM**:

| Component | CPU | RAM |
|---|---|---|
| Backend (Rust, Axum) | 1.5 vCPU | 1.5 GB |
| PostgreSQL 18 | 0.5 vCPU | 512 MB |

| Host (on premise) | |
|---|---|
| CPU | Intel Core i7-1260P (12 cores, 16 threads), laptop |
| RAM | 16 GB |
| OS | Fedora Linux 44, kernel 7.2, Docker 29.9 |

The load generator (k6) runs on the same host, outside these limits, so it
does not take resources from the server under test. The music provider is
replaced by a local stand-in that answers instantly (`mock-provider/`): the
test measures our backend, and must not send thousands of requests to the
real provider.

## What is simulated

One simulated user behaves like one instance of the app: it opens its room
(one WebSocket, kept for the whole test), receives every update of the room,
and acts through the REST API with pauses, like a person.

Both services are used **at the same time, in every room**
([room.js](room.js)), as people would at a party. Each room has 20 people:

- **Music Playlist Editor**: every 10 to 30 seconds each member edits the
  shared queue — adds, removes or moves a track — and the whole room
  receives the change. Several people removing or moving the same track at
  once is expected: the server must refuse one of them cleanly (404/409),
  never fail.
- **Music Control Delegation**: every 15 to 30 seconds the owner hands
  playback control to another member's device, or takes it back. The device
  in control moves the playback every 2 to 4 seconds, and every member must
  receive it.

The load grows in steps: 500, 1000, 2000, 4000, 6000 then 8000
simultaneous users. At each step the new users join during 20 seconds,
then the step is measured for 2 minutes. Users of the previous steps stay
connected and active.

## Success criteria

A step is **supported** when, during its 2 minutes:

| Criterion | Threshold |
|---|---|
| Users who get into their room | > 99 % |
| Requests failing (5xx, timeout) | < 1 % |
| Response time of edits, of delegations, of all requests (p95) | < 500 ms |
| Room update reaching every member, p95 | < 300 ms |
| Playback command reaching every member, p95 | < 300 ms |

p95: 95 % of the measurements are below this value. The server broadcasts a
change before it answers the request, so the response time of an edit or a
delegation already includes the broadcast. 300 ms is about the delay from
which people in the same room notice that they no longer see or hear the
same thing.

The maximum number of simultaneous users is the **last step that passes**.

## Running it

Requirements: Docker with Compose, curl.

```bash
cd loadtest
./run.sh                                  # about 15 minutes
STEPS=500,1000,2000 HOLD=60 ./run.sh      # other steps, shorter
```

While it runs, **http://localhost:5665** shows k6's charts live (users,
requests, response times, errors). The page stops updating when the test
ends: k6 cannot finish while a page is connected to it, so the page goes
through a relay that `run.sh` cuts at the end.

Each run starts a fresh server with an empty database (separate from the
development stack: project `music-room-loadtest`, database in memory). Stop
it with `docker compose -f compose.yaml --profile run down`.

Results, in `results/`:

- `room.md`: the verdict and key figures of each step;
- `room.html`: the same charts as the live page, to open in a browser;
- `room-resources.txt`: CPU and memory of the backend and of PostgreSQL
  every 5 seconds (seconds since the start of the run).

### How the code is organised

- [room.js](room.js): what the people of one room do. One k6 VU = one room:
  it creates the room, connects its members, and runs both services until
  the end.
- [lib.js](lib.js): the shared pieces — the load steps and criteria,
  `join()` (open the room's WebSocket and measure each update), `api()`
  (a REST call that counts errors), the test accounts.
- [compose.yaml](compose.yaml), [seed.sql](seed.sql), [run.sh](run.sh):
  the server under test and how to start it.

Test accounts are created directly in the database (`seed.sql`) and k6 signs
their access tokens with the test server's secret: logging thousands of
users in would only measure password hashing, which is deliberately slow.

### Why k6

The services are real-time: a user is a WebSocket held open, receiving
updates nobody asked for, plus REST calls. Measuring them takes long-lived
WebSockets, per-user logic reacting to what is received, and custom
measurements such as the time a broadcast takes to reach each member.

- **AB, Siege** only replay HTTP requests: no WebSocket, no session, so
  they measure requests per second, not simultaneous users.
- **JMeter** needs a plugin for WebSockets and one thread per user, which
  limits a single machine to far fewer users.
- **Tsung, Gatling** handle WebSockets and many users, but are built
  around "send, then check the reply": measuring broadcasts received by
  every member is awkward.
- **k6** keeps thousands of WebSockets in one process, lets each user react
  to the messages it receives in JavaScript, and judges every step against
  the criteria above.

## Results

Measured on 16 November 2026 on the server described above, with both
services used at the same time in every room.

| Users | Verdict | In the room | Edit response p95 | Delegation response p95 | Update delivery p95 | Command delivery p95 | Errors |
|---|---|---|---|---|---|---|---|
| 500 | PASS | 100 % | 5 ms | 1 ms | 4 ms | 5 ms | 0 % |
| 1000 | PASS | 100 % | 4 ms | 1 ms | 4 ms | 6 ms | 0 % |
| 2000 | PASS | 100 % | 3 ms | 1 ms | 4 ms | 6 ms | 0 % |
| 4000 | PASS | 100 % | 7 ms | 2 ms | 5 ms | 11 ms | 0 % |
| 6000 | PASS | 100 % | 16 ms | 5 ms | 8 ms | 18 ms | 0 % |
| 8000 | PASS | 100 % | 81 ms | 30 ms | 10 ms | 56 ms | 0 % |

Up to 4000 users everything stays around 10 ms. From 6000, response times
start to rise, and at 8000 the server nears its limits: the backend uses
1.36 GB of its 1.5 GB of memory and about half of its 1.5 vCPU, PostgreSQL
about 43 % of its 0.5 vCPU. Every criterion is still met, far below the
thresholds (81 ms for 500 ms allowed, 56 ms for 300 ms).

### Conclusion

On a 2 vCPU / 2 GB server, **8000 people can use the Playlist Editor and
the Control Delegation at the same time**, every criterion met. This is
consistent with a low-end server ("thousands of users").

What gives way first, and how to push it further:

- **Memory**: about 150 KB per connected user, mostly the read and write
  buffers kept for each WebSocket. Smaller buffers would raise the
  capacity at no other cost.
- **Database CPU**: each queue edit is followed by a full read of the queue
  to broadcast it. More CPU for PostgreSQL, or sending only the change,
  would push the limit.

Earlier runs of each service alone (same server) gave the same picture:
the Playlist Editor alone still passed at 10000 users, with PostgreSQL's
CPU and the backend's memory at their limits; the Control Delegation alone
passed at 8000 users using half a core.

These figures are a little optimistic: a vCPU of the host's i7 is faster
than a typical shared VPS vCPU, and k6 reaches the server without a real
network in between.

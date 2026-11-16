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

**Music Playlist Editor** ([playlist.js](playlist.js)): rooms of 25 people.
Every 10 to 30 seconds each of them edits the shared queue — adds, removes
or moves a track — and the whole room receives the change. Several people
removing or moving the same track at once is expected: the server must
refuse one of them cleanly (404/409), never fail.

**Music Control Delegation** ([delegation.js](delegation.js)): rooms of 10
people. Every 15 to 30 seconds the owner hands playback control to another
member's device, or takes it back. The device in control moves the playback
every 2 to 4 seconds, and every member must receive it.

The load grows in steps: 500, 1000, 2000, 3000 then 5000 simultaneous users.
At each step the new users join during 20 seconds, then the step is measured
for 2 minutes. Users of the previous steps stay connected and active.

## Success criteria

A step is **supported** when, during its 2 minutes:

| Criterion | Threshold |
|---|---|
| Users who get into their room | > 99 % |
| Requests failing (5xx, timeout) | < 1 % |
| API response time, 95th percentile | < 500 ms |
| Room update reaching every member, p95 | < 300 ms |
| Control command reaching every member, p95 (delegation) | < 300 ms |

The server broadcasts a change before it answers the request, so the API
response time of an edit or a delegation already includes the broadcast.
300 ms is about the delay from which people in the same room notice that
they no longer see or hear the same thing.

The maximum number of simultaneous users of a service is its **last step
that passes**.

## Running it

Requirements: Docker with Compose, curl.

```bash
cd loadtest
./run.sh playlist          # about 12 minutes
./run.sh delegation
STEPS=500,1000 HOLD=60 ./run.sh playlist    # other steps, shorter
```

Each run starts a fresh server with an empty database (separate from the
development stack: project `music-room-loadtest`, database in memory). Stop
it with `docker compose -f compose.yaml down`.

Results, in `results/`:

- `<scenario>.md`: the verdict and key figures of each step;
- `<scenario>.html`: k6's charts of the whole run, to open in a browser;
- `<scenario>-resources.txt`: CPU and memory of the backend and of
  PostgreSQL every 5 seconds (seconds since the start of the run).

### How the code is organised

- [playlist.js](playlist.js), [delegation.js](delegation.js): what the
  people of one room do. One k6 VU = one room: it creates the room,
  connects its members, makes them act until the end.
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

Measured on 16 November 2026 on the server described above, in two runs
per service (500 to 5000 users, then 6000 to 10000). p95: 95 % of the
measurements are below this value.

### Music Playlist Editor

| Users | Verdict | In the room | Edit response p95 | Update delivery p95 | Errors |
|---|---|---|---|---|---|
| 500 | PASS | 100 % | 5 ms | 4 ms | 0 % |
| 1000 | PASS | 100 % | 4 ms | 4 ms | 0 % |
| 2000 | PASS | 100 % | 3 ms | 4 ms | 0 % |
| 3000 | PASS | 100 % | 4 ms | 5 ms | 0 % |
| 5000 | PASS | 100 % | 8 ms | 6 ms | 0 % |
| 6000 | PASS | 100 % | 12 ms | 8 ms | 0 % |
| 8000 | PASS | 100 % | 69 ms | 11 ms | 0 % |
| 10000 | PASS | 100 % | 246 ms | 13 ms | 0 % |

At 5000 users the server is far from its limits (backend about 24 % of a
core, PostgreSQL about 25-40 %). From 8000 users the response time of edits
rises: at 10000, PostgreSQL uses all of its 0.5 vCPU (every edit writes the
queue and reads it back to broadcast it) and the backend reaches its 1.5 GB
of memory. 10000 users still pass, but with no margin left.

### Music Control Delegation

| Users | Verdict | In the room | Delegation response p95 | Command reaching members p95 | Errors |
|---|---|---|---|---|---|
| 500 to 5000 | PASS | 100 % | 1 ms | 2 ms | 0 % |
| 6000 | PASS | 100 % | 1 ms | 2 ms | 0 % |
| 8000 | PASS | 100 % | 1 ms | 2 ms | 0 % |

Delegation and playback commands go through the server's memory, with one
database write per command, so they cost little: at 8000 users the backend
uses about half a core and 1.2 GB. Beyond 10000 users the server's limit of
1000 simultaneous rooms (10 people each here) is reached; that limit is a
setting, not a lack of resources.

### Conclusion

On a 2 vCPU / 2 GB server, both services can be used by **8000
simultaneous users** with every criterion met and a comfortable margin;
the Playlist Editor still passes at 10000, at the limit of the database
and of the memory. This is consistent with a low-end server ("thousands of
users").

What gives way first, and how to push it further:

- **Memory**: about 150 KB per connected user, mostly the read and write
  buffers kept for each WebSocket. Smaller buffers would raise the
  capacity at no other cost.
- **Database CPU** for the Playlist Editor: each edit is followed by a full
  read of the queue for the broadcast. More CPU for PostgreSQL, or sending
  only the change, would push the limit.

These figures are a little optimistic: a vCPU of the host's i7 is faster
than a typical shared VPS vCPU, and k6 reaches the server without a real
network in between.

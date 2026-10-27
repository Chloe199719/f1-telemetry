# F1 25 telemetry starter

A Rust project for an F1 25 UDP reader. The reader in `src/main.rs` and `src/f1_25/`
is yours to implement. The simulator is a separate binary in `src/bin/simulator.rs`,
with independent encoders in `src/bin/simulator/packets.rs`.

## Dependencies

| Crate | Purpose |
| --- | --- |
| `tokio` | Async UDP sockets, timers, Ctrl+C, and the runtime |
| `bytes` | Reading and writing binary fields with explicit endianness |
| `clap` | Command line arguments |
| `anyhow` | Application errors with context |
| `tracing`, `tracing-subscriber` | Logging, configurable with `RUST_LOG` |

## Run the simulator

Start your UDP listener first, then run:

```sh
cargo run --bin simulator
```

Defaults: destination `127.0.0.1:20777`, all 16 packet types enabled, 60 Hz for
high-frequency packets, 10 seconds per synthetic lap, and one event fixture every
500 ms. Stop with Ctrl+C. A 10-second run exercises all 21 event codes:

```sh
cargo run --bin simulator -- --duration 10
cargo run --bin simulator -- --target 127.0.0.1:20777 --hz 30 --duration 60 --lap-seconds 10
cargo run --bin simulator -- --help
```

`--target` takes an IP address and port, including IPv6 such as `[::1]:20777`.
`--hz` accepts 1–240; duration is positive whole seconds, and lap length is 1–3600
seconds. Fixed-rate packets retain the rates listed below, including at low `--hz`.
Events are sent on scheduler ticks; their interval can be shortened with
`--event-interval-ms`, for example `--event-interval-ms 50` for faster coverage.
UDP sends do not guarantee delivery, so start the listener before the simulator
to capture the initial events.

To isolate specific packet IDs:

```sh
# Original telemetry + events stream
cargo run --bin simulator -- --packets 3,6 --duration 10
# Only the session packet
cargo run --bin simulator -- --packets 1 --duration 5
```

To build a standalone executable:

```sh
cargo build --release --bin simulator
./target/release/simulator --duration 30
```

## Simulated packets

Uses the **2025 UDP format**, packet version 1, with packed little-endian fields
checked against the included [Data Output from F1 25 v3.pdf](Data%20Output%20from%20F1%2025%20v3.pdf).
Every listed payload field is encoded, including fixed-capacity arrays and unused
entries. Packet sizes include the header:

| ID | Packet | Bytes | Send rate |
| --- | --- | --- | --- |
| 0 | Motion | 1349 | `--hz` |
| 1 | Session | 753 | 2 Hz |
| 2 | Lap Data | 1285 | `--hz` |
| 3 | Event | 45 | Startup, configured interval, and shutdown |
| 4 | Participants | 1284 | Every 5 seconds |
| 5 | Car Setups | 1133 | 2 Hz |
| 6 | Car Telemetry | 1352 | `--hz` |
| 7 | Car Status | 1239 | `--hz` |
| 8 | Final Classification | 1042 | On shutdown |
| 9 | Lobby Info | 954 | 2 Hz |
| 10 | Car Damage | 1041 | 10 Hz |
| 11 | Session History | 1460 | 20 Hz, cycling through all 22 cars; bulk update on shutdown |
| 12 | Tyre Sets | 231 | 20 Hz, cycling through all 22 cars |
| 13 | Motion Ex | 273 | `--hz` |
| 14 | Time Trial | 101 | 1 Hz |
| 15 | Lap Positions | 1131 | 1 Hz; a second chunk after 50 laps |

All periodic packet types send immediately at startup, then follow their rate.
Headers contain all 12 fields: packet format, game year, major/minor game version,
packet version, packet ID, session UID, session time, frame identifier, overall
frame identifier, player index, and secondary player index. The header is 29
bytes; player indices are 0 and 255. Each run has a new session UID.

The F1 25 additions are included: 32-byte UTF-8 names and livery colours,
retirement/DRS-disabled reasons, stop-go service time, C6 tyres, final result
reason, tyre blisters, chassis pitch, wheel camber/gain, and lap positions.
Wheel arrays use rear-left, rear-right, front-left, front-right order, with distinct
values where useful to detect ordering errors. Final classification race time is
an 8-byte double; other floating-point fields use 4 bytes.

## Events and fixture behaviour

All 21 event codes in the PDF are emitted, with their respective details and a
12-byte padded event union:

```text
SSTA SEND FTLP RTMT DRSE DRSD TMPT CHQF RCWN PENA SPTP
STLG LGOT DTSV SGSV FLBK BUTN RDFL OVTK SCAR COLL
```

Startup sends `SSTA`, `STLG`, `LGOT`, and `DRSE`. The 15 running events cycle at the
configured interval. Shutdown sends `CHQF`, `RCWN`, final classification, the bulk
history update, and finally `SEND`, subject to `--packets` selection.

This is a protocol coverage stream, not a realistic race replay: lobby, race, and
time-trial fixtures are mixed; race-control events do not change the other fixture
states. `FLBK` targets the current frame and time, so it does not rewind the
stream. Lap histories cap at 100 records, lap positions use 50-row chunks, and
unused array entries are zeroed. Values are repeatable for a given simulation time.
This targets F1 25 v3, not the 2026 Season Pack format.

## Your reader

Implement the reader in `src/main.rs` and run it with `cargo run`. Bind to the
same destination port as the simulator. The largest packet in this specification
is Session History at 1460 bytes; use a buffer of at least that size (2048 bytes is
convenient). Read the header's packet ID to dispatch to your decoder. Keep the
reader's decoding logic independent of these fixture encoders.

```sh
cargo test --bin simulator
cargo test --test simulator_udp
cargo clippy --bin simulator -- -D warnings
```

The simulator tests check every packet's size and version, all header fields,
v3-specific offsets, 32-byte UTF-8 names, event union details/padding, sector
minute parts, signed tyre-set deltas, history limits, and lap-position chunks.
The UDP integration tests launch the binary, receive every packet type and all
21 event codes, verify car cycling and headers, and check packet filtering and
timed shutdown. They require permission to bind local UDP sockets.

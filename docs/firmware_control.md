# Espresso Machine Controller — Firmware Spec

Bare-metal Rust (`no_std`) firmware for an M5Stack AtomS3R (ESP32-S3) acting as the
**controller** in an espresso machine. A separate ESP32 driving a touchscreen acts as
the **UI node**. This document is the starting point for implementation; it captures
architecture decisions, the reasoning behind them, and the open questions that need
hardware verification.

> **Status legend used throughout**
> - `[DECIDED]` — settled, implement as written
> - `[PREFERRED]` — chosen approach, alternative documented as fallback
> - `[VERIFY]` — needs measurement on real hardware or confirmation against the TRM
> - `[OPEN]` — not yet decided

---

## 1. System overview

### Roles

**Controller (this firmware, AtomS3R / ESP32-S3):**
- Owns all real-time control. Autonomous — never blocked on the UI node.
- Phase-angle control of the AC pump via a RobotDyn 4A dimmer.
- Two relays over I²C (boiler / solenoid valve — assign in `config.rs`).
- Sensor acquisition: temperature, pressure, weight, over I²C and/or UART.
- Shot logic: "start, stop at N grams", plus local safety cutoffs.

**UI node (touchscreen ESP32, separate firmware, out of scope):**
- Sends commands (`Start`, `Stop`, …).
- Receives telemetry at ~20 Hz for live display.
- Owns WiFi for shot logging: connects to the network *after* the shot ends and
  POSTs the shot record to a backend. The controller is not involved.
- Can send a CAN command to ask the controller to bring up its own WiFi for
  debugging/OTA (see §5.3).

### Non-goals
No BLE, no audio, no microphone, no display, no FreeRTOS, no `std`.

---

## 2. Hardware

### Board
- **M5Stack AtomS3R** — ESP32-S3-PICO-1-N8R8: dual-core Xtensa LX7, 8 MB flash,
  8 MB octal PSRAM, 512 KB internal SRAM.
- **PSRAM is deliberately NOT enabled.** It shares the cache with flash and adds
  cache pressure for zero benefit here. Do not turn it on.

### Pin budget — 8 usable GPIOs total

| Function | Pins | Location | GPIO | Notes |
|---|---|---|---|---|
| I²C (relays + sensors) | 2 | Port.A / Grove | `[VERIFY]` | SDA/SCL, shared bus |
| UART (sensors) | 2 | Rear header | `[VERIFY]` | `[OPEN]` — may be unused if all sensors are I²C |
| TWAI / CAN | 2 | Rear header | `[VERIFY]` | TX/RX to transceiver |
| Dimmer | 2 | Rear header | `[VERIFY]` | ZC in, PSM out |

**Action for implementation:** read the AtomS3R schematic and fill the GPIO column
into `src/config.rs` as `const` values. Do not scatter pin numbers through the code.

Notes:
- The AtomS3R has an on-board IMU on an internal I²C bus — check whether it shares
  the Port.A bus before assuming addresses are free. `[VERIFY]`
- CAN needs an external transceiver (SN65HVD230 / TJA1051) on both nodes.
  At 5–10 cm, termination is nominally unnecessary but fit 120 Ω at both ends anyway.
- **PSM pin must have a pull-down** so the triac is off during reset and pre-init.

### RobotDyn AC Dimmer, 4A

- **ZC output:** opto-isolated (H11AA1 / 4N25 class), open-collector. Produces a pulse
  roughly **350 µs wide straddling the true zero crossing** — it is *not* an edge at
  the crossing. The rising edge is ~175 µs before the true zero, the falling edge
  ~175 µs after, and the phototransistor's slow turn-off makes this asymmetric.
  - Pick **one** edge and use it forever.
  - `ZC_OFFSET_US` is a calibration constant measured with a scope. `[VERIFY]`
  - May need an external pull-up. `[VERIFY]`
- **Gate side:** MOC3021 — a *random-phase* (non-zero-cross) opto-triac driving a
  BT136/BTA-class triac. Correct for phase control. The PSM input is effectively an
  LED: current flows while the pin is high.

### Load: OLAB Silent Green, 35 W vibratory pump

Solenoid — inductive. Current lags voltage. Relevant consequences in §6.

---

## 3. Toolchain and environment

### Target
```
target = "xtensa-esp32s3-none-elf"
```

Xtensa needs the Espressif LLVM fork. Install via `espup`:

```bash
cargo install espup --locked
espup install
# then source the export file espup writes (adds the Xtensa toolchain to PATH)
source ~/export-esp.sh
```

Nightly is likely required: `picoserve` has historically needed
`#![feature(impl_trait_in_assoc_type)]`. `[VERIFY]` against the picoserve version pinned.

### Flashing / monitoring
```bash
cargo install espflash --locked
cargo run --release        # via runner = "espflash flash --monitor" in .cargo/config.toml
```

### Bootstrapping
`esp-generate` produces a working baseline. Use it to get `.cargo/config.toml`,
`build.rs`, linker args and `rust-toolchain.toml` correct rather than hand-writing them:

```bash
cargo install esp-generate --locked
esp-generate --chip esp32s3 espresso-controller
```

### Critical build settings

```toml
[profile.dev]
opt-level = 3       # esp-radio does not work reliably below opt-level 2

[profile.release]
opt-level = 3
lto = "fat"
codegen-units = 1
debug = 2           # keep symbols; they don't ship in the .bin
```

**Logging:** `esp-radio` is full of trace-level logging that will wreck timing if
compiled in. Set `release_max_level_off` for the `log` crate, or use `defmt` with
level filtering. Never log from a priority-3 ISR.

---

## 4. Crates

```toml
[dependencies]
esp-hal            = { version = "1.1", features = ["esp32s3", "unstable"] }
esp-hal-embassy    = { version = "*",   features = ["esp32s3"] }
embassy-executor   = { version = "*",   features = ["task-arena-size-32768"] }
embassy-time       = "*"
embassy-sync       = "*"
embassy-futures    = "*"

portable-atomic    = { version = "1", features = ["require-cas"] }
static_cell        = "2"
heapless           = "0.8"
bitflags           = "2"

# --- link: can ---
embedded-can       = { version = "0.4", optional = true }

# --- link: wifi ---
esp-radio          = { version = "*", optional = true, features = ["esp32s3", "wifi"] }
esp-rtos           = { version = "*", optional = true, features = ["esp32s3", "esp-radio", "esp-alloc", "embassy"] }
esp-alloc          = { version = "*", optional = true, features = ["esp32s3"] }
embassy-net        = { version = "*", optional = true, features = ["tcp", "dhcpv4", "medium-ethernet"] }
picoserve          = { version = "*", optional = true, features = ["embassy"] }
serde              = { version = "1", optional = true, default-features = false, features = ["derive"] }
serde-json-core    = { version = "*", optional = true }
```

### Notes on the ecosystem

- `esp-hal` reached **1.0** in Oct 2025; the stable surface is small and most drivers
  (`twai`, `mcpwm`, `i2c`, `uart`) sit behind the `unstable` feature. Unstable here means
  *API* stability — the drivers work, but expect churn between versions.
  **Pin exact versions and read the migration guides when bumping.**
- **`esp-wifi` has been renamed `esp-radio`.** Older tutorials and blog posts reference
  `esp-wifi`; translate accordingly.
- `esp-radio` requires a **heap** (`esp-alloc`) and a **preemptive task scheduler**
  (`esp-rtos`). `esp-rtos` implements the capability set FreeRTOS provides (threads,
  queues, semaphores, timers) but is native Rust — this satisfies "no FreeRTOS" in the
  sense of no FreeRTOS C code, though it *is* a preemptive scheduler. There is no way
  around this if you want the radio.
- The CAN-only build pulls in **none** of that: no heap, no scheduler, no blob. Plain
  `esp-hal-embassy` cooperative executor only.

---

## 5. Build configuration

### 5.1 Feature flags

```toml
[features]
default        = ["link-can", "wifi-on-demand"]

# Transport selection — exactly one required
link-can       = ["dep:embedded-can"]
link-wifi      = ["_radio", "dep:embassy-net", "dep:picoserve", "dep:serde", "dep:serde-json-core"]

# CAN primary, WiFi compiled in but dormant until a CAN command enables it
wifi-on-demand = ["link-can", "_radio", "dep:embassy-net", "dep:picoserve", "dep:serde", "dep:serde-json-core"]

# Internal: the radio stack and its dependencies
_radio         = ["dep:esp-radio", "dep:esp-rtos", "dep:esp-alloc"]

ota            = []   # exposes the OTA POST endpoint; implies a radio feature
```

Enforce mutual exclusivity in `build.rs` or with a `compile_error!` in `lib.rs`.

### 5.2 The three configurations

| Config | Features | Use |
|---|---|---|
| **Dev** | `link-wifi` | Desk development. WiFi STA up at boot, WebSocket always available, no CAN hardware needed. Iterate from the Mac without sitting next to the machine. |
| **Production** | `link-can`, `wifi-on-demand` | CAN is the command path. Radio compiled in but the controller is *not* initialised and the RF is off until asked. |
| **Production-lean** | `link-can` only | Radio entirely absent from the binary. Smallest, fully deterministic, no heap. Ship this if OTA is handled another way. |

**Important discipline:** develop against the `link-wifi` config, which is the *harsher*
environment (scheduler, blob, RF bursts, heap). Shipping `link-can` then strictly removes
load. Validating on bare-metal CAN first and adding radio later is how you discover a
timing problem in production. **But** build and smoke-test the CAN-only config in CI or
it will bit-rot.

### 5.3 WiFi on demand

Flow:

1. UI node sends `Command::EnableWifi { minutes: u8 }` over CAN (button on the screen).
2. Controller starts `esp-rtos`, initialises `esp-radio`, brings up STA + DHCP, starts
   `picoserve`.
3. Controller reports its IP back over CAN as `Telemetry::WifiUp { ip: [u8; 4] }` so the
   screen can display it.
4. A timer (default 15 min) or `Command::DisableWifi` tears it back down.
5. **CAN remains the authoritative command path throughout.** WebSocket commands are
   debug-only and gated (see §7.2).

Constraints to design around:

- `esp-rtos` must be started **before** `esp_radio::init()`. `[VERIFY]` whether the
  scheduler can be started late (after `main` has been running on the plain embassy
  executor for minutes) or whether it must be started at boot and merely left idle.
  This is the highest-risk unknown in the whole design — if late start is not supported,
  the fallback is: always start `esp-rtos` at boot, but only call `esp_radio::init()`
  on demand. Prototype this early.
- The heap must be reserved at boot regardless (`esp_alloc::heap_allocator!`), so the
  RAM is spent whether or not WiFi is ever enabled in a `wifi-on-demand` build.
- Bringing the radio up mid-shot will perturb timing. **Refuse `EnableWifi` while a
  shot is in progress.**

---

## 6. Dimmer control

### 6.1 Principle

Phase-angle control. Each mains half-cycle: wait a delay after the zero crossing, then
gate the triac. Later firing = less power.

```
   ZC pulse   ┐‾‾‾‾‾┌───────────────────────────────────┐‾‾‾‾‾┌
              │350µs│                                   │
   true zero ─────┼─────────────────────────────────────────┼──
              ├offset┤
              ├────────── delay_us (from control loop) ──┤
   PSM        _____________________________________┌─────────┐__
                                                   fire      release
                                                        (~300µs before next ZC)
```

### 6.2 Implementation: priority-3 ISR `[PREFERRED]`

Chosen over the MCPWM approach. A 35 W vibratory pump regulated by a 20–50 Hz
pressure/flow loop tolerates a few hundred µs of jitter — the pump's mechanical
inertia and the control loop average it away. Visible-flicker-grade precision is not
required here, and an ISR is far easier to instrument and debug than MCPWM register
configuration.

**Structure — two-shot timer:**

```
GPIO ZC ISR   → validate edge → read PHASE_US → arm TIMG1 alarm A at (delay - offset)
Alarm A ISR   → PSM high      → arm alarm B at hold_us
Alarm B ISR   → PSM low
```

**Non-negotiable rules for both handlers:**

- `#[handler(priority = Priority::Priority3)]` — the highest level usable from Rust on
  Xtensa. Levels 4+ require assembly.
- `#[ram]` on the handlers, and `#[ram(rodata)]` on any data they read, so a flash cache
  disable (NVS write, OTA erase) cannot stall them.
- **Zero `critical_section::with` anywhere in the path.** On Xtensa, esp-hal's critical
  section raises `PS.INTLEVEL` above priority 3 — a CS taken by your own logging or by
  an embassy mutex will mask the dimmer. Communicate only via
  `portable_atomic::AtomicU32` (the LX7 has native `S32C1I` CAS, so no CS fallback).
- No logging, no allocation, no `defmt` from inside these handlers.
- Run them on **core 1** (see §8). On the S3, esp-hal's critical section is a spinlock
  plus *local* interrupt disable — a CS on core 0 does not mask core 1's interrupts.
  This is what makes the radio genuinely unable to delay the pump.

**ZC edge validation:** the RobotDyn ZC line is noisy and sits next to a triac switching
an inductive load. Reject any edge arriving less than ~7 ms after the previous one
(at 50 Hz). Count rejections and expose as telemetry.

### 6.3 Alternative: MCPWM hardware firing `[fallback]`

Documented for completeness; implement only if ISR jitter proves problematic.

Route ZC to an MCPWM `SYNC` input via the GPIO matrix. Timer 0 at 1 MHz in
**one-shot count-up, stop at period** mode, period ≈ 11 ms. Sync resets and starts the
counter. Generator actions: low at TEZ, **high at CMPR_A**, **low at CMPR_B**, low at TEP.
Software writes only the two compare registers (shadowed, updating at TEZ).

Properties: zero software in the firing path, jitter = one 1 µs tick, and a free
fail-safe — if ZC stops, no sync arrives, the timer stays stopped at period with the
output low, and the pump stops with no watchdog code.

Costs: esp-hal's `mcpwm` driver is unstable and almost certainly does not expose
external sync or one-shot mode, so this means raw PAC register work
(`esp_hal::peripherals::MCPWM0`). The one-shot-stop-at-TEP behaviour needs confirming
against the ESP32-S3 TRM. `[VERIFY]`

Note: the ESP32-S3 has **no ETM** (that arrived on C6/H2/P4), so there is no simpler
hardware event-routing option.

### 6.4 Gate pulse duration — long hold `[PREFERRED]`

A triac latches: once the gate triggers it *and* the main-terminal current exceeds the
**latching current** (I_L), it self-sustains with no gate drive until the current drops
below the holding current near the next zero crossing. For a **resistive** load, current
follows voltage instantly and a 50–100 µs pulse is plenty.

**The OLAB pump is inductive**, so current starts at zero at the firing instant and ramps
up at a rate set by L. If the gate pulse ends before current reaches I_L, the triac never
latches and the half-cycle is silently skipped. This is worst at low power (late firing =
less remaining voltage = slower ramp), which is exactly the operating region that matters
for pre-infusion and flow profiling. The symptom is erratic buzzing and inconsistent flow
at low settings that cleans up as power increases.

**Therefore: hold the gate high from the firing angle until shortly before the next zero
crossing.**

```
hold_us = half_period_us - ZC_OFFSET_US - delay_us - ZC_GUARD_US
```

Cost is ~15 mA through the MOC3021 LED for the remainder of the half-cycle — negligible
thermally and electrically. In exchange, an entire class of intermittent, load-dependent
misfires disappears.

**`ZC_GUARD_US` ≈ 300 µs is mandatory.** If the gate is still asserted across the zero
crossing, the triac retriggers immediately in the next half-cycle and jumps to full power
with no control. Make the "release" alarm unconditional and belt-and-braces: also force
PSM low in the ZC handler itself.

**Calibration task (do this early, with a scope and a current probe or 0.1 Ω shunt on
the pump):** fire at the lowest intended power with a short 200 µs pulse. If every
half-cycle conducts, a short pulse is sufficient and you may prefer it. If half-cycles
drop, extend until they stop. Record the result here. `[VERIFY]`

### 6.5 Half-period tracking

A **low-priority** ISR (or a separate low-priority timer) timestamps each ZC and maintains
a median-filtered half-period. Not in the firing path, freely preemptible.

Provides: 50/60 Hz autodetection, the value for `hold_us`, and a mains-health signal.
If no ZC arrives for >30 ms, force PSM low and raise `Fault::MainsLost`.

### 6.6 Power → delay mapping

Not linear in phase angle. Build a 101-entry `const` lookup table (power 0–100% →
`delay_us`) generated offline from the integral of the half-sine, then bend it
empirically against measured flow.

Clamps:
- Minimum delay ~300 µs after the true zero — never fire inside the ZC pulse window.
- Maximum delay = `half_period - ZC_OFFSET_US - MIN_CONDUCTION_US`.
- **Practical floor:** a vibratory pump stalls and buzzes below roughly 25–30%. The
  usable range is narrower than 0–100% and is machine-specific. Determine and encode
  `MIN_USEFUL_POWER`. `[VERIFY]`

Slew-rate limit the setpoint in the control task, not in the ISR.

### 6.7 Disarmed state

Define an explicit disarmed representation (e.g. `PHASE_US = u32::MAX`) that the ZC
handler checks first and returns immediately without arming anything. Default at boot.

---

## 7. Link layer

### 7.1 Shared message model `[DECIDED]`

**Abstract at the message level, never at the byte level.** The ESP32-S3's TWAI is
classic CAN 2.0 only — **no CAN-FD, 8 bytes per frame maximum**. If bytes are the
abstraction, either CAN fragments or the WebSocket is crippled to 8-byte messages.

```rust
// src/link/mod.rs

pub enum Command {
    Start { target_weight_dg: u16 },   // decigrams
    Stop,
    Tare,
    SelectProfile(u8),
    EnableWifi { minutes: u8 },
    DisableWifi,
    // debug-only, see 7.2
    SetPumpPower(u8),
}

pub enum Telemetry {
    Temp { c_x10: i16 },
    Pressure { bar_x100: u16 },
    Weight { dg: i16 },
    PumpPower { pct: u8 },
    ShotState(ShotState),
    Fault(FaultCode),
    WifiUp { ip: [u8; 4] },
    Diag(DiagSnapshot),               // ZC period, rejected edges, missed cycles
}

pub trait Link {
    async fn recv(&mut self) -> Option<Command>;
    async fn send(&mut self, t: Telemetry) -> Result<(), LinkError>;
}
```

Each transport implements `Link` and encodes as it likes. Nothing above the link layer
knows which transport is active.

**Commands must be idempotent and absolute** — `SetPumpPower(62)`, never "increase by 5".
A duplicated CAN frame or a TCP retransmit burst then cannot accumulate error.

### 7.2 Debug command gating `[DECIDED]`

`SetPumpPower` and similar direct actuator commands are accepted **only**:
- over the WebSocket, and
- when no shot is in progress, and
- behind a compile-time `debug-commands` feature that is off in production.

The screen is a display and a button, not a controller. Nothing on the wire ever gets to
bypass shot logic or safety limits.

### 7.3 CAN encoding

- 500 kbit/s, standard 11-bit IDs, `esp_hal::twai`.
- **One ID per telemetry channel**, fixed layout, no serialization framework. The UI node
  configures hardware acceptance filters and gets per-channel filtering free.
- Suggested map (adjust as needed):

| ID | Direction | Payload |
|---|---|---|
| `0x100` | → UI | Temp, i16 LE |
| `0x101` | → UI | Pressure, u16 LE |
| `0x102` | → UI | Weight, i16 LE |
| `0x103` | → UI | PumpPower u8 + ShotState u8 |
| `0x10F` | → UI | Fault code u8 |
| `0x110` | → UI | WifiUp, 4 bytes IP |
| `0x200` | ← UI | Command opcode u8 + up to 7 bytes args |

- Lower ID = higher priority on the bus. Faults deliberately sit above commands? No —
  note that CAN priority only matters under contention, which will not occur on a
  two-node bus at this rate. Keep the map readable rather than clever.
- Telemetry rate: ~20 Hz × 5 channels = 100 frames/s. Trivially within budget.

### 7.4 WebSocket encoding

- JSON via `serde-json-core` (`heapless::String` output, no alloc), or postcard if you
  want compactness. JSON is preferred for debugging — you will be reading it in a browser
  console.
- **Batch** telemetry: one JSON object per tick containing all channels, ~20 Hz. Do not
  send one message per sensor.
- Commands: `{"cmd":"start","target_g":36.0}` etc.

---

## 8. Core split

```
Core 1 (APP) — bare metal, no executor, no allocator, no critical sections
  GPIO ZC ISR   (#[ram], Priority3)  → arm alarm A
  TIMG1 alarm A (#[ram], Priority3)  → PSM high, arm alarm B
  TIMG1 alarm B (#[ram], Priority3)  → PSM low
  reads:  static PHASE_US:  AtomicU32
  writes: static HALF_PERIOD_US: AtomicU32, static MISSED_CYCLES: AtomicU32
  main:   loop { wfi }

Core 0 (PRO) — embassy executor (+ esp-rtos when radio is up)
  task: control_loop   50-100 Hz — profile PID → power → LUT → PHASE_US.store()
  task: sensors        I²C / UART acquisition
  task: relays         I²C
  task: link_rx        Command handling
  task: link_tx        Telemetry at ~20 Hz
  task: shot_logic     start/stop/weight target/safety timers
  task: net            (radio builds only) picoserve + DHCP
```

**Implementation gotchas:**

- Interrupts are bound **per core**. `enable_*_interrupt()` must be called from *inside*
  the core-1 closure passed to `CpuControl::start_app_core`, not before.
- GPIO on the ESP32-S3 has separate `PCPU_INT` / `APP_CPU_INT` status registers. Confirm
  esp-hal routes the ZC interrupt to core 1 and not core 0. Test explicitly with a scope
  or a toggled GPIO — do not assume. `[VERIFY]`
- Cross-core state is `portable_atomic::AtomicU32` only. Use `Relaxed` ordering; there is
  no data dependency requiring anything stronger.
- If a command queue between cores becomes necessary, use `heapless::spsc` — lock-free,
  no CS.

---

## 9. WebSocket and OTA endpoints

Only present in radio builds. Served by `picoserve` over `embassy-net`.

### 9.1 Server setup

- Fixed socket pool, one embassy task per socket, statically allocated. **Reject beyond
  the pool** — no dynamic connection growth.
- Budget ~4 KB RX + 4 KB TX smoltcp buffers plus ~2 KB picoserve request buffer per
  connection. 2–4 concurrent connections is unremarkable on 512 KB SRAM.
- `esp-alloc` heap: start at 64 KB internal + reclaimed region, tune down later.

### 9.2 Routes

| Route | Method | Purpose |
|---|---|---|
| `/` | GET | Tiny static HTML debug page (embedded with `include_str!`) |
| `/ws` | GET (upgrade) | Bidirectional telemetry + debug commands |
| `/api/status` | GET | One-shot JSON snapshot, for `curl` |
| `/api/ota` | POST | Firmware upload (feature `ota`) |

### 9.3 WebSocket

`picoserve` provides a `WebSocketUpgrade` extractor and handles the HTTP/1.1 upgrade,
the `Sec-WebSocket-Accept` SHA-1, frame parsing, client-side unmasking, and ping/pong.
`SocketRx::next_frame` / `next_message` for reads.

Behaviour:
- On connect, send a full state snapshot, then stream batched telemetry at ~20 Hz.
- Application-level ping/pong with a timeout. TCP keepalive alone will not detect a
  half-open connection after a WiFi drop fast enough.
- Dropping the connection **must not** affect the shot. The controller is autonomous.
- Backpressure: if the socket write would block, **drop telemetry frames rather than
  stalling the control task**. Telemetry is disposable; the control loop is not.

Alternative if picoserve proves heavy: `edge-ws` gives framing over any
`embedded-io-async` socket with no HTTP router, or hand-roll it (the handshake is ~30
lines and unmasking is an XOR).

**No TLS.** `embedded-tls` on top of this is painful and browsers will not accept
`wss://` without a real certificate. Trusted LAN only — anyone on the network can drive
the triac. This is a further reason the debug commands are compile-time gated.

### 9.4 OTA `[OPEN]`

Concept:

1. `POST /api/ota` with the raw `.bin` body, streamed — **never buffered in RAM**.
2. Write to the inactive OTA partition in chunks via `esp-storage` /
   `embedded-storage` traits.
3. Verify: length, a magic/version header, and a CRC32 or SHA-256 over the image
   before committing anything.
4. Update `otadata` to point at the new slot, then reboot.
5. Rollback: mark the new image "pending verify"; if it does not confirm healthy within
   N seconds of boot, the bootloader reverts.

Open questions to resolve before implementing:
- `no_std` OTA support is much less mature than the ESP-IDF path. Investigate
  `esp-hal-ota` and similar community crates; assess maintenance status. `[VERIFY]`
- Partition table must reserve two app slots plus `otadata`. Confirm 8 MB is enough for
  two app slots at your image size — it will be, comfortably.
- **Flash erase/write disables the cache.** This is precisely why the dimmer handlers are
  `#[ram]`. Even so: **refuse OTA while a shot is in progress**, and force the pump
  disarmed before the first erase.

Treat OTA as a later milestone. Get a working WebSocket first.

---

## 10. Safety

All of these are **local** and do not depend on link liveness. The controller is
autonomous by design; connection-status watchdogs are deliberately not part of the model.

### Shot logic limits
- **No weight increase after 15 s from pump start** → stop, `Fault::NoFlow`.
- **Absolute pump runtime cap 60 s** → stop, `Fault::MaxRuntime`.
- Target weight reached → stop (with a lead-time offset for drip; calibrate).

### Actuator safety
- Boot state: PSM low, pump disarmed, relays open. Hardware pull-down on PSM.
- `Fault::MainsLost` if no ZC for >30 ms → disarm.
- Over-pressure and over-temperature cutoffs evaluated in the control task at loop rate.
- Sensor plausibility: NaN, out-of-range, or stale (no update in N ms) → disarm and fault.
- RWDT (hardware watchdog) armed; the control task feeds it. A hung control task must
  result in a reset, and reset must be safe.

### Fault behaviour
Faults latch. Clearing requires an explicit `Command` or a power cycle — never an
automatic retry on the pump.

---

## 11. Suggested build order

1. **Blink + serial.** Confirm toolchain, flashing, `defmt`/`esp-println` output.
2. **ZC scope work.** Interrupt on ZC, toggle a spare GPIO, measure pulse width and edge
   offset. Fill in `ZC_OFFSET_US`. Confirm the ISR runs on core 1.
3. **Dimmer open-loop.** Priority-3 two-shot ISR, `PHASE_US` set over serial. Scope the
   PSM pin and the pump current. **Do the pulse-duration calibration (§6.4) here.**
4. **Core split.** Move the dimmer to core 1, put an embassy executor on core 0, confirm
   with a stress task on core 0 that ISR jitter does not move.
5. **Sensors + relays** over I²C. Decide whether UART is needed at all.
6. **Control loop.** Profile → power → LUT → `PHASE_US`, closed on pressure.
7. **Link abstraction + WiFi.** `link-wifi` build, WebSocket, debug page. Work from the
   Mac from here on.
8. **Jitter stress test.** WiFi hammering, flash writes, and confirm the dimmer is
   unaffected. This is the go/no-go for the whole architecture.
9. **Shot logic + safety.** Weight targeting, all cutoffs, fault latching.
10. **CAN transport.** Same `Link` trait, `link-can` build. Verify against the UI node.
11. **`wifi-on-demand`.** Prototype the late `esp-rtos` start question (§5.3) — do this
    earlier if it looks risky.
12. **OTA.**

---

## 12. Open questions

| # | Question | Section |
|---|---|---|
| 1 | Can `esp-rtos` be started late, after minutes of plain embassy operation? | §5.3 |
| 2 | Actual GPIO numbers on the AtomS3R header and Port.A | §2 |
| 3 | Does the on-board IMU share the Port.A I²C bus? | §2 |
| 4 | Measured ZC pulse width and edge offset on this specific module | §2, §6 |
| 5 | Does a short (200 µs) gate pulse latch reliably on the OLAB at low power? | §6.4 |
| 6 | Minimum useful pump power before stall | §6.6 |
| 7 | Does esp-hal route the GPIO interrupt to the core that enabled it? | §8 |
| 8 | Maturity of `no_std` OTA crates | §9.4 |
| 9 | Are any sensors UART-only, or can everything go on I²C? | §2 |

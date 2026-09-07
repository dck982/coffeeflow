# Firmware

Two ESP32-S3 boards run the machine. The Waveshare 4.3" lives on the front and is the only node that talks to the outside world (touch UI, Wi-Fi, BLE). The XIAO lives in `boitier_dc`, in the hot technical compartment, and is the only node that touches sensors and 230 V actuators. They meet on a one-segment CAN bus that already has 120 Ω termination at both ends.

This note is the firmware plan: what each module does, which peripherals it owns, the brew features the screen will eventually run, and the bootstrap that lets both boards move into their boxes and be flashed without USB. Language, CAN encoding, and the OTA layout are left as follow-ups at the end — they are the three decisions to settle before writing code.

Placeholder trees: `firmware/screen/` and `firmware/sensors/`. A shared library can sit next to them later if both firmwares speak the same protocol from the same source.

## Split of work

The screen is the brain. It owns the brew algorithm, the UI, the Acaia Lunar (BLE GATT client), and any talk to the LAN. It never drives the pump or the valve directly.

The sensors module is the hands. It executes commands (SSR, dimmer), counts the flowmeter, reads pressure and temperature, and publishes telemetry when asked. It has no radio. If the screen disappears, the sensors module fails safe: SSR off, dimmer at 0, streaming off.

That split is also a thermal and electrical one. The XIAO is specified to 85 °C and sits in a compartment that reaches 45–50 °C. The Waveshare stays on the fascia. 230 V never leaves the machine; only the CAN pair comes out through the old brew-button hole.

## Sensors module (XIAO ESP32-S3)

Grove Shield pinout, as wired:

| Port | Device | Bus | GPIO |
| --- | --- | --- | --- |
| R1 | XDB401 pressure/temperature | I2C | SDA 4, SCL 5 |
| L4 | RBDimmer DimmerLink | I2C (same bus) | SDA 4, SCL 5 |
| R2 | Digmesa FHKSC 932-9525-B | pulse, falling edge | 7 |
| R3 | Adafruit CAN Pal (TJA1051T/3) | TWAI | TX 8, RX 9 |
| R4 | M5Stack Unit SSR | GPIO out | 10 |

I2C is shared. The dimmer is at `0x50`, the XDB401 at `0x7F`. The only pull-ups on that bus are the 4.7 kΩ on the XDB401; take the pressure sensor off and the dimmer goes mute. Access must be serialised (a mutex around the bus). Do not add a second pair of pull-ups while the XDB401 is fitted.

### SSR

GPIO 10, HIGH = valve open, LOW = closed. The Unit SSR is zero-crossing (MOC3043), so there is no zero-cross ISR to write and no timing to get right beyond “the pin is high or it isn’t”. Default and fail-safe is LOW. The 5 V rail for the module comes from the Wago, not from the Grove port.

### Dimmer

I2C DimmerLink, not UART. The dimmer’s own Cortex does zero-cross and triac timing; the XIAO only writes registers. Bench code already talks to it (`tests/test_rbi2c.py`):

- `0x10` level, 0–100 % — the command that matters every shot
- `0x00` status, `0x02` error, `0x11` curve, `0x20` mains frequency — worth exposing so the screen can show “dimmer not ready” instead of silently doing nothing

Without mains on the dimmer the module sits in `Calibrating...` and rejects writes. Command `0x03` switches the dimmer to UART **and stores that in its EEPROM**; do not call it from firmware. The pump stalls below some level — that number is a calibration, not a guess in source.

### Flowmeter

Digmesa 932-9525-B, 1.00 mm nozzle, **2382 impulses per litre** (0.42 g per pulse), open-collector NPN. An RC on the shield (1 kΩ to 3.3 V, 10 nF to GND) makes a 3.3 V falling edge on GPIO 7; the internal pull-up stays off. An ISR increments a counter. A task turns that counter into:

- **volume** — millilitres since last reset (`count × 1000 / 2382`)
- **flow** — ml/s over a short window (a few hundred milliseconds). Instantaneous Hz is coarse: a typical 36 g / 28 s shot is about 3.1 pulses per second.

A CAN command from the screen resets the accumulator at the start of a brew. The sensor is upstream of the pump and only rated 3 bar, so it measures pump inlet, not group pressure. Pre-infusion at ~0.5 g/s sits at the bottom of the linear range; volume is more trustworthy than instantaneous flow there (see `docs/debitmetres.md`).

### XDB401

Same I2C bus. Trigger a conversion (`0x30` / `0x0A`), wait ~50 ms, read 5 bytes from `0x06`: 24-bit pressure, 16-bit temperature. Full scale is a property of the part (the bench script assumes 10 bar — confirm against the unit on the machine). This is group-side pressure, upstream of the solenoid.

## Screen module (Waveshare ESP32-S3-Touch-LCD-4.3)

800 × 480 RGB panel, GT911 touch, 16 MB flash, 8 MB PSRAM, TJA1051T/3 on board. CAN is GPIO 15 TX / 16 RX. `CAN_SEL` is CH422G EXIO5 and **must be held high** or the transceiver is not selected (that line is also USB_SEL, active low). Bring-up notes for the CH422G and GT911 live in `tests/screen/hello_waveshare/`.

The screen does four concurrent jobs, and they do not all matter at the same time:

| Job | When it is hot |
| --- | --- |
| LVGL UI | always, especially during a brew |
| BLE client → Acaia Lunar | during a brew (weight) |
| CAN → sensors | during a brew (status + commands) |
| Wi-Fi / HTTP | configuration, flashing, posting a shot to a local server at the end |

Wi-Fi is idle mid-shot. BLE + CAN + LVGL are not. Pin LVGL (and the brew loop that reads weight and telemetry) on one core; leave Wi-Fi and the BLE stack on the other, which is where Espressif already puts them. CAN is interrupt + queue, drained by whichever task owns the protocol.

Wi-Fi credentials are entered once — either on the touchscreen or through a temporary access point and a landing page — and stored in **NVS** (the ESP32 has no EEPROM). They never appear in source. A shared HTTP secret lives in a header that is not committed (`secrets.h` or equivalent), used as an `Authorization` header. The server is HTTP, not HTTPS; the secret only keeps the LAN from being a toy.

HTTP, first cut:

- `GET` — latest streamed telemetry (pressure, temperature, flow, volume, dimmer, SSR, and whatever the screen itself knows: weight, brew state)
- `POST` — set dimmer level and SSR
- `POST` firmware image, with a destination of **screen** or **sensors**
- optional `POST` to enable a WebSocket; or a compile-time flag. A connected client sees CAN traffic, at least read-only, so a laptop can replace USB serial once the boards are in their boxes. Read-write on that socket is nice if it is cheap.

## Features the screen will run (after bootstrap)

These are not the first firmware. They are why the sensors interface looks the way it does.

- **Flush** — SSR on, dimmer 100 %, for 5 s or while the button is held.
- **Brew by weight** — run until the Acaia hits the target, minus a stop-early offset for the last drops in the group.
- **Brew by time** — fallback when the scale is missing.
- **Pre-infusion** — low pressure into the headspace (pressure sensor + flowmeter to know when it is full), pause on the puck, then ramp the pump toward a target that may be less than 100 % dimmer.
- **Flow control** — if flow or pressure collapses (channeling), back the pump off.

Plenty of calibration sits under this: flowmeter K-factor at low flow, dimmer-to-pressure map, stall floor of the vibratory pump, stop-early grams. Keep those in NVS, not compiled in.

## Bootstrap — get off USB first

The boards will live in printed boxes, with only CAN and 5 V between them. USB is a workshop luxury. The first firmwares therefore exist to make USB unnecessary:

1. **Sensors** — CAN only, plus the flash/reset/ping subset of the protocol. Actuators stay safe (SSR low, dimmer 0) even if a truncated image is parsed. No brew logic.
2. **Screen** — Wi-Fi provisioning (AP + landing page is enough; on-screen SSID entry can wait for LVGL), the same CAN protocol, HTTP so a laptop can push a `.bin`, local OTA for itself, and a CAN-tunnelled OTA for the sensors. A WebSocket CAN tap so debug does not need a cable.

Once that works, both boards go into their housings. Every later feature — LVGL, BLE, dimmer, flowmeter, brew — arrives as an OTA image.

## Safety, even in the bootstrap

A lost screen, a wedged LVGL task, or a CAN wire that falls off must not leave the solenoid open and the pump at 100 %. The sensors module treats “no pong from the screen” as “everything off, stop streaming”. A command watchdog is the same idea at shorter scale: if streaming is on and status has been flowing, a gap longer than a second or two still kills the actuators. The SSR defaults to off at boot. None of this waits for the brew algorithm; it belongs in the first sensors image that is allowed to drive GPIO 10.

---

# Follow-up 1 — C++ vs Rust, and which framework

**Recommendation: C++ on ESP-IDF 5.x for both boards, with a shared protocol library. FreeRTOS tasks, not Arduino `loop()`.**

The temptation of Rust + `esp-hal` / Embassy on the sensors module is real and well aimed. That board is exactly the kind of firmware Rust is good at: GPIO, I2C, an ISR, TWAI, no Wi-Fi, no GUI. Embassy’s async model matches “a CAN command can arrive at any moment” without a nest of queues. The XIAO ESP32-S3 is a supported target.

The screen is the opposite. The Waveshare 4.3" is a board-support problem: CH422G expander, GT911 on the same I2C as the expander (the Arduino `bb_captouch` library already mis-identifies the CH422G — see the hello-world sketch), RGB LCD via `esp_lcd`, LVGL, BLE GATT client for a device whose sample code you already have, `esp_http_server`, WebSocket, NVS, dual-core pinout. That stack is documented in C/C++. `lvgl-rs` lags, there is no maintained BSP for this Waveshare in Embassy, and a BLE GATT *client* on ESP32 in Rust is still the sharp edge of `esp-idf-svc`. Dual-language also means two copies of the CAN codec, or a codegen story, for a bus that will carry firmware images.

So: one language, because the protocol and the OTA framing must be identical, and because the screen is not optional. ESP-IDF rather than Arduino-as-application, because we need a custom partition table, `esp_ota_ops` rollback, the TWAI driver, `httpd` with WebSocket, and the ability to pin LVGL to core 1. Arduino-ESP32 3.x can still be pulled in as an IDF component if a library demands it; the Waveshare hello-world can stay as a bring-up reference, not as the architecture.

Rust on the sensors remains a legitimate later rewrite, once the factory image (CAN + OTA) is boring and proven in C++. That factory image should stay on the well-trodden stack — it is the one you cannot USB-recover when it is wrong.

Concrete shape:

```
firmware/
  common/     CAN codec, message types, CRC (built by both)
  sensors/    ESP-IDF project, XIAO ESP32-S3
  screen/     ESP-IDF project, ESP32-S3-WROOM-1-N16R8
```

On the sensors side, tasks: TWAI RX, I2C poll (pressure + dimmer status), flowmeter (ISR → counter, task → flow/volume), stream publisher, presence/fail-safe. On the screen: LVGL + brew on core 1; HTTP and the NimBLE client on core 0; a CAN task in the middle.

---

# Follow-up 2 — CAN protocol

**Recommendation: 500 kbit/s, 11-bit IDs for dest/src/priority, line-oriented ASCII with a 7-byte fragmentation header, and a separate binary ID for flash payloads.**

ESP32 TWAI is classic CAN 2.0, 8 bytes, no CAN FD. An ASCII protocol that people can read on a WebSocket therefore cannot be “one text line = one frame” except for the shortest commands. It can still *look* like ASCII once reassembled, which is what you actually want from a Mac sniffer.

### Identity

One letter in the text, a nibble in the ID:

| Letter | Nibble | Node |
| --- | --- | --- |
| `*` | 0 | broadcast |
| `S` | 1 | screen |
| `X` | 2 | sensors (xiao) |

The ID nibble is what the TWAI acceptance filter uses. The letter is what the WebSocket prints. They must match.

11-bit ID, lower number = higher bus priority:

```
bits 10–8  priority   0 = command / reset / ping
                      1 = flash control
                      2 = status stream
                      3 = log
bits  7–4  destination
bits  3–0  source
```

A `CMD` that turns the SSR off then wins against a flood of `STATUS` frames. That matters.

### Framing

Byte 0 of each data frame: `more` flag in bit 7, sequence 0–127 in bits 6–0. Bytes 1–7: UTF-8 text (ASCII in practice). Sequence 0 starts a message. Reassemble until `more` is clear. A complete message is one line, no embedded newline, spaces as separators.

Ping and pong should fit in a **single** frame so they do not depend on the reassembly state:

```
* PING
S PONG screen 0.1.0
X PONG sensors 0.1.0
```

Everything else can span frames.

### Messages

```
* PING
S PONG screen <version>
X PONG sensors <version>

* LOG <from> <text>              boot banner, errors, flash progress
X CMD ssr <0|1>
X CMD dim <0-100>
X CMD volume reset
X RESET
X STREAM <hz>                    0 = stop; default at boot is 0
X STATUS p=<bar> t=<c> f=<ml/s> v=<ml> d=<0-100> s=<0|1>

X FLASH begin <size> <crc32>
X FLASH data                     (binary frames, see below)
X FLASH end
X FLASH abort
X BOOT factory                   optional, jump back to the recovery slot
```

`LOG` is the general text message. A node emits one at boot with name and version; it also carries flash progress (`LOG sensors flash 40%`). The screen mirrors `LOG` and every raw frame onto the WebSocket.

`STREAM <hz>` is a request from the screen. The sensors module starts publishing `STATUS` at that rate and stops if presence is lost or if it reboots. It never streams by default.

Commands are idempotent and last-wins. The screen may send `CMD dim` at 10 Hz during a ramp; the sensors module does not queue a backlog of dimmer writes, it applies the latest.

### Presence (ping / pong)

Every node keeps `last_presence`. A **pong received**, or a **ping received**, both count as presence — the peer is alive. If nothing has counted for **5 s plus `rand() ∈ [0, 2 s]`**, the node sends `* PING`. Whoever hears a ping answers immediately with its own `PONG`. Seeing that pong (or any pong) resets everyone else’s timer, so the bus does not chatter.

The node that *answered* must reset its own timer too: TWAI does not loop its own frames back. Receiving the ping is the signal that the other side is there.

If the sensors module’s presence of the screen expires: stop streaming, SSR low, dimmer 0. A lone board on the bench will ping every 5–7 s forever and never see a pong; that is the correct idle.

### Flash data plane

ASCII hex of a 1 MB image over 7-byte chunks is painful. Give flash payload a dedicated ID (same dest/src, priority 1) whose 8 bytes are raw image content, sequenced in software (offset in an ASCII `FLASH begin`, then a running counter). Control stays ASCII (`FLASH begin|end|abort`, `LOG` progress). Streaming pauses during a flash so the bus is not shared with `STATUS`.

500 kbit/s on a short, terminated pair is conservative; 1 Mbit/s is plausible later. Start at 500.

### Why not fully binary?

A binary codec is smaller and easier to parse. It is worse on the WebSocket, worse to type in a test, and worse when the first debugging session is “is anything on the wire”. ASCII with a thin fragment header is the right trade for two nodes you own. If a third node appears, the ID space still works.

---

# Follow-up 3 — Firmware update

**Recommendation: factory recovery image that is never overwritten, two OTA slots, app-rollback, and CAN used only as a dumb pipe for the sensors `.bin`.**

Both chips have enough flash (XIAO typically 8 MB, Waveshare 16 MB). Partition table, same layout on both, sizes tuned per chip:

```
nvs        data  nvs      24K
otadata    data  ota      8K
phy_init   data  phy      4K
factory    app   factory  recovery image (this bootstrap)
ota_0      app   ota_0    current
ota_1      app   ota_1    next
```

The factory app is the USB-free lifeboat: sensors = CAN + FLASH + ping; screen = Wi-Fi + HTTP + CAN + FLASH. It is not upgraded in place. `X BOOT factory` (and a matching HTTP route on the screen) points `otadata` at it and resets. If an OTA image is broken, that is how you come back without opening the box.

### Local flash (the screen)

`POST /firmware?target=screen` with the `.bin` body, `Authorization` header required. Write the inactive OTA slot with `esp_ota_begin/write/end`, set it bootable, reset. After boot the new image must call `esp_ota_mark_app_valid_cancel_rollback()` **only once CAN ping/pong still works** (and, for the screen, once Wi-Fi or at least the AP still comes up). If it doesn’t, the IDF rollback timer reboots the previous slot. That is the fail-proof path for the board you can no longer reach with USB.

### Remote flash (the sensors, through the screen)

Same HTTP endpoint, `target=sensors`. The screen does not parse the image. It sends `FLASH begin <size> <crc32>`, then binary chunks, then `FLASH end`. The sensors module erases the inactive slot, writes, verifies CRC, sets boot, reports `LOG` progress on the bus (which the WebSocket shows), and resets. After reboot it must pong with the new version; the screen waits for that pong. If it never comes, the sensors rollback kicks in on its own — the screen can also send `BOOT factory` if the recovery image is still the one answering.

During flash, actuators stay off. A `FLASH abort` or a lost presence aborts the write and does not flip `otadata`.

### Why factory + two OTA, not only two OTA

Two OTA slots with rollback cover “the new app crashed”. They do not cover “the new app boots, marks itself valid, then has a CAN bug that cannot flash again”. The factory image never marks itself as anything; it only speaks the bootstrap protocol. Keep it small and do not put LVGL or brew in it.

### First images to build, in order

1. Sensors factory: TWAI up, ping/pong, `LOG` at boot, `RESET`, `FLASH` into `ota_0`, GPIO 10 held low.
2. Screen factory: CH422G `CAN_SEL` high, TWAI up, same protocol, AP + landing page writing SSID/password to NVS, HTTP `GET/POST` with the auth header, local OTA, proxy OTA to sensors, WebSocket CAN tap.
3. Box the boards.
4. OTA the real applications into `ota_0` / `ota_1`.

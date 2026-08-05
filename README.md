# coffeeflow

Flow control for a Profitec Go espresso machine (vibratory pump dimming), built around an M5Stack Atom S3 Voice (Echo).

## Target device: M5Stack Atom S3 Voice (Echo)

- **MCU**: ESP32-S3 (WiFi + BLE, revision v0.2 on the unit used here)
- **USB**: native USB-OTG wired directly to the USB-C port (no external CP210x/CH9102/CH340 bridge chip). It enumerates as a USB-CDC serial device — no driver installation needed on macOS.
- **Audio**: built-in I2S speaker (and mic), driven via `M5.Speaker` (M5Unified library). Supports simple tones (`M5.Speaker.tone()`) and WAV playback (`M5.Speaker.playWav()`).
- **Input**: one main button on top, exposed as `M5.BtnA`. There is also a physical reset button on the side of the unit — press it if the board stops enumerating over USB (see Troubleshooting).
- **Board FQBN**: `m5stack:esp32:m5stack_atoms3` (there is no dedicated FQBN for the Echo/Voice variant; the plain Atom S3 target matches its pinout).

## Flashing on this host

Toolchain already installed on this machine: `arduino-cli`, with the `m5stack:esp32` and `esp32:esp32` cores, and the `M5Unified` library.

1. Connect the board via a USB-C **data** cable (not charge-only).
2. Find the serial port:
   ```
   ls /dev/cu.usbmodem*
   ```
   It typically shows up as `/dev/cu.usbmodem2201` (the numeric suffix can change between reconnects).
3. Compile:
   ```
   arduino-cli compile --fqbn m5stack:esp32:m5stack_atoms3 sound_test
   ```
4. Upload:
   ```
   arduino-cli upload -p /dev/cu.usbmodem2201 --fqbn m5stack:esp32:m5stack_atoms3 sound_test
   ```

### Troubleshooting

- **Board doesn't show up under `/dev/cu.usbmodem*` at all**: try a different USB-C cable (many are charge-only) and a direct port on the Mac (not through a hub). A startup sound confirms power, not data connectivity.
- **Still nothing**: press the reset button on the side of the unit while it's connected, then rescan.
- Port names can change between reconnects/resets — always re-check `ls /dev/cu.usbmodem*` before uploading.

## Sketches

- `sound_test/` — plays a short tone on boot and a TTS-generated WAV (`startup_wav.h`, embedded as a byte array) when the main button is pressed.

## TTS generation (`tts/`)

`tts/generate_tts.py` calls the Gemini TTS API (`google.genai`) to generate a WAV file from text, for embedding into sketches as sound assets.

Requires a `GOOGLE_API_KEY` in `tts/.env` (not committed — see `.gitignore`).

Run with [uv](https://docs.astral.sh/uv/) (inline PEP 723 script dependencies, no separate install step):

```
cd tts
uv run generate_tts.py "Espresso ready" -o startup.wav
```

To embed a generated WAV into a sketch as a C byte array:

```
xxd -i tts/startup.wav | sed 's/tts_startup_wav/startup_wav/; s/unsigned char/const uint8_t/; s/unsigned int/const unsigned int/' > sound_test/startup_wav.h
```

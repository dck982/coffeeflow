#include <M5Unified.h>
#include "startup_wav.h"

void playBeep() {
  M5.Speaker.tone(1000, 200);  // 1kHz for 200ms
}

void playStartupWav() {
  M5.Speaker.playWav(startup_wav, startup_wav_len);
}

void setup() {
  auto cfg = M5.config();
  M5.begin(cfg);

  delay(500);  // let the speaker/I2S driver finish initializing
  playBeep();  // startup sound
}

void loop() {
  M5.update();

  if (M5.BtnA.wasPressed()) {
    playStartupWav();
  }

  delay(10);
}

// Minimal bring-up test for the Waveshare ESP32-S3-Touch-LCD-4.3.
// Exercises flash (this sketch running at all), the 800x480 RGB LCD
// (ST7262 panel, driven directly, no LVGL) and the GT911 touch panel.
// Draws "Hello world" and a REBOOT button; touching the button calls
// ESP.restart().
//
// Wiring per Waveshare's own docs (docs.waveshare.com/ESP32-S3-Touch-LCD-4.3):
//   I2C (touch + CH422G IO expander): SDA=8, SCL=9
//   RGB panel: DE=5 VSYNC=3 HSYNC=46 PCLK=7
//     R0-R4 = 1,2,42,41,40   G0-G5 = 39,0,45,48,47,21   B0-B4 = 14,38,18,17,10
//   Touch IRQ: GPIO4.  Touch reset: CH422G EXIO1.  Backlight: CH422G EXIO2.
//
// The GT911 is talked to directly over Wire, by hand, rather than through
// the bb_captouch library: that library's auto-detect probes the TMA445
// touch chip's I2C address 0x24 *before* GT911's, and 0x24 is also the
// CH422G's "mode register" address on this board, which ACKs. bb_captouch
// therefore always mis-identifies the CH422G as a TMA445 touch chip, sends
// it a TMA445 reset/security-key sequence (clobbering the CH422G's output
// config), and never talks to the real GT911. Skipping the library sidesteps
// the collision entirely.
//
// Library: "GFX Library for Arduino" (moononournation/Arduino_GFX), for the
// LCD only.

#include <Wire.h>
#include <Arduino_GFX_Library.h>

// --- CH422G IO expander (no library: it uses raw I2C "addresses" as
// registers rather than address+register byte pairs) ---
#define CH422G_MODE 0x24 // write 0x01 => EXIO0-7 push-pull output mode
#define CH422G_OUT 0x38  // write => EXIO0-7 output levels
#define EXIO_TP_RST (1 << 1)
#define EXIO_LCD_BL (1 << 2)
#define EXIO_LCD_RST (1 << 3)
#define EXIO_SD_CS (1 << 4)
#define EXIO_USB_SEL (1 << 5)

static void ch422g_write(uint8_t addr, uint8_t val) {
  Wire.beginTransmission(addr);
  Wire.write(val);
  Wire.endTransmission();
}

static void ch422g_bringup() {
  ch422g_write(CH422G_MODE, 0x01);
  // hold touch + LCD reset low, backlight off, SD deselected
  ch422g_write(CH422G_OUT, EXIO_SD_CS | EXIO_USB_SEL);
  delay(20);
  // release resets, keep backlight off until the panel is initialised
  ch422g_write(CH422G_OUT, EXIO_TP_RST | EXIO_LCD_RST | EXIO_SD_CS | EXIO_USB_SEL);
  delay(120); // GT911 wants ~100ms after reset before it responds on I2C
}

static void ch422g_backlight(bool on) {
  uint8_t val = EXIO_TP_RST | EXIO_LCD_RST | EXIO_SD_CS | EXIO_USB_SEL;
  if (on) val |= EXIO_LCD_BL;
  ch422g_write(CH422G_OUT, val);
}

// --- GT911, direct register access (16-bit register address + payload) ---
#define GT911_ADDR1 0x5D
#define GT911_ADDR2 0x14
#define GT911_POINT_INFO 0x814E
#define GT911_POINT_1 0x814F

static bool i2c_probe(uint8_t addr) {
  Wire.beginTransmission(addr);
  return Wire.endTransmission() == 0;
}

static bool gt911_read(uint8_t addr, uint16_t reg, uint8_t *buf, uint8_t len) {
  Wire.beginTransmission(addr);
  Wire.write((uint8_t)(reg >> 8));
  Wire.write((uint8_t)(reg & 0xFF));
  if (Wire.endTransmission(false) != 0) return false;
  uint8_t n = Wire.requestFrom((int)addr, (int)len);
  for (uint8_t i = 0; i < n && i < len; i++) buf[i] = Wire.read();
  return n == len;
}

static void gt911_write_u8(uint8_t addr, uint16_t reg, uint8_t val) {
  Wire.beginTransmission(addr);
  Wire.write((uint8_t)(reg >> 8));
  Wire.write((uint8_t)(reg & 0xFF));
  Wire.write(val);
  Wire.endTransmission();
}

uint8_t gt911_addr = 0; // 0 = not found

static bool gt911_find() {
  if (i2c_probe(GT911_ADDR1)) { gt911_addr = GT911_ADDR1; return true; }
  if (i2c_probe(GT911_ADDR2)) { gt911_addr = GT911_ADDR2; return true; }
  return false;
}

// Returns true if a finger is down; fills x,y with the first touch point.
static bool gt911_poll(int16_t *x, int16_t *y, uint8_t *raw_status) {
  uint8_t status = 0;
  if (!gt911_read(gt911_addr, GT911_POINT_INFO, &status, 1)) return false;
  *raw_status = status;
  if (!(status & 0x80)) return false; // buffer not ready
  uint8_t n = status & 0x0F;
  bool got = false;
  if (n >= 1 && n <= 5) {
    uint8_t pt[7];
    if (gt911_read(gt911_addr, GT911_POINT_1, pt, 7)) {
      *x = pt[1] | (pt[2] << 8);
      *y = pt[3] | (pt[4] << 8);
      got = true;
    }
  }
  gt911_write_u8(gt911_addr, GT911_POINT_INFO, 0); // clear, ready for next sample
  return got;
}

Arduino_ESP32RGBPanel *rgbpanel = new Arduino_ESP32RGBPanel(
    5 /* DE */, 3 /* VSYNC */, 46 /* HSYNC */, 7 /* PCLK */,
    1 /* R0 */, 2 /* R1 */, 42 /* R2 */, 41 /* R3 */, 40 /* R4 */,
    39 /* G0 */, 0 /* G1 */, 45 /* G2 */, 48 /* G3 */, 47 /* G4 */, 21 /* G5 */,
    14 /* B0 */, 38 /* B1 */, 18 /* B2 */, 17 /* B3 */, 10 /* B4 */,
    0 /* hsync_polarity */, 40 /* hsync_front_porch */, 48 /* hsync_pulse_width */, 88 /* hsync_back_porch */,
    0 /* vsync_polarity */, 13 /* vsync_front_porch */, 3 /* vsync_pulse_width */, 32 /* vsync_back_porch */,
    1 /* pclk_active_neg */, 16000000 /* prefer_speed */, false /* useBigEndian */,
    0 /* de_idle_high */, 0 /* pclk_idle_high */, 0 /* bounce_buffer_size_px */);

Arduino_RGB_Display *gfx = new Arduino_RGB_Display(
    800 /* width */, 480 /* height */, rgbpanel, 0 /* rotation */, true /* auto_flush */);

bool touch_ok = false;

// REBOOT button geometry
const int16_t btn_x = 300, btn_y = 340, btn_w = 200, btn_h = 80;

static void draw_ui(const char *status) {
  gfx->fillScreen(RGB565_BLACK);
  gfx->setTextColor(RGB565_WHITE);
  gfx->setTextSize(4);
  gfx->setCursor(260, 160);
  gfx->print("Hello world");

  gfx->setTextSize(1);
  gfx->setCursor(20, 20);
  gfx->print(status);

  gfx->fillRoundRect(btn_x, btn_y, btn_w, btn_h, 12, RGB565_DARKGREY);
  gfx->drawRoundRect(btn_x, btn_y, btn_w, btn_h, 12, RGB565_WHITE);
  gfx->setTextColor(RGB565_WHITE);
  gfx->setTextSize(3);
  gfx->setCursor(btn_x + 25, btn_y + 28);
  gfx->print("REBOOT");
}

void setup() {
  Serial.begin(115200);
  Wire.begin(8, 9);
  Wire.setClock(400000);

  ch422g_bringup();

  if (!gfx->begin()) {
    Serial.println("gfx->begin() failed");
  }
  ch422g_backlight(true);

  touch_ok = gt911_find();

  char status[48];
  if (touch_ok) {
    snprintf(status, sizeof(status), "touch: GT911 @ 0x%02X", gt911_addr);
  } else {
    snprintf(status, sizeof(status), "touch: GT911 NOT found");
  }
  draw_ui(status);
}

void loop() {
  if (touch_ok) {
    int16_t x = 0, y = 0;
    uint8_t raw_status = 0;
    bool down = gt911_poll(&x, &y, &raw_status);

    // Debug line at the top: overwrite it in place so the rest of the UI
    // (including the button) is left alone.
    gfx->fillRect(0, 0, 799, 16, RGB565_BLACK);
    gfx->setTextColor(RGB565_YELLOW);
    gfx->setTextSize(1);
    gfx->setCursor(20, 4);
    if (down) {
      gfx->printf("touch: status=0x%02X x=%d y=%d", raw_status, x, y);
    } else {
      gfx->printf("touch: status=0x%02X (no press)", raw_status);
    }

    if (down) {
      // mark every raw touch point so a coordinate/axis mismatch with the
      // button rectangle is visible even when the button itself never fires
      gfx->fillCircle(x, y, 4, RGB565_RED);

      if (x >= btn_x && x <= btn_x + btn_w && y >= btn_y && y <= btn_y + btn_h) {
        gfx->fillScreen(RGB565_BLACK);
        gfx->setTextColor(RGB565_WHITE);
        gfx->setTextSize(3);
        gfx->setCursor(200, 220);
        gfx->print("Rebooting...");
        delay(400);
        ESP.restart();
      }
    }
  }
  delay(20);
}

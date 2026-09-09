// Bring-up LCD jetable (Waveshare ESP32-S3-Touch-LCD-4.3) — docs/plan-phase6.md,
// lot 2, section "Piège".
//
// But unique : vérifier si les timings HSYNC/VSYNC de
// tests/screen/hello_waveshare/hello_waveshare.ino (Arduino_GFX direct, sans
// LVGL, confirmé sans glitch sur ce même banc) corrigent le glitch horizontal
// observé dans firmware/screen/main/service_screen.cpp, qui utilise lui les
// timings génériques de l'exemple ESP-IDF officiel Waveshare
// (tmp/ESP32-S3-Touch-LCD-4.3/examples/ESP-IDF/09_lvgl_v9_demo/).
//
// Rien d'autre : pas de CAN, pas de pont série, pas de cœur métier. Le
// bring-up CH422G et la séquence de reset tactile/LCD reprennent
// hello_waveshare.ino, portés en C++ ESP-IDF natif (i2c_master de
// esp_driver_i2c) plutôt qu'Arduino Wire. Les structs et noms de champs
// esp_lcd/esp_lcd_touch_gt911/esp_lvgl_port sont alignés sur
// firmware/screen/main/service_screen.cpp et board.cpp (déjà prouvés
// fonctionner en IDF v6.1), seuls les timings du tableau ci-dessous
// diffèrent.

#include <cstdio>

#include "driver/gpio.h"
#include "driver/i2c_master.h"
#include "esp_err.h"
#include "esp_lcd_panel_io.h"
#include "esp_lcd_panel_ops.h"
#include "esp_lcd_panel_rgb.h"
#include "esp_lcd_touch.h"
#include "esp_lcd_touch_gt911.h"
#include "esp_log.h"
#include "esp_lvgl_port.h"
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"

namespace {

constexpr const char* kTag = "screen-lcd-test";

// --- I2C partagé CH422G / GT911 (SDA=8, SCL=9), voir board.h/board.cpp de
// firmware/screen et hello_waveshare.ino. ---
constexpr gpio_num_t kI2cSda = GPIO_NUM_8;
constexpr gpio_num_t kI2cScl = GPIO_NUM_9;

constexpr uint16_t kCh422gModeAddr = 0x24;
constexpr uint16_t kCh422gOutAddr = 0x38;
constexpr uint8_t kCh422gModeOutputs = 0x01;  // EXIO0-7 en push-pull

enum Ch422gBit : uint8_t {
  kCh422gTpRst = 1 << 1,
  kCh422gLcdBl = 1 << 2,
  kCh422gLcdRst = 1 << 3,
  kCh422gSdCs = 1 << 4,
  kCh422gUsbSel = 1 << 5,  // CAN_SEL/USB_SEL — laissé bas, pas de CAN sur ce banc de test
};

i2c_master_bus_handle_t g_i2c_bus = nullptr;
i2c_master_dev_handle_t g_ch422g_mode_dev = nullptr;
i2c_master_dev_handle_t g_ch422g_out_dev = nullptr;
uint8_t g_ch422g_out_mirror = 0;

void ch422g_write_out(uint8_t val) {
  g_ch422g_out_mirror = val;
  ESP_ERROR_CHECK(i2c_master_transmit(g_ch422g_out_dev, &g_ch422g_out_mirror, 1, -1));
}

// Séquence reprise telle quelle de hello_waveshare.ino : maintien reset
// tactile/LCD bas, SD désélectionné, backlight éteinte jusqu'à l'init du
// panneau (ch422g_backlight()), puis reset impulsionnel du GT911 (~100 ms).
void ch422g_bringup() {
  i2c_master_bus_config_t bus_config{};
  bus_config.i2c_port = -1;
  bus_config.sda_io_num = kI2cSda;
  bus_config.scl_io_num = kI2cScl;
  bus_config.clk_source = I2C_CLK_SRC_DEFAULT;
  bus_config.glitch_ignore_cnt = 7;
  bus_config.flags.enable_internal_pullup = true;
  ESP_ERROR_CHECK(i2c_new_master_bus(&bus_config, &g_i2c_bus));

  i2c_device_config_t mode_dev_cfg{};
  mode_dev_cfg.dev_addr_length = I2C_ADDR_BIT_LEN_7;
  mode_dev_cfg.device_address = kCh422gModeAddr;
  mode_dev_cfg.scl_speed_hz = 400000;
  ESP_ERROR_CHECK(i2c_master_bus_add_device(g_i2c_bus, &mode_dev_cfg, &g_ch422g_mode_dev));

  i2c_device_config_t out_dev_cfg{};
  out_dev_cfg.dev_addr_length = I2C_ADDR_BIT_LEN_7;
  out_dev_cfg.device_address = kCh422gOutAddr;
  out_dev_cfg.scl_speed_hz = 400000;
  ESP_ERROR_CHECK(i2c_master_bus_add_device(g_i2c_bus, &out_dev_cfg, &g_ch422g_out_dev));

  uint8_t mode_val = kCh422gModeOutputs;
  ESP_ERROR_CHECK(i2c_master_transmit(g_ch422g_mode_dev, &mode_val, 1, -1));

  // Reset tactile+LCD maintenus bas, SD désélectionné, backlight éteinte.
  ch422g_write_out(kCh422gSdCs | kCh422gUsbSel);
  vTaskDelay(pdMS_TO_TICKS(20));

  // Sortie des resets, backlight toujours éteinte jusqu'à l'init du panneau.
  ch422g_write_out(kCh422gTpRst | kCh422gLcdRst | kCh422gSdCs | kCh422gUsbSel);
  vTaskDelay(pdMS_TO_TICKS(120));  // GT911 : ~100 ms après reset avant de répondre en I2C
}

void ch422g_backlight(bool on) {
  uint8_t val = kCh422gTpRst | kCh422gLcdRst | kCh422gSdCs | kCh422gUsbSel;
  if (on) {
    val |= kCh422gLcdBl;
  }
  ch422g_write_out(val);
}

// --- Panneau RGB 800x480, timings hello_waveshare.ino (confirmés sans
// glitch sur ce banc) : hsync pulse=48/back=88/front=40, vsync
// pulse=3/back=32/front=13, pclk=16 MHz, pclk_active_neg=1. Seuls ces
// timings diffèrent de firmware/screen/main/service_screen.cpp (qui a
// pulse=4/8, back=8/8, front=8/8) ; brochage, format couleur, num_fbs et
// bounce buffer restent alignés sur ce fichier déjà prouvé fonctionner. ---
constexpr uint32_t kLcdHRes = 800;
constexpr uint32_t kLcdVRes = 480;
constexpr uint32_t kLcdPixelClockHz = 16 * 1000 * 1000;

// Bounce buffer : 20 lignes, raisonnable pour ce test isolé (pas de charge
// CPU concurrente CAN/pont série comme dans firmware/screen).
constexpr uint32_t kBounceLines = 20;
constexpr uint32_t kBounceBufferSizePx = kLcdHRes * kBounceLines;

esp_lcd_panel_handle_t init_rgb_panel() {
  esp_lcd_rgb_panel_config_t panel_config{};
  panel_config.clk_src = LCD_CLK_SRC_DEFAULT;
  panel_config.timings.pclk_hz = kLcdPixelClockHz;
  panel_config.timings.h_res = kLcdHRes;
  panel_config.timings.v_res = kLcdVRes;
  panel_config.timings.hsync_pulse_width = 48;
  panel_config.timings.hsync_back_porch = 88;
  panel_config.timings.hsync_front_porch = 40;
  panel_config.timings.vsync_pulse_width = 3;
  panel_config.timings.vsync_back_porch = 32;
  panel_config.timings.vsync_front_porch = 13;
  panel_config.timings.flags.pclk_active_neg = 1;
  panel_config.data_width = 16;
  panel_config.in_color_format = LCD_COLOR_FMT_RGB565;
  panel_config.out_color_format = LCD_COLOR_FMT_RGB565;
  // avoid_tearing (num_fbs=2, bounce_buffer=0, direct_mode) testé et
  // abandonné (2026-09-09) : même en isolation totale (sans CAN/pont série),
  // résultat cassé — d'abord bruit/clignotement, puis (avec direct_mode
  // ajouté) un défilement horizontal permanent de l'image. Cause identifiée
  // dans le code source ESP-IDF (esp_lcd_panel_rgb.c, rgb_panel_draw_bitmap) :
  // le driver documente lui-même que le moment du vrai basculement de
  // framebuffer n'est pas garanti ("it's hard to know the time when the new
  // frame buffer starts", à cause du prefetch DMA) — limite du driver sur
  // cette version d'IDF, pas une erreur de configuration de notre part. Voir
  // docs/firmware-implementation.md pour le détail complet de l'investigation.
  // Retour à bb_mode / bounce buffer, seule config connue stable à ce jour.
  panel_config.num_fbs = 1;
  panel_config.bounce_buffer_size_px = kBounceBufferSizePx;
  panel_config.hsync_gpio_num = GPIO_NUM_46;
  panel_config.vsync_gpio_num = GPIO_NUM_3;
  panel_config.de_gpio_num = GPIO_NUM_5;
  panel_config.pclk_gpio_num = GPIO_NUM_7;
  panel_config.disp_gpio_num = GPIO_NUM_NC;
  panel_config.data_gpio_nums[0] = GPIO_NUM_14;
  panel_config.data_gpio_nums[1] = GPIO_NUM_38;
  panel_config.data_gpio_nums[2] = GPIO_NUM_18;
  panel_config.data_gpio_nums[3] = GPIO_NUM_17;
  panel_config.data_gpio_nums[4] = GPIO_NUM_10;
  panel_config.data_gpio_nums[5] = GPIO_NUM_39;
  panel_config.data_gpio_nums[6] = GPIO_NUM_0;
  panel_config.data_gpio_nums[7] = GPIO_NUM_45;
  panel_config.data_gpio_nums[8] = GPIO_NUM_48;
  panel_config.data_gpio_nums[9] = GPIO_NUM_47;
  panel_config.data_gpio_nums[10] = GPIO_NUM_21;
  panel_config.data_gpio_nums[11] = GPIO_NUM_1;
  panel_config.data_gpio_nums[12] = GPIO_NUM_2;
  panel_config.data_gpio_nums[13] = GPIO_NUM_42;
  panel_config.data_gpio_nums[14] = GPIO_NUM_41;
  panel_config.data_gpio_nums[15] = GPIO_NUM_40;
  panel_config.flags.fb_in_psram = 1;

  esp_lcd_panel_handle_t panel_handle = nullptr;
  ESP_ERROR_CHECK(esp_lcd_new_rgb_panel(&panel_config, &panel_handle));
  ESP_ERROR_CHECK(esp_lcd_panel_reset(panel_handle));
  ESP_ERROR_CHECK(esp_lcd_panel_init(panel_handle));
  return panel_handle;
}

// GT911 — même logique de repli que service_screen.cpp::init_touch() :
// tactile non indispensable pour ce test visuel, on continue sans lui en
// cas d'échec (observé possible au tout premier accès I2C post-reset).
esp_lcd_touch_handle_t init_touch() {
  esp_lcd_panel_io_i2c_config_t tp_io_config{};
  tp_io_config.dev_addr = ESP_LCD_TOUCH_IO_I2C_GT911_ADDRESS;
  tp_io_config.scl_speed_hz = 400000;
  tp_io_config.control_phase_bytes = 1;
  tp_io_config.dc_bit_offset = 0;
  tp_io_config.lcd_cmd_bits = 16;
  tp_io_config.flags.disable_control_phase = 1;

  esp_lcd_touch_config_t tp_cfg{};
  tp_cfg.x_max = kLcdHRes;
  tp_cfg.y_max = kLcdVRes;
  tp_cfg.rst_gpio_num = GPIO_NUM_NC;  // TP_RST déjà géré par ch422g_bringup()
  tp_cfg.int_gpio_num = GPIO_NUM_NC;  // pas câblé sur ce banc, lecture par scrutation
  tp_cfg.levels.reset = 0;
  tp_cfg.levels.interrupt = 0;

  constexpr int kAttempts = 3;
  for (int attempt = 0; attempt < kAttempts; ++attempt) {
    esp_lcd_panel_io_handle_t tp_io_handle = nullptr;
    esp_err_t err = esp_lcd_new_panel_io_i2c(g_i2c_bus, &tp_io_config, &tp_io_handle);
    if (err == ESP_OK) {
      esp_lcd_touch_handle_t touch_handle = nullptr;
      err = esp_lcd_touch_new_i2c_gt911(tp_io_handle, &tp_cfg, &touch_handle);
      if (err == ESP_OK) {
        return touch_handle;
      }
    }
    ESP_LOGW(kTag, "GT911 init failed (attempt %d): %s", attempt + 1, esp_err_to_name(err));
    vTaskDelay(pdMS_TO_TICKS(100));
  }
  return nullptr;
}

void init_lvgl_port(esp_lcd_panel_handle_t panel_handle, esp_lcd_touch_handle_t touch_handle) {
  lvgl_port_cfg_t lvgl_cfg = ESP_LVGL_PORT_INIT_CONFIG();
  ESP_ERROR_CHECK(lvgl_port_init(&lvgl_cfg));

  lvgl_port_display_cfg_t disp_cfg{};
  disp_cfg.panel_handle = panel_handle;
  disp_cfg.buffer_size = kLcdHRes * 60;
  disp_cfg.double_buffer = false;
  disp_cfg.hres = kLcdHRes;
  disp_cfg.vres = kLcdVRes;
  disp_cfg.color_format = LV_COLOR_FORMAT_RGB565;
  disp_cfg.flags.buff_spiram = true;

  lvgl_port_display_rgb_cfg_t rgb_cfg{};
  rgb_cfg.flags.bb_mode = 1;  // bounce buffer côté LVGL, en écho au bounce buffer esp_lcd

  lv_display_t* disp = lvgl_port_add_disp_rgb(&disp_cfg, &rgb_cfg);
  ESP_ERROR_CHECK(disp != nullptr ? ESP_OK : ESP_FAIL);

  if (touch_handle != nullptr) {
    lvgl_port_touch_cfg_t touch_cfg{};
    touch_cfg.disp = disp;
    touch_cfg.handle = touch_handle;
    touch_cfg.scale.x = 1.0f;
    touch_cfg.scale.y = 1.0f;
    lv_indev_t* indev = lvgl_port_add_touch(&touch_cfg);
    if (indev == nullptr) {
      ESP_LOGW(kTag, "lvgl_port_add_touch failed");
    }
  }
}

lv_obj_t* g_counter_label = nullptr;

void counter_timer_cb(lv_timer_t* /*timer*/) {
  static uint32_t tick = 0;
  char buf[16];
  std::snprintf(buf, sizeof(buf), "%lu", static_cast<unsigned long>(tick++));
  lv_label_set_text(g_counter_label, buf);
}

// UI minimale : "Hello world" statique + un compteur qui change toutes les
// 200 ms (seule condition qui reproduit le glitch résiduel observé dans
// firmware/screen/main/service_screen.cpp — un contenu figé ne suffit pas).
// Rien d'autre : pas de flux d'événements, pas de cœur, pas de CAN.
void build_ui() {
  lv_obj_t* scr = lv_screen_active();
  lv_obj_set_style_bg_color(scr, lv_color_black(), 0);

  lv_obj_t* label = lv_label_create(scr);
  lv_obj_set_style_text_color(label, lv_color_white(), 0);
  lv_obj_set_style_text_font(label, &lv_font_montserrat_20, 0);
  lv_label_set_text(label, "Hello world");
  lv_obj_center(label);

  g_counter_label = lv_label_create(scr);
  lv_obj_set_style_text_color(g_counter_label, lv_color_white(), 0);
  lv_obj_set_pos(g_counter_label, 4, 4);
  lv_label_set_text(g_counter_label, "0");

  lv_timer_create(counter_timer_cb, 200, nullptr);
}

}  // namespace

extern "C" void app_main() {
  ESP_LOGI(kTag, "screen-lcd-test: bring-up CH422G");
  ch422g_bringup();

  ESP_LOGI(kTag, "screen-lcd-test: init panneau RGB (timings hello_waveshare.ino)");
  esp_lcd_panel_handle_t panel_handle = init_rgb_panel();

  ESP_LOGI(kTag, "screen-lcd-test: backlight ON");
  ch422g_backlight(true);

  ESP_LOGI(kTag, "screen-lcd-test: init GT911");
  esp_lcd_touch_handle_t touch_handle = init_touch();
  if (touch_handle == nullptr) {
    ESP_LOGW(kTag, "GT911 absent, on continue sans tactile");
  }

  ESP_LOGI(kTag, "screen-lcd-test: init LVGL port");
  init_lvgl_port(panel_handle, touch_handle);

  if (lvgl_port_lock(0)) {
    build_ui();
    lvgl_port_unlock();
  }

  ESP_LOGI(kTag, "screen-lcd-test: pret");
}

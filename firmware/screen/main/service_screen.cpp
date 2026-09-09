#include "service_screen.h"

#include <cstdio>
#include <cstring>

#include "esp_lcd_panel_io.h"
#include "esp_lcd_panel_ops.h"
#include "esp_lcd_panel_rgb.h"
#include "esp_lcd_touch.h"
#include "esp_lcd_touch_gt911.h"
#include "esp_lvgl_port.h"
#include "esp_timer.h"
#include "freertos/FreeRTOS.h"
#include "freertos/semphr.h"
#include "freertos/task.h"

#include "board.h"
#include "can_link.h"
#include "common/version.hpp"
#include "core/core.h"

namespace service_screen {

namespace {

// Résolution et brochage du panneau RGB (Waveshare ESP32-S3-Touch-LCD-4.3),
// confirmés contre tmp/ESP32-S3-Touch-LCD-4.3/examples/ESP-IDF/09_lvgl_v9_demo
// (dépôt officiel, IDF v5.x — adapté ici pour IDF v6.1 et pour piloter le
// CH422G exclusivement via board::panel_power_on(), voir docs/plan-phase6.md).
constexpr uint32_t kLcdHRes = 800;
constexpr uint32_t kLcdVRes = 480;
constexpr uint32_t kLcdPixelClockHz = 16 * 1000 * 1000;

// Tentative avoid_tearing (num_fbs=2, bb_mode=0) essayée puis ABANDONNÉE
// (2026-09-09) : résultat bien pire que le glitch résiduel qu'elle devait
// corriger (écran clignotant blanc/bruit/texte, décalage permanent) —
// vraisemblablement un souci de synchronisation VSYNC entre les deux
// framebuffers et ce panneau/ce driver en v6.1, pas creusé plus loin.
// Retour à bb_mode=1 / num_fbs=1 / bounce buffer, seule config stable connue
// à ce jour. Le glitch résiduel (tearing sur contenu qui change, ~10 % de
// l'intensité du glitch horizontal déjà corrigé par les timings HSYNC/VSYNC
// ci-dessous) reste non résolu — à reprendre plus tard si besoin, pas par
// avoid_tearing tel quel.
constexpr uint32_t kBounceLines = 40;
constexpr uint32_t kBounceBufferSizePx = kLcdHRes * kBounceLines;

// Nombre d'événements affichés, les plus récents en haut.
constexpr size_t kVisibleEvents = 5;

// Combien de temps les coordonnées tactiles restent affichées après un
// appui (docs/plan-phase6.md, lot 2, point 5).
constexpr uint32_t kTouchLabelHoldMs = 1000;

// Période de rafraîchissement de l'écran de service — largement suffisante
// pour une console de debug, et pour détecter la perte de présence CAN en
// moins de 3 s (le vrai critère de sortie du lot).
constexpr uint32_t kRefreshPeriodMs = 200;

lv_obj_t* g_version_label = nullptr;
lv_obj_t* g_can_label = nullptr;
lv_obj_t* g_net_label = nullptr;
lv_obj_t* g_ip_label = nullptr;
lv_obj_t* g_event_labels[kVisibleEvents] = {};
lv_obj_t* g_touch_label = nullptr;

uint32_t g_last_touch_ms = 0;
bool g_touch_label_visible = false;

uint32_t now_ms() { return static_cast<uint32_t>(esp_timer_get_time() / 1000); }

// lv_label_set_text() invalide toujours le widget, même si la chaîne est
// identique. Sur ce panneau RGB, chaque invalidation inutile dispute le bus
// PSRAM à l'ISR DMA (firmware/screen/AGENTS.md).
void set_label_if_changed(lv_obj_t* label, const char* text) {
  const char* current = lv_label_get_text(label);
  if (current != nullptr && std::strcmp(current, text) == 0) {
    return;
  }
  lv_label_set_text(label, text);
}

// esp_lcd RGB + GT911, sans passer par aucune couche BSP Waveshare : celle-ci
// écrit le registre de sortie CH422G en une seule fois (voir
// waveshare_rgb_lcd_port.c, waveshare_rgb_lcd_backlight_on()) et couperait
// CAN_SEL au passage — exactement le piège documenté dans
// docs/plan-phase6.md. board::panel_power_on() (appelé par le code qui
// précède l'appel à init_hardware ici) est la seule chose qui touche ce
// registre.
esp_lcd_panel_handle_t init_rgb_panel() {
  esp_lcd_rgb_panel_config_t panel_config{};
  panel_config.clk_src = LCD_CLK_SRC_DEFAULT;
  panel_config.timings.pclk_hz = kLcdPixelClockHz;
  panel_config.timings.h_res = kLcdHRes;
  panel_config.timings.v_res = kLcdVRes;
  // Timings hello_waveshare.ino, validés sans glitch sur firmware/screen-lcd-test
  // (banc jetable) : la cause du glitch horizontal était ici, pas le bounce
  // buffer (voir commentaire sur kBounceLines ci-dessus).
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

// GT911 observé intermittent au tout premier accès I2C juste après le reset
// impulsionnel (bus qui finit de se stabiliser après la mise sous tension de
// la dalle) : un échec isolé au premier boot, jamais au second, pendant le
// bring-up (docs/plan-phase6.md, "attends-toi à des écarts... prévoir un
// repli"). Le tactile n'est pas indispensable au reste de l'écran de service
// (CAN/version/événements) : on retente une fois puis on continue sans
// tactile plutôt que de faire planter tout l'écran (et donc le pont CAN).
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
  tp_cfg.rst_gpio_num = GPIO_NUM_NC;  // TP_RST déjà géré par board::panel_power_on() (CH422G)
  tp_cfg.int_gpio_num = GPIO_NUM_NC;  // pas câblé sur ce banc, lecture par scrutation
  tp_cfg.levels.reset = 0;
  tp_cfg.levels.interrupt = 0;

  constexpr int kAttempts = 3;
  for (int attempt = 0; attempt < kAttempts; ++attempt) {
    esp_lcd_panel_io_handle_t tp_io_handle = nullptr;
    esp_err_t err = esp_lcd_new_panel_io_i2c(board::i2c_bus(), &tp_io_config, &tp_io_handle);
    if (err == ESP_OK) {
      esp_lcd_touch_handle_t touch_handle = nullptr;
      err = esp_lcd_touch_new_i2c_gt911(tp_io_handle, &tp_cfg, &touch_handle);
      if (err == ESP_OK) {
        return touch_handle;
      }
    }
    can_link::send_log(common::LogCode::kI2cError, common::LogSeverity::kWarn,
                        static_cast<uint16_t>(ESP_LCD_TOUCH_IO_I2C_GT911_ADDRESS));
    vTaskDelay(pdMS_TO_TICKS(100));
  }
  return nullptr;
}

void init_lvgl_port(esp_lcd_panel_handle_t panel_handle, esp_lcd_touch_handle_t touch_handle) {
  lvgl_port_cfg_t lvgl_cfg = ESP_LVGL_PORT_INIT_CONFIG();
  // LVGL et ISR LCD sur le cœur 1 (docs/screen-issue.md) : l'ISR préempte
  // les écritures framebuffer au lieu de les concurrencer depuis le cœur 0.
  lvgl_cfg.task_affinity = 1;
  lvgl_cfg.task_stack = 12 * 1024;
  ESP_ERROR_CHECK(lvgl_port_init(&lvgl_cfg));

  lvgl_port_display_cfg_t disp_cfg{};
  disp_cfg.panel_handle = panel_handle;
  disp_cfg.buffer_size = kLcdHRes * 60;
  disp_cfg.double_buffer = false;
  disp_cfg.hres = kLcdHRes;
  disp_cfg.vres = kLcdVRes;
  disp_cfg.color_format = LV_COLOR_FORMAT_RGB565;
  // Piste 2 (contention bus PSRAM, avis Opus 2026-09-09) testée et
  // invalidée : buffer en RAM interne (buff_spiram=false) n'a rien changé,
  // le glitch résiduel est revenu identique. Retour en PSRAM.
  disp_cfg.flags.buff_spiram = true;

  lvgl_port_display_rgb_cfg_t rgb_cfg{};
  rgb_cfg.flags.bb_mode = 1;  // bounce buffer côté LVGL, en écho au bounce
                              // buffer esp_lcd (voir avoid_tearing, abandonné,
                              // commenté au-dessus de kBounceLines).

  lv_display_t* disp = lvgl_port_add_disp_rgb(&disp_cfg, &rgb_cfg);
  ESP_ERROR_CHECK(disp != nullptr ? ESP_OK : ESP_FAIL);

  // touch_handle peut être nul si le GT911 n'a pas répondu (voir init_touch) :
  // l'écran de service reste utile sans tactile (CAN/version/événements),
  // seul le point 5 du lot (coordonnées à l'appui) est alors indisponible.
  if (touch_handle != nullptr) {
    lvgl_port_touch_cfg_t touch_cfg{};
    touch_cfg.disp = disp;
    touch_cfg.handle = touch_handle;
    touch_cfg.scale.x = 1.0f;
    touch_cfg.scale.y = 1.0f;
    lv_indev_t* indev = lvgl_port_add_touch(&touch_cfg);
    if (indev == nullptr) {
      can_link::send_log(common::LogCode::kI2cError, common::LogSeverity::kWarn, 0);
    }
  }
}

void on_screen_pressed(lv_event_t* e) {
  lv_indev_t* indev = lv_indev_get_act();
  if (indev == nullptr) {
    return;
  }
  lv_point_t point;
  lv_indev_get_point(indev, &point);
  char buf[32];
  std::snprintf(buf, sizeof(buf), "TOUCH x=%d y=%d", static_cast<int>(point.x), static_cast<int>(point.y));
  set_label_if_changed(g_touch_label, buf);
  g_last_touch_ms = now_ms();
  g_touch_label_visible = true;
  (void)e;
}

// Console de debug volontairement laide : pas de style, pas de cote de
// ui.md, police montserrat intégrée à LVGL (docs/plan-phase6.md, lot 2).
void build_ui() {
  lv_obj_t* scr = lv_screen_active();
  lv_obj_set_style_bg_color(scr, lv_color_black(), 0);
  lv_obj_add_flag(scr, LV_OBJ_FLAG_CLICKABLE);
  lv_obj_add_event_cb(scr, on_screen_pressed, LV_EVENT_PRESSED, nullptr);

  static char version_text[48];
  std::snprintf(version_text, sizeof(version_text), "screen v%u.%u.%u", common::kFirmwareVersionMajor,
                common::kFirmwareVersionMinor, common::kFirmwareVersionPatch);

  int y = 4;
  g_version_label = lv_label_create(scr);
  lv_obj_set_style_text_color(g_version_label, lv_color_white(), 0);
  lv_obj_set_style_text_font(g_version_label, &lv_font_montserrat_20, 0);
  lv_label_set_text(g_version_label, version_text);
  lv_obj_set_pos(g_version_label, 4, y);
  y += 26;

  g_can_label = lv_label_create(scr);
  lv_obj_set_style_text_color(g_can_label, lv_color_white(), 0);
  lv_obj_set_pos(g_can_label, 4, y);
  y += 20;

  g_net_label = lv_label_create(scr);
  lv_obj_set_style_text_color(g_net_label, lv_color_white(), 0);
  lv_label_set_text(g_net_label, "RESEAU: absent");  // lot 4 : Wi-Fi
  lv_obj_set_pos(g_net_label, 4, y);
  y += 20;

  g_ip_label = lv_label_create(scr);
  lv_obj_set_style_text_color(g_ip_label, lv_color_white(), 0);
  lv_label_set_text(g_ip_label, "IP: -");  // lot 4 : Wi-Fi
  lv_obj_set_pos(g_ip_label, 4, y);
  y += 28;

  lv_obj_t* events_title = lv_label_create(scr);
  lv_obj_set_style_text_color(events_title, lv_color_white(), 0);
  lv_label_set_text(events_title, "EVENEMENTS (plus recent en haut) :");
  lv_obj_set_pos(events_title, 4, y);
  y += 20;

  for (size_t i = 0; i < kVisibleEvents; ++i) {
    g_event_labels[i] = lv_label_create(scr);
    lv_obj_set_style_text_color(g_event_labels[i], lv_color_white(), 0);
    lv_label_set_text(g_event_labels[i], "-");
    lv_obj_set_pos(g_event_labels[i], 4, y);
    y += 18;
  }

  g_touch_label = lv_label_create(scr);
  lv_obj_set_style_text_color(g_touch_label, lv_color_white(), 0);
  lv_label_set_text(g_touch_label, "");
  lv_obj_set_pos(g_touch_label, 4, kLcdVRes - 24);
}

void refresh_timer_cb(lv_timer_t* /*timer*/) {
  // Détection de perte/reprise de présence CAN : c'est ici, pas dans une
  // tâche dédiée, que tick_presence() est appelé — 200 ms de période
  // suffisent très largement pour le seuil de 3 s du critère de sortie.
  can_link::tick_presence();

  set_label_if_changed(g_can_label, can_link::presence_lost() ? "CAN: PERDU" : "CAN: OK");

  core::Event events[kVisibleEvents];
  size_t n = core::events::recent(events, kVisibleEvents);
  for (size_t i = 0; i < kVisibleEvents; ++i) {
    if (i < n) {
      char buf[40];
      std::snprintf(buf, sizeof(buf), "%lu.%03lus %s", static_cast<unsigned long>(events[i].uptime_ms / 1000),
                    static_cast<unsigned long>(events[i].uptime_ms % 1000), core::events::to_text(events[i].kind));
      set_label_if_changed(g_event_labels[i], buf);
    } else {
      set_label_if_changed(g_event_labels[i], "-");
    }
  }

  if (g_touch_label_visible && (now_ms() - g_last_touch_ms) > kTouchLabelHoldMs) {
    set_label_if_changed(g_touch_label, "");
    g_touch_label_visible = false;
  }
}

// L'ISR LCD est attachée au cœur qui appelle esp_lcd_new_rgb_panel.
// docs/screen-issue.md : l'installer depuis le cœur 1, avec LVGL, pour
// qu'elle préempte les écritures framebuffer au lieu de les concurrencer
// depuis app_main (cœur 0).
void init_on_core1() {
  // Dalle et tactile alimentés uniquement via l'état CH422G maintenu en RAM
  // (board::panel_power_on(), voir board.cpp) — jamais un accès direct au
  // registre de sortie partagé avec CAN_SEL.
  can_link::send_log(common::LogCode::kLcdInitStep, common::LogSeverity::kDebug, 1);
  board::panel_power_on();

  can_link::send_log(common::LogCode::kLcdInitStep, common::LogSeverity::kDebug, 2);
  esp_lcd_panel_handle_t panel_handle = init_rgb_panel();

  can_link::send_log(common::LogCode::kLcdInitStep, common::LogSeverity::kDebug, 3);
  esp_lcd_touch_handle_t touch_handle = init_touch();

  can_link::send_log(common::LogCode::kLcdInitStep, common::LogSeverity::kDebug, 4);
  init_lvgl_port(panel_handle, touch_handle);

  can_link::send_log(common::LogCode::kLcdInitStep, common::LogSeverity::kDebug, 5);
  // Timeout non nul : cette tâche est volontairement sous la priorité LVGL
  // (4), donc le mutex peut être relâché. Un timeout 0 sauterait l'UI si
  // la tâche LVGL tenait déjà le verrou au premier tick.
  if (lvgl_port_lock(1000)) {
    build_ui();
    lv_timer_create(refresh_timer_cb, kRefreshPeriodMs, nullptr);
    lvgl_port_unlock();
  }

  can_link::send_log(common::LogCode::kLcdInitStep, common::LogSeverity::kDebug, 6);
  core::events::push(core::EventKind::kBoot);
  can_link::send_log(common::LogCode::kLcdInitStep, common::LogSeverity::kDebug, 7);
}

void lcd_init_task(void* arg) {
  init_on_core1();
  xSemaphoreGive(static_cast<SemaphoreHandle_t>(arg));
  vTaskDelete(nullptr);
}

}  // namespace

void init() {
  if (xPortGetCoreID() == 1) {
    init_on_core1();
    return;
  }

  SemaphoreHandle_t done = xSemaphoreCreateBinary();
  ESP_ERROR_CHECK(done != nullptr ? ESP_OK : ESP_ERR_NO_MEM);
  // Priorité 3 < LVGL (4) : évite un deadlock sur le mutex LVGL si l'ISR
  // ou la tâche LVGL le tiennent déjà sur ce même cœur.
  BaseType_t created =
      xTaskCreatePinnedToCore(lcd_init_task, "lcd_init", 12 * 1024, done, 3, nullptr, 1);
  ESP_ERROR_CHECK(created == pdPASS ? ESP_OK : ESP_FAIL);
  xSemaphoreTake(done, portMAX_DELAY);
  vSemaphoreDelete(done);
}

}  // namespace service_screen

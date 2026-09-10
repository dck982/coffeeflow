// Diagnostic d'ordre d'allocation LCD/Wi-Fi pour Waveshare ESP32-S3-Touch-LCD-4.3.
// Projet autonome : CH422G, AP Wi-Fi/httpd, LCD RGB, LVGL et mire uniquement.
// Il isole l'hypothese v0.2.22 : Wi-Fi/httpd fragmente la RAM interne avant les
// buffers RGB DMA. CONFIG_LCD_TEST_WIFI_FIRST selectionne l'ordre A/B.

#include <cstdio>
#include <cstring>

#include "driver/gpio.h"
#include "driver/i2c_master.h"
#include "esp_err.h"
#include "esp_event.h"
#include "esp_http_server.h"
#include "esp_lcd_panel_ops.h"
#include "esp_lcd_panel_rgb.h"
#include "esp_log.h"
#include "esp_netif.h"
#include "esp_wifi.h"
#include "esp_lvgl_port.h"
#include "freertos/FreeRTOS.h"
#include "freertos/semphr.h"
#include "freertos/task.h"
#include "nvs_flash.h"

namespace {
constexpr const char* kTag = "screen-lcd-test";
constexpr gpio_num_t kI2cSda = GPIO_NUM_8;
constexpr gpio_num_t kI2cScl = GPIO_NUM_9;
constexpr uint16_t kCh422gModeAddr = 0x24;
constexpr uint16_t kCh422gOutAddr = 0x38;
constexpr uint8_t kCh422gModeOutputs = 0x01;
constexpr uint8_t kTpRst = 1 << 1;
constexpr uint8_t kLcdBl = 1 << 2;
constexpr uint8_t kLcdRst = 1 << 3;
constexpr uint8_t kSdCs = 1 << 4;
constexpr uint32_t kLcdHRes = 800;
constexpr uint32_t kLcdVRes = 480;
constexpr uint32_t kPixelClockHz = 16 * 1000 * 1000;
constexpr uint32_t kBounceBufferSizePx = kLcdHRes * 40;

i2c_master_dev_handle_t g_ch422g_out = nullptr;
uint8_t g_ch422g_outputs = 0;

void set_ch422g_outputs(uint8_t outputs) {
  g_ch422g_outputs = outputs;
  ESP_ERROR_CHECK(i2c_master_transmit(g_ch422g_out, &g_ch422g_outputs, 1, -1));
}

void init_ch422g() {
  i2c_master_bus_config_t bus{};
  bus.i2c_port = -1;
  bus.sda_io_num = kI2cSda;
  bus.scl_io_num = kI2cScl;
  bus.clk_source = I2C_CLK_SRC_DEFAULT;
  bus.glitch_ignore_cnt = 7;
  bus.flags.enable_internal_pullup = true;
  i2c_master_bus_handle_t bus_handle = nullptr;
  ESP_ERROR_CHECK(i2c_new_master_bus(&bus, &bus_handle));

  i2c_device_config_t cfg{};
  cfg.dev_addr_length = I2C_ADDR_BIT_LEN_7;
  cfg.device_address = kCh422gModeAddr;
  cfg.scl_speed_hz = 400000;
  i2c_master_dev_handle_t mode = nullptr;
  ESP_ERROR_CHECK(i2c_master_bus_add_device(bus_handle, &cfg, &mode));
  cfg.device_address = kCh422gOutAddr;
  ESP_ERROR_CHECK(i2c_master_bus_add_device(bus_handle, &cfg, &g_ch422g_out));
  const uint8_t mode_outputs = kCh422gModeOutputs;
  ESP_ERROR_CHECK(i2c_master_transmit(mode, &mode_outputs, 1, -1));

  // USB/CAN_SEL reste bas : CAN est hors scope et la console USB reste lisible.
  set_ch422g_outputs(kSdCs);
  vTaskDelay(pdMS_TO_TICKS(20));
  set_ch422g_outputs(kSdCs | kLcdRst | kLcdBl);
  vTaskDelay(pdMS_TO_TICKS(20));
  set_ch422g_outputs(kSdCs | kLcdRst | kLcdBl | kTpRst);
  vTaskDelay(pdMS_TO_TICKS(200));
}

esp_err_t status_handler(httpd_req_t* request) {
  static constexpr char kResponse[] = "screen-lcd-test: Wi-Fi and HTTP running\n";
  httpd_resp_set_type(request, "text/plain; charset=utf-8");
  return httpd_resp_send(request, kResponse, HTTPD_RESP_USE_STRLEN);
}

void init_wifi_http() {
  ESP_ERROR_CHECK(esp_netif_init());
  ESP_ERROR_CHECK(esp_event_loop_create_default());
  ESP_ERROR_CHECK(esp_netif_create_default_wifi_ap() != nullptr ? ESP_OK : ESP_FAIL);
  wifi_init_config_t wifi_cfg = WIFI_INIT_CONFIG_DEFAULT();
  ESP_ERROR_CHECK(esp_wifi_init(&wifi_cfg));
  // Aucun credential et aucune ecriture NVS : AP ouvert, reserve au diagnostic.
  ESP_ERROR_CHECK(esp_wifi_set_storage(WIFI_STORAGE_RAM));
  wifi_config_t ap{};
  std::snprintf(reinterpret_cast<char*>(ap.ap.ssid), sizeof(ap.ap.ssid), "CoffeeFlow-LCD-Diag");
  ap.ap.ssid_len = std::strlen(reinterpret_cast<const char*>(ap.ap.ssid));
  ap.ap.channel = 1;
  ap.ap.max_connection = 1;
  ap.ap.authmode = WIFI_AUTH_OPEN;
  ESP_ERROR_CHECK(esp_wifi_set_mode(WIFI_MODE_AP));
  ESP_ERROR_CHECK(esp_wifi_set_config(WIFI_IF_AP, &ap));
  ESP_ERROR_CHECK(esp_wifi_start());

  httpd_handle_t server = nullptr;
  httpd_config_t http_cfg = HTTPD_DEFAULT_CONFIG();
  http_cfg.max_open_sockets = 1;
  ESP_ERROR_CHECK(httpd_start(&server, &http_cfg));
  const httpd_uri_t status{.uri = "/", .method = HTTP_GET, .handler = status_handler, .user_ctx = nullptr};
  ESP_ERROR_CHECK(httpd_register_uri_handler(server, &status));
}

esp_lcd_panel_handle_t init_lcd() {
  esp_lcd_rgb_panel_config_t cfg{};
  cfg.clk_src = LCD_CLK_SRC_DEFAULT;
  cfg.timings.pclk_hz = kPixelClockHz;
  cfg.timings.h_res = kLcdHRes;
  cfg.timings.v_res = kLcdVRes;
  cfg.timings.hsync_pulse_width = 48;
  cfg.timings.hsync_back_porch = 88;
  cfg.timings.hsync_front_porch = 40;
  cfg.timings.vsync_pulse_width = 3;
  cfg.timings.vsync_back_porch = 32;
  cfg.timings.vsync_front_porch = 13;
  cfg.timings.flags.pclk_active_neg = 1;
  cfg.data_width = 16;
  cfg.in_color_format = LCD_COLOR_FMT_RGB565;
  cfg.out_color_format = LCD_COLOR_FMT_RGB565;
  cfg.num_fbs = 1;
  cfg.bounce_buffer_size_px = kBounceBufferSizePx;
  cfg.hsync_gpio_num = GPIO_NUM_46;
  cfg.vsync_gpio_num = GPIO_NUM_3;
  cfg.de_gpio_num = GPIO_NUM_5;
  cfg.pclk_gpio_num = GPIO_NUM_7;
  cfg.disp_gpio_num = GPIO_NUM_NC;
  constexpr gpio_num_t kDataPins[16] = {GPIO_NUM_14, GPIO_NUM_38, GPIO_NUM_18, GPIO_NUM_17,
                                         GPIO_NUM_10, GPIO_NUM_39, GPIO_NUM_0, GPIO_NUM_45,
                                         GPIO_NUM_48, GPIO_NUM_47, GPIO_NUM_21, GPIO_NUM_1,
                                         GPIO_NUM_2, GPIO_NUM_42, GPIO_NUM_41, GPIO_NUM_40};
  for (size_t i = 0; i < 16; ++i) cfg.data_gpio_nums[i] = kDataPins[i];
  cfg.flags.fb_in_psram = 1;
  esp_lcd_panel_handle_t panel = nullptr;
  ESP_ERROR_CHECK(esp_lcd_new_rgb_panel(&cfg, &panel));
  ESP_ERROR_CHECK(esp_lcd_panel_reset(panel));
  ESP_ERROR_CHECK(esp_lcd_panel_init(panel));
  return panel;
}

void add_stripe(lv_obj_t* screen, int y, lv_color_t color) {
  lv_obj_t* stripe = lv_obj_create(screen);
  lv_obj_remove_style_all(stripe);
  lv_obj_set_size(stripe, kLcdHRes, 80);
  lv_obj_set_pos(stripe, 0, y);
  lv_obj_set_style_bg_color(stripe, color, 0);
  lv_obj_set_style_bg_opa(stripe, LV_OPA_COVER, 0);
}

void init_lvgl_and_pattern(esp_lcd_panel_handle_t panel) {
  lvgl_port_cfg_t port_cfg = ESP_LVGL_PORT_INIT_CONFIG();
  port_cfg.task_affinity = 1;
  port_cfg.task_stack = 12 * 1024;
  ESP_ERROR_CHECK(lvgl_port_init(&port_cfg));
  lvgl_port_display_cfg_t disp_cfg{};
  disp_cfg.panel_handle = panel;
  disp_cfg.buffer_size = kLcdHRes * 60;
  disp_cfg.hres = kLcdHRes;
  disp_cfg.vres = kLcdVRes;
  disp_cfg.color_format = LV_COLOR_FORMAT_RGB565;
  disp_cfg.flags.buff_spiram = true;
  lvgl_port_display_rgb_cfg_t rgb_cfg{};
  rgb_cfg.flags.bb_mode = 1;
  ESP_ERROR_CHECK(lvgl_port_add_disp_rgb(&disp_cfg, &rgb_cfg) != nullptr ? ESP_OK : ESP_FAIL);

  ESP_ERROR_CHECK(lvgl_port_lock(0) ? ESP_OK : ESP_ERR_TIMEOUT);
  lv_obj_t* screen = lv_screen_active();
  lv_obj_remove_style_all(screen);
  lv_obj_set_style_bg_color(screen, lv_color_black(), 0);
  lv_obj_set_style_bg_opa(screen, LV_OPA_COVER, 0);
  add_stripe(screen, 0, lv_palette_main(LV_PALETTE_RED));
  add_stripe(screen, 80, lv_palette_main(LV_PALETTE_YELLOW));
  add_stripe(screen, 160, lv_palette_main(LV_PALETTE_GREEN));
  add_stripe(screen, 240, lv_palette_main(LV_PALETTE_BLUE));
  add_stripe(screen, 320, lv_palette_main(LV_PALETTE_PURPLE));
  add_stripe(screen, 400, lv_palette_main(LV_PALETTE_ORANGE));
  lv_obj_t* label = lv_label_create(screen);
  lv_obj_set_style_text_color(label, lv_color_white(), 0);
  lv_obj_set_style_text_font(label, &lv_font_montserrat_20, 0);
#if CONFIG_LCD_TEST_WIFI_FIRST
  lv_label_set_text(label, "Wi-Fi + HTTP  ->  LCD / LVGL");
#else
  lv_label_set_text(label, "LCD / LVGL  ->  Wi-Fi + HTTP");
#endif
  lv_obj_center(label);
  lvgl_port_unlock();
}

// app_main tourne sur le cœur 0 (CONFIG_ESP_MAIN_TASK_AFFINITY_CPU0) alors
// que LVGL est épinglé au cœur 1 (task_affinity ci-dessus) : comme dans
// firmware/screen/main/service_screen.cpp (docs/screen-issue.md), l'ISR DMA
// du panneau RGB doit être installée depuis le même cœur que LVGL, sinon
// les deux cœurs se disputent la PSRAM et le texte saute/décale.
void lcd_and_lvgl_task(void* arg) {
  init_lvgl_and_pattern(init_lcd());
  xSemaphoreGive(static_cast<SemaphoreHandle_t>(arg));
  vTaskDelete(nullptr);
}

void init_lcd_and_lvgl_on_core1() {
  SemaphoreHandle_t done = xSemaphoreCreateBinary();
  ESP_ERROR_CHECK(done != nullptr ? ESP_OK : ESP_ERR_NO_MEM);
  BaseType_t created =
      xTaskCreatePinnedToCore(lcd_and_lvgl_task, "lcd_lvgl_init", 12 * 1024, done, 3, nullptr, 1);
  ESP_ERROR_CHECK(created == pdPASS ? ESP_OK : ESP_FAIL);
  xSemaphoreTake(done, portMAX_DELAY);
  vSemaphoreDelete(done);
}
}  // namespace

extern "C" void app_main() {
  // Comme storage::init() dans firmware/screen : NVS doit être prêt avant
  // esp_wifi_init(), qui l'ouvre en interne pour la calibration radio.
  ESP_ERROR_CHECK(nvs_flash_init());
  ESP_LOGI(kTag, "initialisation CH422G");
  init_ch422g();
#if CONFIG_LCD_TEST_WIFI_FIRST
  ESP_LOGI(kTag, "ordre test: Wi-Fi/HTTP avant LCD");
  init_wifi_http();
  init_lcd_and_lvgl_on_core1();
#else
  ESP_LOGI(kTag, "ordre controle: LCD avant Wi-Fi/HTTP");
  init_lcd_and_lvgl_on_core1();
  init_wifi_http();
#endif
  ESP_LOGI(kTag, "mire prete; AP CoffeeFlow-LCD-Diag, HTTP /");
}

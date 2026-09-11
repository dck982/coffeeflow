idf_component_register(
  SRCS "main.cpp" "board.cpp" "can_link.cpp" "serial_bridge.cpp" "ota_local.cpp" "ota_proxy.cpp" "ble_scale.cpp"
       "service_screen.cpp" "storage.cpp" "net_wifi.cpp" "net_http.cpp" "net_ws.cpp" "core/events.cpp" "core/core.cpp" "core/config.cpp" "core/machine.cpp" "ui/ui_root.cpp" "ui/ui_home.cpp"
       "ui/ui_fonts/ui_font_18.c" "ui/ui_fonts/ui_font_26.c" "ui/ui_fonts/ui_font_28.c" "ui/ui_fonts/ui_font_32.c" "ui/ui_fonts/ui_font_40.c" "ui/ui_fonts/ui_font_76.c" "ui/ui_fonts/ui_font_104.c"
  INCLUDE_DIRS "."
  REQUIRES common driver freertos esp_driver_uart esp_driver_i2c esp_driver_gpio esp_timer app_update esp_partition
           esp_lcd esp_lvgl_port esp_lcd_touch_gt911 lvgl nvs_flash esp_wifi esp_event esp_netif esp_http_server espressif__cjson bt
)

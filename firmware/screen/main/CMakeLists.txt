idf_component_register(
  SRCS "main.cpp" "board.cpp" "can_link.cpp" "serial_bridge.cpp" "ota_local.cpp" "ota_proxy.cpp"
       "service_screen.cpp" "core/events.cpp" "core/core.cpp"
  INCLUDE_DIRS "."
  REQUIRES common driver freertos esp_driver_uart esp_driver_i2c esp_driver_gpio esp_timer app_update esp_partition
           esp_lcd esp_lvgl_port esp_lcd_touch_gt911 lvgl
)

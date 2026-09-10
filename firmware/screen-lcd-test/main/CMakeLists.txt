idf_component_register(
  SRCS "main.cpp"
  INCLUDE_DIRS "."
  REQUIRES driver esp_driver_gpio esp_driver_i2c esp_lcd esp_lvgl_port lvgl
           esp_wifi esp_event esp_netif esp_http_server nvs_flash
)

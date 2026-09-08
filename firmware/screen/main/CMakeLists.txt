idf_component_register(
  SRCS "main.cpp"
  INCLUDE_DIRS "."
  REQUIRES common driver freertos esp_driver_uart esp_driver_i2c esp_driver_gpio esp_timer
)

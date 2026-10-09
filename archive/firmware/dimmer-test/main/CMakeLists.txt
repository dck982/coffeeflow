idf_component_register(
  SRCS "main.cpp"
  INCLUDE_DIRS "."
  REQUIRES driver esp_driver_i2c esp_driver_gpio
)

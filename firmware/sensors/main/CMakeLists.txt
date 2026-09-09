idf_component_register(
  SRCS "main.cpp"
  INCLUDE_DIRS "."
  REQUIRES common driver esp_driver_gpio esp_driver_i2c esp_timer esp_system app_update esp_partition
)

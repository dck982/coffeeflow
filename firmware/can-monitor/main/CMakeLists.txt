idf_component_register(
  SRCS "main.cpp"
  INCLUDE_DIRS "."
  REQUIRES common driver freertos esp_driver_usb_serial_jtag
)

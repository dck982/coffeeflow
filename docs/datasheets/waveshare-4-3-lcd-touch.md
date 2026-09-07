# ESP32-S3-Touch-LCD-4.3

URL: https://docs.waveshare.com/ESP32-S3-Touch-LCD-4.3

[![ESP32-S3-Touch-LCD-4.3-1](https://docs.waveshare.com/assets/images/ESP32-S3-Touch-LCD-4.3-1-78b46cc0b6805f2016035291b09fc98a.webp)](https://www.waveshare.com/esp32-s3-touch-lcd-4.3.htm)

| SKU | Product |
| --- | --- |
| 25948 | ESP32-S3-Touch-LCD-4.3 |
| 30493 | ESP32-S3-LCD-4.3 |

## Product Introduction

### Product Overview

The ESP32-S3-Touch-LCD-4.3 is a low-cost, high-performance microcontroller development board designed by Waveshare. It supports 2.4GHz Wi-Fi and BLE 5, integrates large-capacity Flash and PSRAM, and features an onboard 4.3inch wide capacitive touch LCD screen, enabling smooth operation of GUI applications such as LVGL. Combined with various peripheral interfaces (e.g., CAN, I2C, and RS485), it allows for rapid development of HMI applications based on the ESP32-S3. The rich functions and interfaces meet the power consumption requirements for application scenarios such as the Internet of Things (IoT), mobile devices, and smart homes.

### Product Parameters

| Basic Parameters |  |
| --- | --- |
| **Processor** | High-performance Xtensa 32-bit LX7 dual-core processor, up to 240 MHz |
| **Wi-Fi/Bluetooth** | Supports 2.4 GHz Wi-Fi (802.11 b/g/n) and Bluetooth 5 (LE), onboard antenna |
| **Embedded Memory** | 512KB SRAM and 384KB ROM |
| **Flash** | 16MB Flash |
| **PSRAM** | 8MB PSRAM |
| **Power Supply** | TypeC 5V |
| **Screen Parameters** |  |
| **Resolution** | 800 x 480 |
| **Display Colors** | 65K colors |
| **Display Interface** | RGB |
| **Display Panel** | IPS |
| **Viewing Angle** | 160° |
| **Brightness** | 270 Cd/m² |
| **Touch Type** | Capacitive |
| **Touch Panel** | Tempered Glass |
| **Touch Features** | Supports I2C interface control for capacitive touch (optional), 5-point touch, interrupt support |
| **Peripheral Interfaces** |  |
| **Communication Interfaces** | CAN, RS485, I2C, USB |
| **USB** | Integrated full-speed USB |
| **Others** |  |
| **Power Consumption** | 5V 450mA |
| **Operating Temperature** | 0℃ ~ 65℃ |
| **Dimensions (L×W)** | Non-touch version: 105.4×67.1mm Touch version: 106.1×67.8mm |

### Features

- Powered by a high-performance Xtensa 32-bit LX7 dual-core processor, with a main frequency of up to 240MHz
- Supports 2.4 GHz Wi-Fi (802.11 b/g/n) and Bluetooth 5 (LE) with an onboard antenna
- Built-in 512KB SRAM and 384KB ROM, stacked with 16MB Flash and 8MB PSRAM
- Onboard 4.3inch LCD screen, 800 × 480 resolution, 65K colors
- Supports I2C interface control for capacitive touch (optional), 5-point touch, interrupt support
- Onboard CAN, RS485, I2C interfaces, TF card slot, and integrated full-speed USB
- Supports precise control features like flexible clocking and independent module power supply settings, enabling low-power modes for multiple scenarios

## Onboard Resources

![ESP32-S3-Touch-LCD-4.3-details-intro](https://docs.waveshare.com/assets/images/ESP32-S3-Touch-LCD-4.3-details-intro-412ade570ab519921350b8144d907b94.webp)

1. **ESP32-S3-WROOM-1-N16R8**
Wi-Fi/Bluetooth SoC module, 240MHz operating frequency
Stacked 8MB PSRAM and 16MB Flash
2. **SGM2212-3.3**
800mA low-noise LDO
3. **FSUSB42UMX**
For USB pin multiplexing
4. **Screen Touch Connector**
No touch, left floating
5. **USB TO UART Type-C Port**
6. **USB Type-C Port**
7. **TJA1051T/3/1J**
CAN interface chip
8. **CH422G**
IO expander chip
9. **BOOT Button**
Press and hold while powering on for program flashing
10. **RESET Button**
Press to reset the controller
11. **MP3302DJ-LF-Z**
Screen backlight boost chip

1. **4.3inch Display Panel Connector**
2. **TF Card Slot**
3. **Sensor terminal:** Recommended to use the included HY2.0 3P to Dupont male 3P 10 cm cable
4. **CAN interface:** Recommended to use the included HY2.0 2P to Dupont male 2P 10 cm cable
5. **I2C terminal:** Recommended to use the included HY2.0 4P to Dupont male 4P 10 cm cable
6. **RS485 terminal:** Recommended to use the included HY2.0 2P to Dupont male 2P 10 cm cable
7. **3.7V Single-cell Lithium Battery PH2.0 Header**
8. **CAN Terminal Resistor Selection Interface**
9. **RS485 Terminal Resistor Selection Interface**
10. **CH343P**
USB to UART chip
11. **SP3485**
RS485 transceiver chip
12. **CS8501**
Lithium battery charging management chip
13. **Status Indicators**
PWR power indicator
CHG lithium battery charging indicator
DONE lithium battery charging complete indicator

## Interface Description

When using the ESP32-S3-Touch-LCD-4.3, it's important to understand the hardware connections for different peripherals.

**LCD Interface: Connector for the LCD cable (click to expand)**
| ESP32-S3 | LCD | Description |
| --- | --- | --- |
| GPIO0 | G3 | Green data bit 3 |
| GPIO1 | R3 | Red data bit 3 |
| GPIO2 | R4 | Red data bit 4 |
| GPIO3 | VSYNC | Vertical sync signal |
| GPIO5 | DE | Data enable signal |
| GPIO7 | PCLK | Pixel clock signal |
| GPIO10 | B7 | Blue data bit 7 |
| GPIO14 | B3 | Blue data bit 3 |
| GPIO17 | B6 | Blue data bit 6 |
| GPIO18 | B5 | Blue data bit 5 |
| GPIO21 | G7 | Green data bit 7 |
| GPIO38 | B4 | Blue data bit 4 |
| GPIO39 | G2 | Green data bit 2 |
| GPIO40 | R7 | Red data bit 7 |
| GPIO41 | R6 | Red data bit 6 |
| GPIO42 | R5 | Red data bit 5 |
| GPIO45 | G4 | Green data bit 4 |
| GPIO46 | HSYNC | Horizontal sync signal |
| GPIO47 | G6 | Green data bit 6 |
| GPIO48 | G5 | Green data bit 5 |
| **CH422G** | **LCD** | **-** |
| EXIO2 | DISP | Backlight enable pin |

**Touch Interface: Connects to the touch cable (click to expand)**
| ESP32-S3 | Touch | Description |
| --- | --- | --- |
| GPIO4 | TP_IRQ | Touch interrupt pin |
| GPIO8 | TP_SDA | Touch data pin (I2C) |
| GPIO9 | TP_SCL | Touch clock pin (I2C) |
| **CH422G** | **Touch** | **-** |
| EXIO1 | TP_RST | Touch reset pin |

**USB Interface: Used for power supply and flashing (click to expand)**
| ESP32-S3 | USB | Description |
| --- | --- | --- |
| GPIO19 | USB_DN | Data line D- |
| GPIO20 | USB_DP | Data line D+ |
| **CH422G** | **USB** | **-** |
| EXIO5 | USB_SEL | Pull low to set USB mode, otherwise CAN mode |

**TF Card Interface: Connects to the TF card (click to expand)**
| ESP32-S3 | TF | Description |
| --- | --- | --- |
| GPIO11 | MOSI | TF card input pin |
| GPIO12 | SCK | TF card clock pin |
| GPIO13 | MISO | TF card output pin |
| **CH422G** | **TF** | **-** |
| EXIO4 | SD_CS | TF card chip select, active low |

**RS485 Interface: Onboard RS485 interface for direct device communication; transmission/reception mode is switched automatically (click to expand)**
| ESP32-S3 | RS485 | Description |
| --- | --- | --- |
| GPIO16 | RS485_RXD | Data input |
| GPIO15 | RS485_TXD | Data output |

**CAN Interface: Enables transmission/reception control, data analysis, acquisition, and monitoring for CAN bus networks (click to expand)**
| ESP32-S3 | CAN | Description |
| --- | --- | --- |
| GPIO15 | CANTX | Data output |
| GPIO16 | CANRX | Data input |
| **CH422G** | **CAN** | **-** |
| EXIO5 | CAN_SEL | Pull high to set CAN mode, otherwise USB mode |

**I2C Interface: Connects to the I/O expander chip, touch interface, and external interfaces (click to expand)** ESP32-S3 provides multiple hardware I2C channels. Currently, pins GPIO8 (SDA) and GPIO9 (SCL) are used for the I2C bus.

| ESP32-S3 | I2C | Description |
| --- | --- | --- |
| GPIO8 | SDA | I2C data pin |
| GPIO9 | SCL | I2C clock pin |

- **PH2.0 Battery Interface** : The development board uses the efficient charging/discharging management chip CS8501, which can boost a single-cell lithium battery to 5V. The current charging current is 580mA. Users can change the charging current by replacing resistor R45. For details, please refer to the [ESP32-S3-Touch-LCD-4.3 Schematic](https://files.waveshare.com/wiki/ESP32-S3-Touch-LCD-4.3/ESP32-S3-Touch-LCD-4.3-Sch.pdf)

## Dimensions

**Without Touch Version**

![ESP32-S3-LCD-4.3-details-size](https://docs.waveshare.com/assets/images/ESP32-S3-LCD-4.3-details-size-368725c4e453fa389f729e2b8708ccad.webp)

**With Touch Version**

![Esp32-s3-touch-lcd-4.3-003](https://docs.waveshare.com/assets/images/Esp32-s3-touch-lcd-4.3-003-e8568a2882a6d2b3b0b39a730a8332b8.webp)

## Development Methods

The ESP32-S3-Touch-LCD-4.3 supports both the Arduino IDE and ESP-IDF development frameworks, giving developers flexible options. You can choose the development tool that best suits your project needs and personal preferences.

Arduino generally offers a gentler learning curve and may be more approachable for beginners and hobbyists. ESP-IDF provides advanced tooling and finer control over system behavior, making it better suited to complex projects and applications with demanding performance requirements.

- **Arduino IDE** is a convenient, flexible, and easy-to-use open-source electronics prototyping platform. It requires minimal foundational knowledge, allowing for rapid development after a short learning period. Arduino has a huge global user community, providing a vast amount of open-source code, project examples, and tutorials, as well as a rich library ecosystem that encapsulates complex functions, enabling developers to implement various features rapidly. You can refer to the **[Working with Arduino](https://docs.waveshare.com/ESP32-Arduino-Tutorials/Arduino-IDE-Setup)** to complete the initial setup, and the tutorial also provides related examples for reference.
- **ESP-IDF** , short for Espressif IoT Development Framework, is a professional development framework launched by Espressif Systems for its ESP series of chips. It is based on C language development and includes compilers, debuggers, flashing tools, etc. It supports development via command line or integrated development environments (such as Visual Studio Code with the Espressif IDF plugin), which provides features like code navigation, project management, and debugging. We recommend using VS Code for development. For the specific configuration process, please refer to the **[Working with ESP-IDF](https://docs.waveshare.com/ESP32-ESP-IDF-Tutorials/ESP-IDF-Installation)** . The tutorial also provides relevant example programs for reference.

[Give Feedback](https://docs.google.com/forms/d/e/1FAIpQLSfayJEZ5J-dp-3Wq_dkWsVRhNuQ6C79_GNV62mYuFW8Mj6U8Q/viewform?entry.803599728=https%3A%2F%2Fdocs.waveshare.com%2FESP32-S3-Touch-LCD-4.3)

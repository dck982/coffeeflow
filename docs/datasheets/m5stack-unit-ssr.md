# Unit SSR

<span class="product-sku">SKU:U122</span>

<PictureViewer>
<img src="https://static-cdn.m5stack.com/resource/docs/products/unit/ssr/ssr_01.webp">
<img src="https://static-cdn.m5stack.com/resource/docs/products/unit/ssr/ssr_02.webp">
<img src="https://m5stack-doc.oss-cn-shenzhen.aliyuncs.com/785/U122-01.webp">
<img src="https://static-cdn.m5stack.com/resource/docs/products/unit/ssr/ssr_05.webp">
<img src="https://static-cdn.m5stack.com/resource/docs/products/unit/ssr/ssr_06.webp">
</PictureViewer>

## Description

**Unit SSR** is a solid-state relay integrated with MOC3043M optocoupler isolation and zero-crossing detection. It supports a 3.3-5V DC control signal to control a single-phase 220-250V AC power output. Combined with the M5 controller and the UIFlow programming platform, it can easily achieve remote control. Compared to ordinary mechanical relays, this solid-state relay uses power semiconductor devices for control, offering microsecond-level switching speed and built-in overcurrent protection. It has no physical contacts, long lifespan, high reliability, and can adapt to complex working environments such as severe vibrations. It is used in scenarios requiring frequent switching and fast response, such as lighting control and CNC machine tools.

/#>1. Be cautious when using high-voltage AC loads; do not operate with power on.<br/> 2. This relay is only suitable for AC loads.

## Features

- Zero-crossing SSR
- Built-in overcurrent protection
- Built-in optocoupler isolation with zero-crossing detection
- Fast speed, low noise, long lifespan, high reliability, and high sensitivity
- Uses GROVE interface for easier connection
- Supports UIFlow graphical programming, enabling remote control of the relay with an M5 controller in just 3 minutes

/#>**Advantages of Solid-State Relays over Mechanical Relays:**<br/>1. Faster switching speed with no physical contact wear.<br/>2. Completely silent operation.<br/>3. No electrical sparks, suitable for complex environments.<br/>4. Longer lifespan.<br/>5. Smaller size.

## Includes

- 1 x Unit SSR
- 1 x HY2.0-4P Grove cable (20cm)
- 1 x HT3.96-4P Terminal

## Applications

- Motor control
- Industrial and household lighting
- Heating and static switching
- Stage lighting
- Medical equipment, traffic lights

## Specifications

| Specification          | Parameter                 |
| ---------------------- | ------------------------- |
| Thyristor Model        | BT136S                    |
| Optocoupler Model      | MOC3043                   |
| Control Signal         | 3.3-5V DC                 |
| Switching Voltage      | Single-phase AC: 220-250V |
| Maximum Load Current   | 2A                        |
| Control Channels       | 1                         |
| Overcurrent Protection | Fuse: 2A                  |
| Operating Temperature  | -10 ~ 80°C                |
| Product Size           | 56.0 x 24.0 x 10.2mm      |
| Product Weight         | 8.3g                      |
| Package Size           | 138.0 x 93.0 x 11.2mm     |
| Gross Weight           | 16.5g                     |

## Schematics

<img src="https://static-cdn.m5stack.com/resource/docs/products/unit/ssr/ssr_sch_01.webp" width="80%">

## PinMap

### Unit SSR

::grove-table
| HY2.0-4P | Black | Red | Yellow | White |
| -------- | ----- | --- | ------ | ----- |
| PORT.B   | GND   | 5V  | DIN    | NC    |
::

## Datasheets

- [BT136S](https://m5stack.oss-cn-shenzhen.aliyuncs.com/resource/docs/datasheet/unit/ssr/C23427_BT136S.PDF)
- [MOC3043M](https://m5stack.oss-cn-shenzhen.aliyuncs.com/resource/docs/datasheet/unit/ssr/C41809_MOC3043SR2M_2015-01-24.PDF)

## Model Size

<img src="https://m5stack.oss-cn-shenzhen.aliyuncs.com/resource/docs/products/unit/ssr/model%20size.png" width="100%">

## Structure

- [Unit SSR Structure Files](https://github.com/m5stack/M5_Hardware/tree/master/Products/U122_Unit_SSR/Structures)

## Softwares

### Arduino

- [Unit SSR Arduino Tutorial](/en/arduino/projects/unit/unit_ssr)

### UiFlow2

- [Unit SSR UiFlow2 Docs](https://uiflow-micropython.readthedocs.io/en/latest/unit/ssr.html)

## Video

<video controls>
    <source src="https://m5stack.oss-cn-shenzhen.aliyuncs.com/video/Product_example_video/Unit/SSR_UNIT_VIDEO.mp4" type="video/mp4">
</video>

<TabPanel>
  <template #tab-Bilibili>
      <div class="video-iframe">
        <iframe src="//player.bilibili.com/player.html?isOutside=true&aid=113501624403830&bvid=BV1bGUnYKEyL&p=1&autoplay=0" loading="lazy" scrolling="no" border="0" frameborder="no" framespacing="0" allowfullscreen="true"></iframe>
      </div>
  </template>
  <template #tab-Youtube>
      <div class="video-iframe">
      <iframe width="560" height="315" src="https://www.youtube.com/embed/2pMBdyNAPcA?si=aZBv5QwR4EYAxDLW" title="YouTube video player" frameborder="0" loading="lazy" allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture; web-share" referrerpolicy="strict-origin-when-cross-origin" allowfullscreen></iframe>
      </div>
  </template>
</TabPanel>

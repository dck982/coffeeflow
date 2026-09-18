"""Test minimal de l'OLED Waveshare 0.91" (128 x 32, I2C).

Lancer avec :
    mpremote run tt_oled.py

Les broches sont celles du scan I2C fourni : SDA=2, SCL=1.
"""

from machine import Pin, SoftI2C
import time


SDA_PIN = 2
SCL_PIN = 1
OLED_ADDR = 0x3C
WIDTH = 128
HEIGHT = 32


class OLED:
    def __init__(self, i2c, address=OLED_ADDR):
        self.i2c = i2c
        self.address = address
        self.buffer = bytearray(WIDTH * HEIGHT // 8)

        # Initialisation SSD1306 correspondant à la démo C++.
        self.command(bytes((
            0xAE,       # display off
            0x40,       # start line 0
            0xB0,       # page address
            0xC8,       # COM scan direction
            0x81, 0xFF, # contrast
            0xA1,       # segment remap
            0xA6,       # normal display
            0xA8, 0x1F, # multiplex ratio: 32 lines
            0xD3, 0x00, # display offset
            0xD5, 0xF0, # clock divider
            0xD9, 0x22, # pre-charge
            0xDA, 0x02, # COM pins
            0xDB, 0x49, # VCOMH
            0x8D, 0x14, # charge pump
            0xAF,       # display on
        )))
        self.clear()

    def command(self, data):
        self.i2c.writeto(self.address, b"\x00" + data)

    def show(self):
        # Le contrôleur est organisé en 4 pages de 8 pixels.
        for page in range(HEIGHT // 8):
            self.command(bytes((0xB0 + page, 0x00, 0x10)))
            start = page * WIDTH
            self.i2c.writeto(self.address, b"\x40" + self.buffer[start:start + WIDTH])

    def clear(self):
        for i in range(len(self.buffer)):
            self.buffer[i] = 0
        self.show()

    def pixel(self, x, y, value=1):
        if 0 <= x < WIDTH and 0 <= y < HEIGHT:
            index = x + (y // 8) * WIDTH
            mask = 1 << (y & 7)
            if value:
                self.buffer[index] |= mask
            else:
                self.buffer[index] &= ~mask

    def rectangle(self, x0, y0, x1, y1):
        for x in range(x0, x1 + 1):
            self.pixel(x, y0)
            self.pixel(x, y1)
        for y in range(y0, y1 + 1):
            self.pixel(x0, y)
            self.pixel(x1, y)

    def text(self, x, y, message):
        # Police 5x7 minimale, suffisante pour le mot TEST.
        glyphs = {
            "T": (0x01, 0x01, 0x7F, 0x01, 0x01),
            "E": (0x7F, 0x49, 0x49, 0x49, 0x41),
            "S": (0x46, 0x49, 0x49, 0x49, 0x31),
            "U": (0x3F, 0x40, 0x40, 0x40, 0x3F),
            " ": (0, 0, 0, 0, 0),
        }
        for char in message:
            glyph = glyphs.get(char, glyphs[" "])
            for column, bits in enumerate(glyph):
                for row in range(7):
                    self.pixel(x + column, y + row, (bits >> row) & 1)
            x += 6


i2c = SoftI2C(
    sda=Pin(SDA_PIN, Pin.OPEN_DRAIN, Pin.PULL_UP),
    scl=Pin(SCL_PIN, Pin.OPEN_DRAIN, Pin.PULL_UP),
    freq=50000,
)

devices = i2c.scan()
print("Peripheriques I2C :", ["0x{:02X}".format(addr) for addr in devices])

if OLED_ADDR not in devices:
    raise OSError("OLED introuvable a 0x{:02X}".format(OLED_ADDR))

oled = OLED(i2c)
for n in range(10):
    if n%2==0:
        oled.text(8, 4, "TEST")
    else:
        oled.text(8, 4, "TUUT")
    oled.rectangle(85+2*n, 5, 120, 26)
    oled.show()
    time.sleep(0.2)
print("OLED affiche : TEST + carre")

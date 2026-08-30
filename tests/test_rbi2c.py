from machine import Pin, SoftI2C

# DimmerLink I2C. Not crossed: module TX/SDA -> G7, module RX/SCL -> G8.
# Internal pull-ups for a short bench cable. Add 4.7 kΩ to 3.3 V if the scan is empty.
i2c = SoftI2C(
    sda=Pin(7, Pin.OPEN_DRAIN, Pin.PULL_UP),
    scl=Pin(8, Pin.OPEN_DRAIN, Pin.PULL_UP),
    freq=50_000,
)

ADDR = 0x50
IDX = 0

REG_STATUS = 0x00
REG_COMMAND = 0x01
REG_ERROR = 0x02
REG_LEVEL = 0x10
REG_CURVE = 0x11
REG_FREQ = 0x20

CMD_SWITCH_UART = 0x03

OK = 0x00
ERR = {
    0xF9: "ERR_SYNTAX",
    0xFC: "ERR_NOT_READY",
    0xFD: "ERR_INDEX",
    0xFE: "ERR_PARAM",
}

CURVE_LINEAR = 0
CURVE_RMS = 1
CURVE_LOG = 2
CURVE_NAME = ("LINEAR", "RMS", "LOG")

found = i2c.scan()
print("I2C:", [hex(a) for a in found])
if ADDR not in found:
    raise RuntimeError("DimmerLink 0x50 introuvable")


def _read(reg):
    return i2c.readfrom_mem(ADDR, reg, 1)[0]


def _write(reg, value):
    i2c.writeto_mem(ADDR, reg, bytes([value]))
    err = _read(REG_ERROR)
    if err != OK:
        raise RuntimeError(ERR.get(err, "ERR_0x{:02X}".format(err)))


def set_brightness(level, idx=IDX):
    """Set brightness 0-100 %."""
    _write(REG_LEVEL, level)


def get_brightness(idx=IDX):
    """Return brightness 0-100 %."""
    return _read(REG_LEVEL)


def set_curve(curve_type, idx=IDX):
    """Set curve: 0=LINEAR, 1=RMS, 2=LOG."""
    _write(REG_CURVE, curve_type)


def get_curve(idx=IDX):
    """Return curve type 0=LINEAR, 1=RMS, 2=LOG."""
    return _read(REG_CURVE)


def get_mains_frequency():
    """Return mains frequency in Hz (50 or 60)."""
    return _read(REG_FREQ)


def switch_uart():
    """Switch to UART. Mode is saved in EEPROM; I2C stays off after reboot."""
    i2c.writeto_mem(ADDR, REG_COMMAND, bytes([CMD_SWITCH_UART]))


def main():
    hz = get_mains_frequency()
    print("mains {} Hz".format(hz))


main()

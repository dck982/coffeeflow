import time
from machine import Pin, UART

# DimmerLink UART 115200 8N1.
# Module RX -> GPIO 8 (ESP32 TX). Module TX -> GPIO 7 (ESP32 RX).
# UART0 is USB/REPL on the XIAO ESP32-S3.
uart = UART(1, baudrate=115200, bits=8, parity=None, stop=1, tx=Pin(8), rx=Pin(7))

STX = 0x02
IDX = 0

CMD_SET = 0x53  # 'S'
CMD_GET = 0x47  # 'G'
CMD_CURVE = 0x43  # 'C'
CMD_GETCURVE = 0x51  # 'Q'
CMD_FREQ = 0x52  # 'R'
CMD_SWITCH_I2C = 0x5B  # '['

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


def _drain():
    while uart.any():
        uart.read()


def _read(n, timeout_ms=50):
    t0 = time.ticks_ms()
    while uart.any() < n:
        if time.ticks_diff(time.ticks_ms(), t0) >= timeout_ms:
            return None
        time.sleep_ms(1)
    return uart.read(n)


def _status(byte):
    if byte == OK:
        return
    name = ERR.get(byte, "ERR_0x{:02X}".format(byte))
    raise RuntimeError(name)


def _cmd(payload, n_resp):
    _drain()
    uart.write(bytes(payload))
    resp = _read(n_resp)
    if resp is None or len(resp) < n_resp:
        raise RuntimeError("timeout, no UART response")
    _status(resp[0])
    return resp


def set_brightness(level, idx=IDX):
    """Set brightness 0-100 %."""
    _cmd((STX, CMD_SET, idx, level), 1)


def get_brightness(idx=IDX):
    """Return brightness 0-100 %."""
    return _cmd((STX, CMD_GET, idx), 2)[1]


def set_curve(curve_type, idx=IDX):
    """Set curve: 0=LINEAR, 1=RMS, 2=LOG."""
    _cmd((STX, CMD_CURVE, idx, curve_type), 1)


def get_curve(idx=IDX):
    """Return curve type 0=LINEAR, 1=RMS, 2=LOG."""
    return _cmd((STX, CMD_GETCURVE, idx), 2)[1]


def get_mains_frequency():
    """Return mains frequency in Hz (50 or 60)."""
    return _cmd((STX, CMD_FREQ), 2)[1]


def switch_i2c():
    """Switch to I2C. Mode is saved in EEPROM; UART stays off after reboot."""
    _cmd((STX, CMD_SWITCH_I2C), 1)


def test_blink():
    while True:
        set_brightness(0)
        print("0 %")
        time.sleep(1)
        set_brightness(100)
        print("100 %")
        time.sleep(1)


def main():
    switch_i2c()


main()

import time
from machine import ADC, Pin, UART

# 3.3 V logic probe / crude voltmeter. NOT 5 V tolerant. Not for mains.
# Wire the signal to SENSE, and the other side's GND to Atom GND.
#
# ESP32-S3 ADC1 is GPIO 1-10, so G1 G2 G5 G6 G7 G8 all work.

try:
    UART(1).deinit()
except OSError:
    pass

SENSE = 6

pin = Pin(SENSE, Pin.IN)
adc = ADC(pin)
adc.atten(ADC.ATTN_11DB)

print("GPIO {}  digital + ADC  (Ctrl+C to stop)".format(SENSE))

while True:
    raw = adc.read()
    mv = adc.read_uv() // 1000
    print("digital={}  adc={}  ~{} mV".format(pin.value(), raw, mv))
    time.sleep(0.2)

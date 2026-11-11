import time
from machine import Pin, PWM

# PUMP_SPEED on GPIO1 drives U8; SPEED_P (J9 pin 2) is filtered to 0-5 V.
# Bench test: J9 pin 1 on board GND, J9 pin 4 on board +5V.
STEP_S = 10

pwm = PWM(Pin(1), freq=1000, duty_u16=0)
for pct in (0, 50, 100):
    print("PUMP_SPEED %d %% for %d s" % (pct, STEP_S))
    pwm.duty_u16(pct * 65535 // 100)
    time.sleep(STEP_S)
pwm.duty_u16(0)
pwm.deinit()
Pin(1, Pin.IN)
print("PUMP_SPEED off")

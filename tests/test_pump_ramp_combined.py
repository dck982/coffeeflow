import time
from machine import Pin, PWM

# PUMP_SPEED on GPIO1 -> U8 -> SPEED_P (J9 pin 2).
# C20 charges through R36 (1.5 kΩ) while U8 conducts and discharges through
# R36 + R35 (11.5 kΩ) while it is off, so for a duty D the setpoint is
# V = VMAX * K * D / (1 + (K - 1) * D), K = 11.5 / 1.5. The duty for a
# target V inverts that curve.
K = 11.5 / 1.5
VMAX = 5.09  # measured on J9 pin 2 at 100 %, bench +5V_P = 5.16 V
STEP_S = 4


def duty_for(volts):
    x = min(max(volts / VMAX, 0.0), 1.0)
    return x / (K - (K - 1) * x)


pwm = PWM(Pin(1), freq=1000, duty_u16=0)
for volts in range(6):
    d = duty_for(volts)
    print("t=%2d s  target %d V  duty %.2f %%" % (volts * STEP_S, volts, d * 100))
    pwm.duty_u16(round(d * 65535))
    time.sleep(STEP_S)
pwm.duty_u16(0)
time.sleep(1)
pwm.deinit()
Pin(1, Pin.IN)
print("t=25 s  PUMP_SPEED off")

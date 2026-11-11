import time
from machine import Pin, PWM

# Low end of the PUMP_SPEED setpoint (J9 pin 2) at 1 kHz: short pulses,
# to see where U8 starts to follow.
FREQ = 1000
STEP_S = 6
DUTIES = (0.1, 0.3, 0.5, 1.0, 2.0)  # %

pwm = PWM(Pin(1), freq=FREQ, duty_u16=0)
for n, pct in enumerate(DUTIES, 1):
    print("%d/%d  duty %.1f %%  (pulse %.0f us)" % (n, len(DUTIES), pct, pct * 1e4 / FREQ))
    pwm.duty_u16(round(pct * 655.35))
    time.sleep(STEP_S)
pwm.duty_u16(0)
time.sleep(1)
pwm.deinit()
Pin(1, Pin.IN)
print("PUMP_SPEED off")

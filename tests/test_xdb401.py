import time
from machine import I2C, Pin

# Initialisation de l'I2C (adaptez les broches selon votre câblage, ici GPIO 26 et 32)
i2c = I2C(0, scl=Pin(8), sda=Pin(7), freq=100000)
SENSOR_ADDR = 0x7F  # Adresse I2C du XDB401


def read_xdb401(fullscale_bar=10):
  # 1. Envoyer la commande 0x0A au registre 0x30 pour lancer l'acquisition
  # (Écriture de 2 octets : [registre, commande])
  i2c.writeto(SENSOR_ADDR, b"\x30\x0A")

  # 2. Attendre que la conversion soit finie (environ 50 ms)
  time.sleep_ms(50)

  # 3. Lire les registres de données (de 0x06 à 0x0A, soit 5 octets)
  # D'après la doc : 0x06, 0x07, 0x08 pour la pression (24-bit) et 0x09, 0x0A pour la température (16-bit)
  i2c.writeto(SENSOR_ADDR, b"\x06")
  data = i2c.readfrom(SENSOR_ADDR, 5)

  # Extraction des octets
  p_high = data[0]
  p_mid = data[1]
  p_low = data[2]
  t_high = data[3]
  t_low = data[4]

  # --- Calcul de la Pression ---
  # Pression ADC 24 bits (m = x * 2^16 + y * 2^8 + z)
  m = (p_high << 16) | (p_mid << 8) | p_low

  # Gestion du signe (complément à deux sur 24 bits)
  if m > 0x7FFFFF:
    m -= 0x1000000

  # Conversion en valeur physique (en bar, basé sur la pleine échelle du capteur)
  # Formule de la doc : pressure_value = (m - 2^24) / 2^23 * Fullscale (si négatif)
  # ou m / 2^23 * Fullscale (si positif)
  if m < 0:
    pressure = (m / 8388608.0) * fullscale_bar  # 2^23 = 8388608
  else:
    pressure = (m / 8388608.0) * fullscale_bar

  # --- Calcul de la Température ---
  # Température 16 bits (n = a * 2^8 + b)
  n = (t_high << 8) | t_low

  # Gestion du signe et conversion en °C selon la documentation
  if n & 0x8000:  # Si le bit de signe est 1 (négatif)
    temp = (n - 65536) / 256.0
  else:
    temp = n / 256.0

  return pressure, temp


# Exemple d'utilisation en boucle
while True:
  try:
    # Remplacer 10 par la pleine échelle de votre modèle (ex: 1, 5, 10, 50 bar)
    pression, temperature = read_xdb401(fullscale_bar=10)
    print(f"Pression: {pression:.2f} bar | Température: {temperature:.2f} °C")
  except Exception as e:
    print(f"Erreur de lecture : {e}")

  time.sleep(1)

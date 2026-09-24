import machine
import onewire
import ds18x20
import time

# Définition de la broche GPIO connectée au signal (ex: GPIO 4)
dat_pin = machine.Pin(8)

# Initialisation du bus 1-Wire et du capteur DS18X20
ds = ds18x20.DS18X20(onewire.OneWire(dat_pin))

# Recherche des capteurs présents sur le bus
roms = ds.scan()
print('Capteur(s) trouvé(s) :', roms)

if not roms:
    print("Aucun capteur DS18B20 détecté ! Vérifiez le câblage.")
else:
    while True:
        # 1. Demander à tous les capteurs de lancer une conversion de température
        ds.convert_temp()
        
        # 2. Attendre au moins 750 ms (temps nécessaire pour la conversion en 12 bits)
        time.sleep_ms(750)
        
        # 3. Lire et afficher la température pour chaque capteur trouvé
        for rom in roms:
            temp = ds.read_temp(rom)
            print(f"Température : {temp:.2f} °C")
            
        time.sleep(2)
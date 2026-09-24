# Mesure de la chauffe

En mode `heat`, `firmware/tools/record_heating.py` active la chauffe, relève `/telemetry` toutes
les 500 ms et écrit les mesures dans `captures/`. Il affiche la température et
la puissance demandée toutes les 10 s. Il s'arrête après 30 s consécutives à
moins de 1 °C de la consigne, ou au bout de 10 minutes, puis désactive la
chauffe. Depuis une machine froide en mode Wi-Fi :

```sh
COFFEEFLOW_HTTP_TOKEN='…' COFFEEFLOW_IP='192.168.2.196' \
  uv run firmware/tools/record_heating.py --mode heat
```

En mode `monitor`, le script vérifie que `heating.enabled` vaut `true`, puis
enregistre pendant la durée demandée sans modifier la configuration. La durée
par défaut est de 60 s. Pour capturer 120 s autour de la consigne :

```sh
COFFEEFLOW_HTTP_TOKEN='…' COFFEEFLOW_IP='192.168.2.196' \
  uv run firmware/tools/record_heating.py --mode monitor --monitor-time 120
```

Pour produire un PNG à partir d'une capture JSON, tracer `elapsed_s` en X,
`telemetry.boiler_temperature_c` en Y gauche et
`telemetry.heating_power_pct` en Y droit :

```sh
CAPTURE_PATH=captures/heating-20260923-230231-829984.json
PLOT_PATH="${CAPTURE_PATH%.json}.png"
uv run --with matplotlib python - "$CAPTURE_PATH" "$PLOT_PATH" <<'PY'
import json, sys
import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt

capture = json.load(open(sys.argv[1]))
samples = capture["samples"]
time_s = [sample["elapsed_s"] for sample in samples]
temperature = [sample["telemetry"].get("boiler_temperature_c") for sample in samples]
power = [sample["telemetry"].get("heating_power_pct") for sample in samples]
fig, left = plt.subplots(figsize=(12, 6))
right = left.twinx()
left.plot(time_s, temperature, color="tab:blue", label="Chaudière")
right.plot(time_s, power, color="tab:orange", label="Puissance demandée")
left.axhline(capture["target_c"], color="tab:green", linestyle="--", label="Consigne")
left.set(xlabel="Temps (s)", ylabel="Température (°C)")
right.set(ylabel="Puissance (%)", ylim=(0, 100))
left.grid(alpha=0.25)
fig.legend(loc="upper center", ncol=3)
fig.tight_layout()
fig.savefig(sys.argv[2], dpi=160)
PY
```

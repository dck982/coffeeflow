#!/bin/sh
# Envoie une image à la Rust A de l'essai 08b par son point d'accès Wi-Fi.
#
# Le Mac rejoint le point d'accès par son interface Wi-Fi et garde sa route
# par défaut (Ethernet) : la session et Internet ne sont pas coupés. Refus si
# la route par défaut passe par le Wi-Fi. À la fin, réussite ou non, le
# Wi-Fi revient à son alimentation de départ ; s'il était allumé, il est
# éteint puis rallumé, pour qu'il rejoigne de lui-même son réseau habituel.
#
# Usage : trial-upload.sh <image.bin>
set -eu

IMAGE=${1:?usage: trial-upload.sh <image.bin>}
SSID=coffeeflow-essai-05
PASSWORD=essai-rust-05
ADDRESS=192.168.71.1
SUBNET=192.168.71.

[ -f "$IMAGE" ] || { echo "image introuvable : $IMAGE" >&2; exit 1; }

WIFI=$(networksetup -listallhardwareports | awk '/Hardware Port: Wi-Fi/ { getline; print $2 }')
[ -n "$WIFI" ] || { echo "pas d'interface Wi-Fi" >&2; exit 1; }
DEFAULT_IF=$(route -n get default 2>/dev/null | awk '/interface:/ { print $2 }')
if [ "$DEFAULT_IF" = "$WIFI" ]; then
  echo "la route par défaut passe par le Wi-Fi ($WIFI) : rejoindre l'essai couperait Internet, refus" >&2
  exit 1
fi

POWER=$(networksetup -getairportpower "$WIFI" | awk '{ print $NF }')
restore() {
  networksetup -setairportpower "$WIFI" off
  if [ "$POWER" = On ]; then
    networksetup -setairportpower "$WIFI" on
  fi
  echo "Wi-Fi ($WIFI) remis : $POWER"
}
trap restore EXIT

networksetup -setairportpower "$WIFI" on
networksetup -setairportnetwork "$WIFI" "$SSID" "$PASSWORD"
i=0
until ipconfig getifaddr "$WIFI" 2>/dev/null | grep -q "^${SUBNET}"; do
  i=$((i + 1))
  [ "$i" -le 30 ] || { echo "pas d'adresse en ${SUBNET}0/24 sur $WIFI après 30 s" >&2; exit 1; }
  sleep 1
done
ROUTE_IF=$(route -n get "$ADDRESS" | awk '/interface:/ { print $2 }')
[ "$ROUTE_IF" = "$WIFI" ] || { echo "$ADDRESS ne passe pas par $WIFI ($ROUTE_IF), refus" >&2; exit 1; }

SIZE=$(stat -f %z "$IMAGE")
echo "envoi de $IMAGE ($SIZE octets) à $ADDRESS par $WIFI"
curl --fail-with-body --silent --show-error --max-time 300 \
  -H 'Expect:' -H 'Content-Type: application/octet-stream' \
  --data-binary @"$IMAGE" \
  -w '\nHTTP %{http_code}, %{time_total} s, %{speed_upload} o/s\n' \
  "http://$ADDRESS/firmware"

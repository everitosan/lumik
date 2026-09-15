#!/usr/bin/env bash
# Descarga el binario de rclone a apps/server/bin/rclone (sin sudo).
# Empaquetado local, análogo a como el desktop empaqueta exiftool.
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN_DIR="$HERE/bin"
DEST="$BIN_DIR/rclone"

if [ -x "$DEST" ]; then
  echo "rclone ya está en $DEST"
  "$DEST" version | head -1
  exit 0
fi

case "$(uname -m)" in
  x86_64)  ARCH=amd64 ;;
  aarch64) ARCH=arm64 ;;
  *) echo "arquitectura no soportada: $(uname -m)"; exit 1 ;;
esac

mkdir -p "$BIN_DIR"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

URL="https://downloads.rclone.org/rclone-current-linux-${ARCH}.zip"
echo "Descargando $URL ..."
curl -fsSL "$URL" -o "$TMP/rclone.zip"
unzip -q "$TMP/rclone.zip" -d "$TMP"
cp "$TMP"/rclone-*/rclone "$DEST"
chmod +x "$DEST"

echo "Instalado en $DEST"
"$DEST" version | head -1

#!/usr/bin/env bash
# Smoke test de rclone (Fase 0): prueba la mecánica real que usará el server
# —levantar `rclone rcd` y copiar un archivo vía la API `operations/copyfile`—
# contra un remote local (alias). No requiere credenciales de nube: valida el
# plumbing (subproceso + rcd + copyfile), que es la pieza más incierta.
# La nube real (gdrive/dropbox/…) se valida en la Fase 3.
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RCLONE="${RCLONE_BIN:-$HERE/bin/rclone}"
ADDR="127.0.0.1:5572"

command -v "$RCLONE" >/dev/null 2>&1 || [ -x "$RCLONE" ] || {
  echo "No encuentro rclone en '$RCLONE'. Corre scripts/install_rclone.sh"; exit 1;
}

DEST_ROOT="$(mktemp -d)"
SRC_DIR="$(mktemp -d)"
CONF="$(mktemp)"
echo "hello lumik $(date -Is)" > "$SRC_DIR/test.txt"
printf '[smoke]\ntype = alias\nremote = %s\n' "$DEST_ROOT" > "$CONF"

"$RCLONE" rcd --rc-addr "$ADDR" --rc-no-auth --config "$CONF" >/tmp/rclone_rcd.log 2>&1 &
RCD_PID=$!
cleanup() { kill "$RCD_PID" 2>/dev/null || true; rm -rf "$DEST_ROOT" "$SRC_DIR" "$CONF"; }
trap cleanup EXIT

# Esperar a que rcd responda.
for _ in $(seq 1 40); do
  if curl -sf -o /dev/null -X POST "http://$ADDR/rc/noop"; then break; fi
  sleep 0.25
done

# Copiar test.txt al remote 'smoke:' vía la API.
RESP="$(curl -sf -X POST "http://$ADDR/operations/copyfile" \
  -H 'Content-Type: application/json' \
  -d "{\"srcFs\":\"$SRC_DIR\",\"srcRemote\":\"test.txt\",\"dstFs\":\"smoke:\",\"dstRemote\":\"test.txt\"}")"
echo "copyfile → $RESP"

if [ -f "$DEST_ROOT/test.txt" ]; then
  echo "✅ SMOKE OK — archivo copiado vía rclone rcd:"
  echo "   $(cat "$DEST_ROOT/test.txt")"
  exit 0
else
  echo "❌ SMOKE FALLÓ — el archivo no llegó al remote. Log:"; cat /tmp/rclone_rcd.log
  exit 1
fi

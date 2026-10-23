#!/usr/bin/env bash
# Genera los clips de la landing desde las grabaciones de Linux y Windows.
# Uso: make_clips.sh DIR_SALIDA [CLIP...]   (sin CLIP genera los cuatro)
set -euo pipefail
M="/home/evesan/Vídeos/ProyectosKDE/2026-10/Lumik/Media"
L="$M/cullin in linux.mp4"; W="$M/culling windows.mp4"
FONT="/home/evesan/Vídeos/ProyectosKDE/2026-10/Lumik/Promo/fonts/Inter-700.ttf"
OUT="$1"; shift
ONLY="${*:-hero-seleccion disco importar teclado}"
CL="crop=1920:1024:0:21"   # Linux: sin barra de título ni panel
CW="crop=1280:682:0:13"    # Windows: sin barra de título ni barra de tareas
CWT="crop=1280:682:0:38"   # Windows con la barra de tareas (escritorio y menú Inicio)

# Etiqueta de velocidad o sistema; con INI y FIN solo se muestra en ese tramo.
lab(){ local en=""; [ -n "${2:-}" ] && en=":enable='between(t,$2,$3)'"
  echo "drawtext=fontfile=$FONT:text='$1':x=w-tw-20:y=h-th-64:fontsize=22:fontcolor=0xe9c349:box=1:boxcolor=0x131313@0.75:boxborderw=8$en"; }

# Cámara por fotogramas clave "T,Z,CX,CY" (segundos del tramo, acercamiento y
# centro en coordenadas del cuadro final de 1280x682). Entre dos claves la
# cámara se mueve con aceleración suave; dos claves iguales la dejan quieta.
cam(){ local -a k=("$@") z cx cy t; local i
  for i in "${!k[@]}"; do IFS=, read -r t[i] z[i] cx[i] cy[i] <<<"${k[i]}"; done
  local ze="${z[0]}" xe="${cx[0]}" ye="${cy[0]}" s e
  for ((i=0; i<${#k[@]}-1; i++)); do
    e="clip((it-${t[i]})/(${t[i+1]}-${t[i]}),0,1)"; s="$e*$e*(3-2*$e)"
    ze+="+(${z[i+1]}-${z[i]})*$s"; xe+="+(${cx[i+1]}-${cx[i]})*$s"; ye+="+(${cy[i+1]}-${cy[i]})*$s"
  done
  echo "zoompan=z='$ze':x='clip(($xe)*iw/1280-iw/zoom/2,0,iw-iw/zoom)':y='clip(($ye)*ih/682-ih/zoom/2,0,ih-ih/zoom)':d=1:s=1280x682:fps=30"
}
# Cámara simple: plano general, acercamiento en 0.3–1.0 s y se queda.
near(){ cam "0,1,$2,$3" "0.3,1,$2,$3" "1,$1,$2,$3"; }

# seg CROP VELOCIDAD -> recorte y nuevo ritmo
seg(){ echo "$1,setpts=(PTS-STARTPTS)/$2,fps=30"; }

enc(){ local name=$1; shift
  case " $ONLY " in *" $name "*) ;; *) return 0;; esac
  ffmpeg -v error -y "$@" -an -c:v libx264 -crf 24 -preset slow -pix_fmt yuv420p -movflags +faststart "$OUT/$name.mp4"
  ffmpeg -v error -y -i "$OUT/$name.mp4" -an -c:v libvpx-vp9 -crf 36 -b:v 0 -row-mt 1 "$OUT/$name.webm"
  ffmpeg -v error -y -sseof -0.1 -i "$OUT/$name.mp4" -frames:v 1 -q:v 3 "$OUT/$name-poster.jpg"
}

# importar: plano general -> acercamiento al resumen (derecha, donde pasa el
# mouse) -> paneo a la izquierda a la barra y el contador mientras importa ->
# resultado con el tiempo total -> se aleja. Tramos: resumen ×2 (0–2.15 s),
# progreso ×40 (2.15–5.1 s), resultado ×1 (5.1–8.6 s).
enc importar -ss 43 -t 4.3 -i "$L" -ss 47.3 -t 118.2 -i "$L" -ss 165.5 -t 3.5 -i "$L" -filter_complex \
 "[0:v]$(seg "$CL" 2)[a];[1:v]$(seg "$CL" 40)[b];[2:v]$(seg "$CL" 1)[c];[a][b][c]concat=n=3:v=1:a=0,$(cam \
   0,1,640,341 0.2,1,640,341 0.9,2,1000,190 2.1,2,1000,190 2.9,2,450,170 6.6,2,450,170 7.4,1,640,341),setsar=1,$(lab '×2' 0 2.15),$(lab '×40' 2.15 5.1),$(lab 'Linux' 5.1 10)"

# disco: proyecto en Linux -> disco expulsado -> en Windows se busca Lumik en
# el menú Inicio (acercado), se abre (alejándose, sin el cuadro blanco de la
# ventana al crearse) y aparece el mismo proyecto.
enc disco -ss 650.6 -t 1.3 -i "$L" -ss 660.3 -t 1.8 -i "$L" -ss 2.4 -t 1.85 -i "$W" -ss 4.7 -t 4.1 -i "$W" -filter_complex \
 "[0:v]$(seg "$CL" 1),$(near 1.7 450 150),setsar=1,$(lab Linux)[a];[1:v]$(seg "$CL" 1),$(near 1.7 450 150),setsar=1,$(lab Linux)[b];[2:v]$(seg "$CWT" 1.25),$(cam \
   0,1,640,341 0.1,1,640,341 0.6,1.6,640,420),setsar=1,$(lab Windows)[c];[3:v]$(seg "$CWT" 1.25),$(cam \
   0,1.6,640,420 0.6,1,640,341),setsar=1,$(lab Windows)[d];[a][b][c][d]concat=n=4:v=1:a=0"

# teclado: calificar, aprobar y tags con autocompletado (Windows), acercado al panel
enc teclado -ss 28.5 -t 8 -i "$W" -ss 41.5 -t 5.5 -i "$W" -filter_complex \
 "[0:v]$(seg "$CW" 1.5),$(near 2.2 1080 470),setsar=1,$(lab '×1.5')[a];[1:v]$(seg "$CW" 1.5),$(near 2.2 1080 470),setsar=1,$(lab '×1.5')[b];[a][b]concat=n=2:v=1:a=0"

# hero-seleccion: plano general -> detalle del panel (estrellas, aprobar, tag)
# -> plano general mientras las fotos cambian (Linux)
enc hero-seleccion -ss 240 -t 14 -i "$L" -filter_complex \
 "[0:v]$(seg "$CL" 2.5),$(cam 0,1,1000,450 0.3,1,1000,450 1,2,1000,450 3.8,2,1000,450 4.5,1,1000,450),setsar=1,$(lab '×2.5')"

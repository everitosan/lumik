# Design

## Context

Ver proposal.md (Why). Estado actual relevante:

- `LandingPage.astro` es una sola página para los dos idiomas (`lang`), con este orden: hero (captura del dashboard) → cómo funciona (4 pasos) → video del tutorial (`YouTubeFacade`) → funciones (3 tarjetas) → capturas (5, con lightbox) → plataformas → roadmap → pie. Los textos viven en `src/i18n/ui.ts` (`en` y `es`, mismas claves).
- La spec exige que el video del tutorial vaya justo después de "cómo funciona", que las plataformas sean las tres con "Disponible" y que el roadmap tenga tres grupos. Este change mantiene las tres cosas.
- La landing ya carga Umami (`umami.evesan.rocks`).
- Material de video: solo hay un clip usable, el de la selección (`hero-seleccion`, 5,9 s, ×1.5), hecho a partir de la grabación de macOS del 2026-10-21. La importación de esa grabación muestra la interfaz anterior a la 0.1.5 y no sirve.

## Goals / Non-Goals

**Goals:**
- Que la estructura quede final antes de grabar: grabar un clip y colocarlo no debe requerir cambiar la página.
- Que la landing se pueda publicar en cualquier momento, con o sin los clips que faltan.

**Non-Goals:**
- Hacer que la página de descargas siga la misma narrativa.
- Un reproductor con controles: los clips no tienen audio ni duran lo suficiente para necesitarlos.

## Decisions

### D1. Orden de la página

```
hero, a pantalla completa (100svh bajo la barra):
  logo centrado arriba
  texto (título, subtítulo, "Descargar gratis") | clip de la selección
  tira de pruebas (gratis · Linux/Windows/macOS · ~150 RAW/min)
cómo funciona (01 importa y firma · 02 revisa · 03 califica · 04 selecciona)
video del tutorial                      <- sin cambios, como exige la spec
pilar: tu disco es tu estudio           <- clip "disco" (Linux -> Windows)
pilar: importa sin miedo                <- clip "importar"
pilar: las manos no dejan el teclado    <- clip "teclado"
capturas (dashboard, ajustes, atajos)
plataformas ("también en Linux")
roadmap
pie
```

Las tres tarjetas de funciones desaparecen: su contenido (visor, teclado, proyectos portátiles) pasa a las secciones de pilar. El pilar 1 (gratis) vive en el hero y la tira; el 5 (Linux), en la tira y en plataformas. Así cada pilar aparece una vez con su prueba, sin repetir secciones.

En escritorio el hero ocupa toda la primera pantalla y contiene la tira de pruebas, para que el mensaje y sus pruebas se vean sin bajar; en pantallas bajas (≤860 px de alto) la marca pasa a una línea y se reduce el aire. Por debajo de 900 px de ancho todo se apila y centra.

Las secciones de pilar alternan el lado del clip (texto a la izquierda, después a la derecha) en escritorio y se apilan, clip debajo del texto, por debajo de 900 px (el mismo corte que ya usa la landing).

### D2. `LoopClip.astro`

```
<LoopClip name="importar" src="/clips/importar" fallback="/screenshot-settings.png" alt={...} />
```

- Con `src`: `<video muted loop playsinline preload="none" poster="{src}-poster.jpg">` con `<source>` WebM y MP4. Un `IntersectionObserver` (margen de 200 px) asigna `preload` y llama a `play()` al acercarse, y `pause()` al salir.
- `prefers-reduced-motion: reduce`: no se reproduce solo; se muestra un botón de play sobre la portada, que también vale con Enter o Espacio.
- Sin `src`: la imagen `fallback` en el mismo marco 16:10 que tendrá el clip, para que colocar el clip no mueva el diseño.
- Sin JavaScript: el `<video>` muestra la portada; no hay reproducción automática.
- En `astro dev` (`import.meta.env.DEV`), un espacio sin clip muestra encima una etiqueta con su `name` ("clip pendiente: importar"). El build de producción no la incluye.

*Alternativa:* reutilizar `YouTubeFacade`. Descartada: un iframe de YouTube por clip pesa más que los clips mismos, muestra la marca y no reproduce en bucle sin interacción.

### D3. Archivos de los clips

En `apps/landing/public/clips/`: `{nombre}.webm`, `{nombre}.mp4` y `{nombre}-poster.jpg`, 1280 px de ancho, 30 fps, sin audio, por debajo de 2 MB cada uno. Los cuatro espacios y sus capturas de respaldo:

| Espacio | Sección | Clip | Respaldo mientras no exista |
|---|---|---|---|
| `hero-seleccion` | Hero | Linux, visor (×2.5): general → detalle del panel → general | `screenshot-viewer.png` |
| `disco` | Tu disco es tu estudio | Linux → disco expulsado → Windows (menú Inicio, se abre Lumik con el proyecto) | `screenshot-dashboard.png` |
| `importar` | Importa sin miedo | Linux, 304 RAW: general → resumen (derecha) → paneo a barra y progreso → general | `screenshot-settings.png` |
| `teclado` | Las manos no dejan el teclado | Windows, estrellas + aprobar + tags (×1.5) | `screenshot-grid.png` |

Los cuatro clips salen de las grabaciones del 2026-10-23 (`cullin in linux.mp4`, `culling windows.mp4`) con `make_clips.sh`, en 1280×682 (sin barra de título ni de tareas); los pilares usan `ratio="1280 / 682"`. La receta de macOS de abajo queda solo como referencia histórica.

El clip del hero se rehace con esta receta si el scratchpad de la sesión ya no existe (fuente: `~/Vídeos/ProyectosKDE/2026-10/Lumik/Media/Grabación de pantalla 2026-10-21 a la(s) 13.04.51.mov`):

```
ffmpeg -ss 257 -t 3.5 -i SRC -ss 268.2 -t 1.3 -i SRC -ss 272.5 -t 4 -i SRC -filter_complex
  "[0:v]crop=2560:1362:0:104[a];[1:v]crop=2560:1362:0:104[b];[2:v]crop=2560:1362:0:104[c];
   [a][b][c]concat=n=3:v=1:a=0,setpts=PTS/1.5,fps=30,scale=1280:-2,
   drawtext=fontfile=Inter-700.ttf:text='×1.5':x=w-tw-20:y=h-th-18:fontsize=22:fontcolor=0xe9c349:box=1:boxcolor=0x131313@0.75:boxborderw=8"
  -an -c:v libx264 -crf 24 -pix_fmt yuv420p -movflags +faststart hero-seleccion.mp4
```

El WebM sale del MP4 con `libvpx-vp9 -crf 36 -b:v 0`, y la portada es su último cuadro.

### D4. Textos

Claves nuevas en `ui.ts`, en los dos idiomas: `hero.*` (reescritas), `proof.free`, `proof.platforms`, `proof.import`, `pillar.disk.*`, `pillar.import.*`, `pillar.keyboard.*` (título y texto), `clip.play`, y los `roadmap.*` que cambian. Se quitan `features.*` y los `screenshots.cap*` de grid y visor. El español no usa "culling"; los dos idiomas dicen "autocompletado de tags".

Textos base (español; el inglés con el mismo sentido):

| Clave | Texto |
|---|---|
| `hero.h1` | Tu disco es tu estudio. |
| `hero.sub` | Selecciona tus sesiones gratis, rápido y a teclado. El proyecto vive en tu disco: conéctalo en cualquier computadora y sigue ahí. |
| `hero.cta.primary` | ↓ Descargar gratis |
| `proof.import` | ~150 RAW por minuto (detalle: varía según el disco y el equipo) |
| `pillar.import.desc` | Vacía la tarjeta sin miedo. Firma y renombra al importar, sin convertir el RAW. Si un archivo falla, el resto sigue y lo reintentas con un clic. |
| `roadmap.later.backup` | Respalda todo, entrega solo lo mejor: respaldo en tu propia nube y galería para tu cliente |
| `footer.copy` | Gratis para fotógrafos profesionales. Creado en 2026 por |

### D5. Eventos de Umami

Cada enlace a descargas lleva `data-umami-event="download"` y `data-umami-event-location` con su lugar: `nav`, `hero` y `platforms`. Umami los registra sin código propio. El de GitHub lleva `data-umami-event="github"`.

## Risks / Trade-offs

- [El clip del hero es de macOS, con mouse y ritmo de explicación] → Es un borrador; el espacio `hero-seleccion` se reemplaza por el clip a teclado sin tocar la página.
- [La cifra de importación depende del disco y del equipo] → Se da como promedio redondeado hacia abajo (304 RAW en 1 min 57 s ≈ 156/min, en la grabación del clip `importar`: 0.1.6, Linux, origen en disco mecánico, 2026-10-23) y el detalle aclara que varía según el disco y el equipo.
- [Reproducción automática en móviles] → `muted` + `playsinline` es lo que iOS y Android exigen para reproducir solos; si el navegador la bloquea, queda la portada.
- [La landing tiene cambios sin commit de `landing-tutorial-videos`] → La primera tarea comprueba que estén en un commit antes de tocar los mismos archivos.

## Migration Plan

Sin migración. Se publica como la landing actual. Revertir es volver a la versión anterior de `LandingPage.astro` y `ui.ts`.

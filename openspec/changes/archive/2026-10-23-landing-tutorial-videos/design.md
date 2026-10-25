# Design

## Context

La landing tiene dos páginas, `LandingPage.astro` y `DownloadsPage.astro`, y cada una recibe `lang`. Los textos están en `src/i18n/ui.ts` (`en` y `es`). Hoy no hay ningún componente `.astro` compartido entre las páginas. Los scripts de cliente van en línea, con `define:vars`, y no usan ningún framework. El sitio no tiene CSP. En la pestaña de macOS, la guía de primera apertura es un bloque `.install-guide` de HTML estático, y los paneles se muestran y ocultan con `style.display`.

Los dos videos tienen miniaturas `maxresdefault.jpg` (1280×720, 16:9) en `i.ytimg.com`.

## Goals / Non-Goals

**Goals:**
- Un único componente de fachada que usen las dos páginas.
- Que una visita que no reproduce nada no cargue los ~500 KB del reproductor de YouTube ni reciba sus cookies.

**Non-Goals:**
- Una librería de terceros (`lite-youtube-embed` u otra).
- Que la miniatura se sirva desde el sitio en lugar de `i.ytimg.com`.

## Decisions

### Componente `YouTubeFacade.astro`
Props: `id`, `title`, `playLabel`, `note` (opcional) y `lang`. Genera un `<a class="yt-facade" href="https://www.youtube.com/watch?v={id}">` con la miniatura `maxresdefault.jpg` (`loading="lazy"`, `alt` = título) y un ícono de play encima. Usa `aspect-ratio: 16 / 9`, `width: 100%`, el radio y el borde de `.screenshot-frame`.

El componente lleva un `<script>` procesado por Astro, no `is:inline`. Así Astro lo incluye una sola vez por página aunque haya varias instancias. El script intercepta el clic en `.yt-facade`, hace `preventDefault` y reemplaza el enlace por un `<iframe src="https://www.youtube-nocookie.com/embed/{id}?autoplay=1&rel=0">` con `allow="autoplay; encrypted-media; picture-in-picture; fullscreen"` y `title`.

Al ser un `<a>`, Enter abre el video sin código extra y, sin JavaScript, lleva a YouTube. Espacio se maneja en el script con un `keydown`, porque en un enlace no lo dispara el navegador. Se puede usar `role="button"` para que los lectores de pantalla lo anuncien como botón.

*Alternativa*: un `<button>` con `data-id`. Se descartó porque sin JavaScript no hace nada.

*Alternativa*: el iframe directo con `loading="lazy"`. Se descartó porque la sección del video de uso está cerca del inicio de la página, así que el lazy loading lo cargaría casi siempre.

### `maxresdefault` como miniatura
Es 16:9 exacto y se ve nítida a 1200 px. `hqdefault` es 4:3 con franjas negras. Las dos miniaturas existen hoy. Si en el futuro un video no tuviera `maxres`, YouTube responde con una imagen gris de 120 px. El `onerror` no sirve para detectarlo (la respuesta es 200), así que el riesgo queda cubierto por la verificación manual de la tarea.

### Datos del video en `ui.ts`
Claves nuevas: `video.usage.id`, `video.usage.title`, `video.usage.sub`, `video.macos.id`, `video.macos.title`, `video.play` y `video.audioNote`. El ID es igual en `en` y `es` por ahora. `video.audioNote` vale "Audio in Spanish" en `en` y cadena vacía en `es`, y el componente no pinta la nota cuando viene vacía. Así la regla de cuándo mostrar la nota vive en las traducciones: si mañana hay un video en inglés, se cambia el ID y la nota se vacía.

*Alternativa*: una prop `audioLang` y comparar con `lang`. Es más explícita, pero obliga a tocar el componente para cada video nuevo.

### Ubicación
- **Landing**: una `<section class="video" id="video">` entre `#workflow` y `#features`, con `section-label`, `section-title` y `section-sub`, igual que las demás secciones. El ancho máximo es de unos 960 px, para que el video no ocupe los 1200 px. No se agrega al nav: ya está pegada a *workflow*.
- **Downloads**: dentro de `.install-guide` de macOS, después del título y el requisito y antes de los pasos, para que el video quede como la forma principal y el texto como referencia. Como el panel puede estar oculto (`display:none`) al cargar, `loading="lazy"` evita bajar la miniatura si el visitante nunca abre la pestaña de macOS.

## Risks / Trade-offs

- [Se quita o se hace privado un video de YouTube] → La fachada sigue mostrando la miniatura en caché o una imagen gris. Al pulsar play, el iframe muestra el error de YouTube. Se revisa cuando se revise la landing; no hay detección automática.
- [El autoplay lo bloquea el navegador] → El usuario ya hizo clic, así que los navegadores lo permiten. Si no, queda el botón de play del propio reproductor: un clic extra, sin roturas.
- [La miniatura de `i.ytimg.com` sigue siendo una petición a Google] → Es aceptable y es lo que pide la spec. No deja cookies.

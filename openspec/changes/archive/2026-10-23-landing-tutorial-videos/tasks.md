# Tasks

## 1. Fachada de YouTube

- [x] 1.1 En `ui.ts` (`en` y `es`), agregar `video.usage.id`, `video.usage.title`, `video.usage.sub`, `video.macos.id`, `video.macos.title`, `video.play` y `video.audioNote` ("Audio in Spanish" en `en`, vacía en `es`). Verificar con `grep` que cada clave existe en los dos idiomas y que `pnpm --filter @lumik/landing build` compila.
- [x] 1.2 Crear `apps/landing/src/components/YouTubeFacade.astro` según el design: un `<a>` a `youtube.com/watch` con la miniatura `maxresdefault` lazy en 16:9, el ícono de play, la nota opcional y un `<script>` procesado que, al hacer clic o al pulsar Espacio, lo reemplaza por el iframe de `youtube-nocookie.com` con autoplay. Verificar en `dev` que hay un solo `<script>` del componente por página aunque haya dos instancias.

## 2. Video de uso en la landing

- [x] 2.1 En `LandingPage.astro`, agregar `<section class="video" id="video">` entre `#workflow` y `#features`, con label, título y subtítulo traducidos, y la fachada del video de uso a un ancho máximo de unos 960 px. Verificar en `/` y `/es/`: la sección aparece después de *workflow*, `/` muestra "Audio in Spanish" y `/es/` no tiene nota.
- [x] 2.2 Verificar en DevTools (Network), al cargar `/` sin pulsar play: la única petición a YouTube es `i.ytimg.com/.../maxresdefault.jpg` y no hay ningún iframe. Al pulsar play con el ratón, con Enter y con Espacio, el reproductor de `youtube-nocookie.com` reproduce el video. Con JavaScript deshabilitado, el clic abre YouTube.

## 3. Video de instalación en macOS

- [x] 3.1 En `DownloadsPage.astro`, insertar la fachada del video de macOS dentro de `.install-guide` de macOS, después de `dl.macos.req` y antes de los pasos. Verificar en `/downloads` y `/es/downloads` que el video aparece solo en la pestaña de macOS, con la nota en inglés y sin nota en español, y que se reproduce al pulsar play.
- [x] 3.2 Verificar que, con `api.github.com` bloqueado en DevTools, la guía de macOS sigue mostrando el video y el botón de copiar el comando sigue funcionando.

## 4. Verificación integral

- [x] 4.1 Correr `pnpm --filter @lumik/landing build` y revisar con `preview` las cuatro páginas (`/`, `/es/`, `/downloads`, `/es/downloads`) en escritorio y a 375 px: los videos en 16:9, sin scroll horizontal y con la miniatura nítida.

## Workflow follow-up

- Publicar la landing.
- Archivar el change.
- Cuando haya versiones en inglés de los videos, cambiar `video.*.id` en `en` y vaciar `video.audioNote`.

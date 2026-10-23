# Tasks

## 1. Preparación

- [x] 1.1 Comprobar con `git status apps/landing openspec/specs/landing-site` que los cambios de `landing-tutorial-videos` (videos tutoriales, `YouTubeFacade.astro`) ya están en un commit. Si no, pedir al usuario que los confirme antes de seguir, para no mezclar los dos trabajos.

## 2. Clips

- [x] 2.1 Crear `src/components/LoopClip.astro` (D2): video en bucle sin audio con portada y fuentes WebM/MP4, carga y reproducción con `IntersectionObserver`, pausa con botón de play si `prefers-reduced-motion`, captura de respaldo en el mismo marco cuando no hay `src`, y etiqueta "clip pendiente: {name}" solo en `astro dev`. Verificar con `pnpm --filter @lumik/landing build` y abriendo `pnpm --filter @lumik/landing dev` con y sin reducir movimiento en el navegador.
- [x] 2.2 Copiar el clip del hero a `public/clips/` (`hero-seleccion.mp4`, `.webm`, `-poster.jpg`), desde el scratchpad o rehecho con la receta de D3. Verificar con `ffprobe` que dura unos 6 s, no tiene audio, mide 1280 px de ancho y cada archivo pesa menos de 2 MB.

## 3. Página

- [x] 3.1 Reescribir en `src/i18n/ui.ts`, en inglés y en español, los textos de D4: hero, tira de pruebas, pasos de cómo funciona (el 01 empieza por importar), secciones de pilar, plataformas, roadmap y pie; quitar `features.*` y los pies de captura de grid y visor. Verificar con `pnpm --filter @lumik/landing build` (sin claves faltantes) y con `grep -ri "open source\|código abierto\|culling\|sugerencias" src` sin resultados en los textos.
- [x] 3.2 Reorganizar `LandingPage.astro` en el orden de D1: hero con `LoopClip` del hero, tira de pruebas, cómo funciona, video del tutorial sin cambios, tres secciones de pilar con su `LoopClip` (`disco`, `importar`, `teclado` sin `src`, con su respaldo), galería de tres capturas, plataformas, roadmap y pie. Verificar en `dev` el orden en `/` y `/es/`, que los espacios pendientes muestran su etiqueta, y a 375 px que no hay scroll horizontal.
- [x] 3.3 Agregar los eventos de Umami de D5 a los enlaces de descargas (`nav`, `hero`, `platforms`) y al de GitHub. Verificar en el HTML del build que cada enlace lleva `data-umami-event` y su `data-umami-event-location`.

## 4. Verificación

- [x] 4.1 Con `pnpm --filter @lumik/landing build && pnpm --filter @lumik/landing preview`, revisar en el navegador `/` y `/es/`: los escenarios de la spec delta (mensaje principal, pruebas, pilares, clips, medición) y los requisitos actuales que siguen (video del tutorial tras cómo funciona, plataformas "Disponible", roadmap con tres grupos, nota de audio en inglés). Comprobar que el build de producción no muestra la etiqueta de clip pendiente y que el clip del hero solo se descarga al cargar la página, y los demás no existen todavía. Correr `openspec validate landing-campaign-narrative --strict`.
- [ ] 4.2 Revisión del usuario con `dev`, ya con los clips finales colocados (ver el follow-up): aprobar textos, orden y clips.

## Workflow follow-up

- Grabar los clips `disco` (Linux a Windows, empezando con Lumik ya instalado), `importar` (Linux, 0.1.6, con un fallo y "Reintentar") y `teclado` (solo atajos), y reemplazar el borrador de `hero-seleccion`. Colocarlos en `public/clips/` y pasar `src` a su `LoopClip`, sin cambiar la página.
- Archivar el change con `/opsx:archive` cuando la landing esté publicada.

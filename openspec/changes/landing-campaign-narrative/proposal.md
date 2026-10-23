# Proposal

## Why

La campaña de lanzamiento se apoya en cinco pilares ("Lumik — Campaña de lanzamiento"), pero la landing no los cuenta. No dice que Lumik es gratis, no muestra la importación y deja la idea central (el proyecto vive en tu disco) en la última frase del hero. Además, la selección y la importación se explican con capturas fijas, que no muestran ni la velocidad ni que un fallo no tumba el lote. Y el pie afirma "open source" aunque el repositorio no tiene licencia.

## What Changes

- **Hero nuevo**: título "Tu disco es tu estudio" / "Your disk is your studio", subtítulo con gratis y portabilidad, botón "Descargar gratis". En lugar de la captura del dashboard va un clip en bucle de la selección.
- **Tira de pruebas** dentro del hero, bajo el mensaje y el clip: gratis, sin suscripción · Linux, Windows y macOS · ~150 RAW por minuto al importar (según el disco y el equipo).
- **"Cómo funciona"** empieza por importar y firmar. El video del tutorial sigue justo después.
- **Tres secciones de pilar** en lugar de las tarjetas de funciones: "Tu disco es tu estudio", "Importa sin miedo" y "Las manos no dejan el teclado". Cada una tiene un espacio para su clip; mientras el clip no exista, se muestra la captura que mejor lo representa.
- **Clips en bucle**: sin audio, con portada, cargados al llegar a su sección y en pausa para quien pide menos movimiento. La aceleración se marca en el propio clip.
- **Galería de capturas** solo con lo estático: dashboard, ajustes de importación y atajos.
- **Plataformas**: el texto destaca que Lumik también corre en Linux; las tres siguen "Disponible".
- **Roadmap**: "Próximamente" incluye los autotags locales con búsqueda en todos los proyectos; "Más adelante" dice "Respalda todo, entrega solo lo mejor" (respaldo y galería para el cliente).
- **Textos**: el pie deja de decir "open source"; "sugerencias de tags" pasa a "autocompletado de tags"; "culling" se traduce en español.
- **Medición**: los botones de descarga registran un evento de Umami con su ubicación.
- **Revisión**: en `astro dev`, cada espacio de clip sin video muestra qué clip va ahí. En el build de producción no aparece.

### Fuera de alcance

- Grabar los clips de importación, del disco (Linux a Windows) y de selección a teclado: se graban después del rediseño y se colocan sin cambiar la estructura.
- La página de descargas.
- Agregar una licencia al repositorio.

## Capabilities

### New Capabilities

_Ninguna._

### Modified Capabilities

- `landing-site`: requisitos nuevos para el mensaje principal, las secciones de pilar con su prueba, los clips en bucle y la medición de descargas. Los requisitos actuales (plataformas, roadmap, video del tutorial tras "cómo funciona", carga diferida, nota de audio) se mantienen.

## Impact

- `apps/landing/src/components/LandingPage.astro`: estructura y estilos de las secciones.
- `apps/landing/src/components/LoopClip.astro` (nuevo): clip en bucle con portada, carga diferida, menos movimiento y marca de revisión en desarrollo.
- `apps/landing/src/i18n/ui.ts`: textos nuevos en inglés y español; se quitan los que dejan de usarse.
- `apps/landing/public/clips/`: el clip del hero (`hero-seleccion.mp4`, `.webm` y portada).
- Sin cambios en la página de descargas, en `YouTubeFacade.astro` ni en otras apps.

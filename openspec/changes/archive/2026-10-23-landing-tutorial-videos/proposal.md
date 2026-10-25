# Proposal

## Why

Hay dos videos nuevos en YouTube: "Tutorial de Lumik app" (`ZtPCxeDthUQ`), que muestra el flujo de uso, e "Instala Lumik en MacOS" (`9ZA1Ri1a8Hw`), que muestra cómo abrir la beta sin notarizar. La landing solo tiene texto y capturas. El paso de Gatekeeper es justo donde más se atora un usuario de Mac, y un video lo explica mejor que los pasos escritos.

## What Changes

- **Video de uso en la landing**: una sección nueva justo después de *workflow* en `/` y `/es/` incrusta "Tutorial de Lumik app".
- **Video de instalación en macOS**: la guía de primera apertura de la pestaña de macOS en `/downloads` y `/es/downloads` incrusta "Instala Lumik en MacOS". Sigue la misma condición que la guía: está mientras el paquete no esté notarizado.
- **Carga diferida (fachada)**: cada video se muestra como su miniatura con un botón de play. El reproductor de YouTube (`youtube-nocookie.com`) se carga solo cuando el visitante pulsa play. Antes de eso no se pide nada a YouTube aparte de la miniatura.
- **Audio en español**: los dos videos se muestran en los dos idiomas. En la versión en inglés llevan una nota visible de que el audio está en español.
- **ID del video por idioma**: el ID de cada video vive en las traducciones, para que más adelante una versión en inglés apunte a su propio video sin tocar el componente.

### Fuera de alcance

- Grabar o subir versiones en inglés, o subtítulos.
- Eventos de analytics (Umami) al reproducir.
- Videos para Linux o Windows.

## Capabilities

### New Capabilities

Ninguna.

### Modified Capabilities

- `landing-site`: requisito nuevo para el video de uso en la landing y su carga diferida. La "Guía de primera apertura en macOS" pasa a incluir el video de instalación.

## Impact

- `apps/landing/src/components/LandingPage.astro`: sección del video de uso.
- `apps/landing/src/components/DownloadsPage.astro`: video dentro de la guía de macOS.
- Un componente nuevo de fachada de YouTube en `apps/landing/src/components/`, compartido por las dos páginas.
- `apps/landing/src/i18n/ui.ts`: títulos, nota de audio, etiqueta del botón de play e IDs de video, en `en` y `es`.
- Sin dependencias nuevas, sin cambios en otras apps.

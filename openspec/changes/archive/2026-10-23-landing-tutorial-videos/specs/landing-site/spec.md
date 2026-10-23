# Spec Delta

## ADDED Requirements

### Requirement: Video de uso en la landing
La landing SHALL mostrar, justo después de la sección de cómo funciona, el video de YouTube "Tutorial de Lumik app" (`ZtPCxeDthUQ`), en inglés y en español, con un título y una línea de contexto traducidos.

#### Scenario: Landing en español
- **WHEN** se abre `/es/`
- **THEN** después de la sección de cómo funciona aparece el video de uso con su título en español

#### Scenario: Landing en inglés
- **WHEN** se abre `/`
- **THEN** después de la sección de cómo funciona aparece el mismo video de uso con su título en inglés

### Requirement: Carga diferida de videos
Cada video incrustado SHALL mostrarse primero como su miniatura con un botón de play accesible por teclado. El reproductor de YouTube SHALL cargarse, desde `youtube-nocookie.com`, solo cuando el visitante pulsa play, y SHALL empezar a reproducir en ese momento. Antes de eso la página SHALL pedir a YouTube solo la miniatura. El video SHALL ocupar el ancho disponible en proporción 16:9, sin scroll horizontal a 375 px.

#### Scenario: Carga de la página
- **WHEN** se abre una página con un video y el visitante no pulsa play
- **THEN** la única petición a dominios de YouTube es la de la miniatura y no se carga ningún iframe

#### Scenario: Reproducir
- **WHEN** el visitante pulsa play, con el ratón o con Enter o Espacio
- **THEN** la miniatura se reemplaza por el reproductor de `youtube-nocookie.com` y el video empieza a reproducirse

#### Scenario: Sin JavaScript
- **WHEN** JavaScript está deshabilitado
- **THEN** el botón de play funciona como enlace que abre el video en YouTube

### Requirement: Nota de audio en español
Mientras un video solo tenga audio en español, la versión en inglés del sitio SHALL mostrar junto a ese video una nota visible de que el audio está en español. La versión en español SHALL omitir esa nota.

#### Scenario: Visitante en inglés
- **WHEN** se abre `/` o `/downloads` en la pestaña de macOS
- **THEN** junto al video se lee que el audio está en español

#### Scenario: Visitante en español
- **WHEN** se abre `/es/` o `/es/downloads` en la pestaña de macOS
- **THEN** el video aparece sin nota de idioma

## MODIFIED Requirements

### Requirement: Guía de primera apertura en macOS
Mientras el paquete de macOS no esté notarizado, la pestaña de macOS SHALL mostrar, aunque no carguen los releases, el sistema mínimo (macOS 12 Monterey) y cómo abrir la app la primera vez: copiarla a Aplicaciones y quitar la cuarentena con `xattr -dr com.apple.quarantine /Applications/Lumik.app`, o usar "Abrir igualmente" en Privacidad y seguridad. El comando SHALL poder copiarse con un botón. La guía SHALL incluir el video de YouTube "Instala Lumik en MacOS" (`9ZA1Ri1a8Hw`).

#### Scenario: Copiar el comando
- **WHEN** el visitante pulsa el botón de copiar junto al comando `xattr`
- **THEN** el comando completo queda en el portapapeles y el botón confirma la copia

#### Scenario: Error al cargar releases
- **WHEN** la API de GitHub no responde
- **THEN** la pestaña de macOS muestra el error con el enlace a los releases de GitHub y la guía de primera apertura sigue visible

#### Scenario: Video de instalación
- **WHEN** el visitante abre la pestaña de macOS
- **THEN** la guía de primera apertura muestra el video de instalación en macOS, y las pestañas de Linux y Windows no lo muestran

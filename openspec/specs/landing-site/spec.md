# landing-site Specification

## Purpose
Define qué muestra el sitio público de Lumik: las plataformas que anuncia, cómo la página de descargas presenta los paquetes publicados en los releases de GitHub para cada sistema y qué guía de instalación acompaña a cada uno.

## Requirements

### Requirement: Descargas por sistema
La página de descargas SHALL tener una pestaña por sistema de escritorio: Linux, Windows y macOS. Cada pestaña SHALL listar los releases publicados (no borradores) de GitHub, del más reciente al más antiguo. En cada release SHALL mostrar solo los paquetes de su sistema: `.deb`, `.AppImage` y `.rpm` en Linux; `.exe` y `.msi` en Windows; `.dmg` en macOS. Cada paquete SHALL mostrar nombre, descripción, tamaño y un botón para descargarlo.

#### Scenario: Pestaña de macOS
- **WHEN** el visitante abre la pestaña de macOS y el release `v0.1.4-beta` tiene `Lumik_0.1.4-beta_universal.dmg`
- **THEN** la pestaña muestra ese release con el `.dmg`, descrito como imagen de disco para macOS, y su botón de descarga

#### Scenario: Paquetes de otro sistema
- **WHEN** un release tiene `.deb`, `.AppImage`, `.exe` y `.dmg`
- **THEN** la pestaña de Windows muestra solo el `.exe` y la de Linux solo el `.deb` y el `.AppImage`

### Requirement: Releases sin paquetes para el sistema
Cada pestaña SHALL ocultar los releases que no tengan ningún paquete de su sistema. El primer release que quede en la lista de la pestaña SHALL llevar la marca "latest". Si no queda ninguno, la pestaña SHALL mostrar un aviso de que todavía no hay paquetes para ese sistema.

#### Scenario: Releases anteriores a macOS
- **WHEN** los releases publicados son `v0.1.4-beta` (con `.dmg`), `v0.1.2-beta` y `v0.1.1-beta` (sin `.dmg`)
- **THEN** la pestaña de macOS muestra solo `v0.1.4-beta`, marcado como "latest"

#### Scenario: Ningún release con paquetes del sistema
- **WHEN** ningún release publicado tiene un `.dmg`
- **THEN** la pestaña de macOS muestra el aviso de que no hay paquetes para macOS y ninguna tarjeta de release

### Requirement: Pestaña según el sistema del visitante
Al cargar la página de descargas SHALL abrirse la pestaña del sistema que se detecta en el navegador del visitante: Windows, macOS o, en cualquier otro caso, Linux. El subtítulo de la página SHALL nombrar el sistema detectado. El visitante SHALL poder cambiar de pestaña libremente.

#### Scenario: Visitante desde una Mac
- **WHEN** se abre la página de descargas desde Safari en macOS
- **THEN** la pestaña activa es macOS y el subtítulo indica que se detectó macOS

#### Scenario: Visitante desde Linux
- **WHEN** se abre la página desde Firefox en Linux
- **THEN** la pestaña activa es Linux

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

### Requirement: Guía de primera apertura en Windows
Mientras el instalador de Windows no esté firmado, la pestaña de Windows SHALL mostrar una nota, aunque no carguen los releases, que explique que SmartScreen puede avisar al abrirlo y cómo continuar: "Más información" y después "Ejecutar de todas formas".

#### Scenario: Pestaña de Windows
- **WHEN** el visitante abre la pestaña de Windows
- **THEN** ve la nota de SmartScreen encima de la lista de releases

#### Scenario: Otras pestañas
- **WHEN** el visitante abre la pestaña de Linux o la de macOS
- **THEN** la nota de SmartScreen no aparece

### Requirement: Error al cargar releases
Si los releases no se pueden obtener de GitHub, cada pestaña SHALL mostrar un aviso de error con un enlace a la página de releases del repositorio.

#### Scenario: Límite de la API
- **WHEN** la API de GitHub responde con un error
- **THEN** la pestaña activa muestra el aviso con el enlace a `github.com/everitosan/lumik/releases`

### Requirement: Plataformas anunciadas
La landing SHALL anunciar como plataformas solo las que tienen un paquete descargable, todas con el mismo estado "Disponible". Que el producto está en beta SHALL indicarse una sola vez en el texto de la sección y no como estado de una plataforma. Android y el iPad SHALL quedar fuera de la sección de plataformas hasta que tengan un paquete descargable.

#### Scenario: Sección de plataformas
- **WHEN** se abre la landing en inglés o en español
- **THEN** la sección de plataformas muestra Linux, Windows y macOS como disponibles, su texto indica que Lumik está en beta, y no menciona Android ni el iPad

### Requirement: Roadmap público
La landing SHALL mostrar un roadmap con tres grupos: disponible ahora, próximamente y más adelante. Solo SHALL estar en "disponible ahora" lo que se puede usar en un paquete publicado. Lo que existe en código pero no está desplegado SHALL ir en "más adelante". Android SHALL quedar fuera del roadmap hasta que tenga un paquete descargable.

#### Scenario: Respaldo aún no desplegado
- **WHEN** el respaldo en la nube y el portal web existen en código pero el server no está desplegado
- **THEN** el roadmap los lista en "más adelante" y no en "disponible ahora"

#### Scenario: macOS publicado
- **WHEN** se abre el roadmap con la 0.1.4-beta publicada
- **THEN** "disponible ahora" incluye macOS junto a Linux y Windows, y "próximamente" incluye macOS firmado y notarizado

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

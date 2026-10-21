# Spec Delta

## Purpose

Define qué muestra el sitio público de Lumik: las plataformas que anuncia, cómo la página de descargas presenta los paquetes publicados en los releases de GitHub para cada sistema y qué guía de instalación acompaña a cada uno.

## ADDED Requirements

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
Mientras el paquete de macOS no esté notarizado, la pestaña de macOS SHALL mostrar, aunque no carguen los releases, el sistema mínimo (macOS 12 Monterey) y cómo abrir la app la primera vez: copiarla a Aplicaciones y quitar la cuarentena con `xattr -dr com.apple.quarantine /Applications/Lumik.app`, o usar "Abrir igualmente" en Privacidad y seguridad. El comando SHALL poder copiarse con un botón.

#### Scenario: Copiar el comando
- **WHEN** el visitante pulsa el botón de copiar junto al comando `xattr`
- **THEN** el comando completo queda en el portapapeles y el botón confirma la copia

#### Scenario: Error al cargar releases
- **WHEN** la API de GitHub no responde
- **THEN** la pestaña de macOS muestra el error con el enlace a los releases de GitHub y la guía de primera apertura sigue visible

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

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
Mientras Apple no verifique el paquete de macOS, la pestaña de macOS SHALL mostrar, aunque no carguen los releases, el sistema mínimo (macOS 12 Monterey) y cómo abrir la app la primera vez: copiarla a Aplicaciones y quitar la cuarentena con `xattr -dr com.apple.quarantine /Applications/Lumik.app`, o usar "Abrir igualmente" en Privacidad y seguridad. El comando SHALL poder copiarse con un botón. La guía SHALL incluir el video de YouTube "Instala Lumik en MacOS" (`9ZA1Ri1a8Hw`).

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
- **THEN** "disponible ahora" incluye macOS junto a Linux y Windows, y "próximamente" incluye macOS firmado y verificado por Apple

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

### Requirement: Mensaje principal
El hero de la landing SHALL presentar a Lumik como gratis y con el proyecto viviendo en el disco del fotógrafo, en inglés y en español. Su botón principal SHALL llevar a la página de descargas e indicar que la descarga es gratis. Mientras el repositorio no tenga licencia, la landing no SHALL describir a Lumik como open source.

#### Scenario: Hero en español
- **WHEN** se abre `/es/`
- **THEN** el título del hero es "Tu disco es tu estudio", el subtítulo dice que es gratis y que el proyecto vive en el disco, y el botón principal dice "Descargar gratis" y lleva a `/es/downloads`

#### Scenario: Sin licencia
- **WHEN** se abre cualquier página de la landing y el repositorio no tiene archivo de licencia
- **THEN** ningún texto dice "open source" ni "código abierto"

### Requirement: Pruebas en el hero
Dentro del hero, debajo del mensaje y del clip, la landing SHALL mostrar tres pruebas cortas, de modo que en escritorio se vean en la primera pantalla junto con el resto del hero: que es gratis y sin suscripción, que corre en Linux, Windows y macOS, y un promedio de importación de unos 150 RAW por minuto, aclarando que depende del medio de almacenamiento y del equipo.

#### Scenario: Tira de pruebas
- **WHEN** se abre la landing en una pantalla de escritorio
- **THEN** ve las tres pruebas en la primera pantalla, dentro del hero y antes de la sección de cómo funciona

### Requirement: Secciones de pilar
La landing SHALL tener una sección para cada uno de estos pilares, en este orden: "Tu disco es tu estudio", "Importa sin miedo" y "Las manos no dejan el teclado". Cada sección SHALL tener un título, un texto de no más de tres frases y un espacio para su clip. Los textos SHALL describir solo lo que se puede usar en un paquete publicado.

#### Scenario: Importa sin miedo
- **WHEN** se abre la sección "Importa sin miedo"
- **THEN** su texto menciona la firma y el renombrado al importar, que un archivo con fallo no detiene el resto y que se puede reintentar

#### Scenario: Funciones no publicadas
- **WHEN** se leen las secciones de pilar
- **THEN** no mencionan selección con IA, Android, respaldo en la nube ni entrega al cliente

### Requirement: Clips en bucle
Cada clip de la landing SHALL reproducirse sin audio, en bucle y sin controles, y SHALL mostrar una imagen de portada mientras no se reproduce. El clip SHALL cargarse solo cuando su sección se acerca a la pantalla. Un clip acelerado SHALL indicar su factor de aceleración en la imagen.

#### Scenario: Llegar a una sección
- **WHEN** el visitante baja hasta una sección con clip
- **THEN** el clip empieza a reproducirse sin sonido y en bucle

#### Scenario: Carga de la página
- **WHEN** se abre la landing y el visitante no baja de la primera pantalla
- **THEN** solo se descarga el clip del hero, y los de las secciones de pilar todavía no

#### Scenario: Pantalla angosta
- **WHEN** la landing se abre a 375 px de ancho
- **THEN** los clips ocupan el ancho disponible sin scroll horizontal

### Requirement: Clips con menos movimiento
Para el visitante que pide menos movimiento, cada clip SHALL quedar en pausa en su portada con un botón para reproducirlo, que funcione con el ratón y con el teclado.

#### Scenario: Menos movimiento
- **WHEN** el sistema del visitante tiene activado reducir el movimiento
- **THEN** cada clip muestra su portada quieta y un botón de play, y solo se reproduce si el visitante lo pulsa o lo activa con Enter o Espacio

### Requirement: Espacio de clip sin video
Mientras un espacio de clip no tenga video, SHALL mostrar su captura de respaldo en el mismo marco que tendrá el clip. El build de producción no SHALL mostrar ningún texto que anuncie que falta el video.

#### Scenario: Espacio sin clip
- **WHEN** la sección "Importa sin miedo" todavía no tiene clip
- **THEN** muestra su captura de respaldo con el mismo tamaño que tendrá el clip, y el build de producción no muestra ninguna marca de "video pendiente"

### Requirement: Medición de descargas
Cada botón de la landing que lleva a la página de descargas SHALL registrar en la analítica del sitio un evento que indique desde qué parte de la página se pulsó.

#### Scenario: Botón del hero
- **WHEN** el visitante pulsa "Descargar gratis" en el hero
- **THEN** la analítica registra un evento de descarga con la ubicación del hero

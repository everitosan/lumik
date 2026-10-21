# project-management Specification

## Purpose

Permite al fotógrafo organizar cada sesión como un proyecto autocontenido que vive en el disco externo, con su propia base de datos y una estructura de carpetas por fecha.

## Requirements

### Requirement: Crear proyecto en un dispositivo montado
El sistema SHALL crear un proyecto solo en un dispositivo de almacenamiento actualmente montado, identificado por su UUID. El proyecto SHALL recibir un identificador UUID estable y su base de datos `project.db` SHALL residir dentro de la carpeta del proyecto en ese dispositivo, sin depender de la base de datos global.

#### Scenario: Dispositivo montado
- **WHEN** el usuario crea el proyecto "Boda" con fecha de sesión `2024-06-15` en el dispositivo montado en `/mnt/disk`
- **THEN** se crea `/mnt/disk/lumik/2024/06/15_Boda/project.db` con el proyecto y un id UUID

#### Scenario: Dispositivo no montado
- **WHEN** el usuario crea un proyecto en un dispositivo cuyo UUID no aparece entre los montados
- **THEN** la creación falla con un error indicando que el dispositivo no está montado y no se escribe nada en disco

### Requirement: Estructura de carpeta por fecha
La carpeta de un proyecto SHALL ser `{montaje}/lumik/{año}/{mes}/{día}_{slug}`, donde la fecha es la fecha de sesión (formato ISO `YYYY-MM-DD` o EXIF `YYYY:MM:DD`) y el slug es el nombre recortado de espacios con `/` reemplazado por `-`. Si no hay fecha de sesión válida se SHALL usar la fecha actual (UTC).

#### Scenario: Nombre con barra
- **WHEN** se crea el proyecto " XV/Ana " con fecha `2024-03-02`
- **THEN** la carpeta es `lumik/2024/03/02_XV-Ana`

#### Scenario: Sin fecha de sesión
- **WHEN** se crea un proyecto sin fecha de sesión
- **THEN** la carpeta usa el año, mes y día de hoy

### Requirement: Subcarpetas del proyecto
Al crear un proyecto el sistema SHALL crear la subcarpeta `_exported/` dentro de la carpeta del proyecto, destinada a las fotos finales editadas fuera de Lumik.

#### Scenario: Proyecto recién creado
- **WHEN** se crea un proyecto
- **THEN** existe la carpeta `_exported/` vacía junto a `project.db`

### Requirement: Descubrimiento de proyectos en dispositivos
El sistema SHALL listar los proyectos encontrados en los dispositivos montados y SHALL dejar de listar los de dispositivos desmontados o en proceso de expulsión. Cada dispositivo visto SHALL registrarse en el registro global de dispositivos conocidos.

#### Scenario: Conectar un disco con proyectos
- **WHEN** se monta un disco que contiene proyectos Lumik
- **THEN** esos proyectos aparecen en el dashboard sin necesidad de importarlos

#### Scenario: Desconectar el disco
- **WHEN** el disco se desmonta o se está expulsando
- **THEN** sus proyectos dejan de aparecer en el dashboard

### Requirement: Orden del dashboard
El dashboard SHALL agrupar los proyectos en secciones por el año de su fecha de sesión o, si no la tienen, de su fecha de creación. Las secciones y los proyectos de cada sección SHALL ordenarse en la dirección elegida por el usuario. Dentro de una sección, los proyectos SHALL ordenarse por esa misma fecha y desempatarse por fecha de creación en la misma dirección. Los proyectos eliminados lógicamente SHALL quedar fuera del dashboard, y los archivados SHALL mostrarse u ocultarse según la elección del usuario.

#### Scenario: Mezcla de proyectos con y sin fecha
- **WHEN** existe un proyecto sin fecha de sesión creado el 2026-05-20 y proyectos con fecha de sesión en 2026
- **THEN** el proyecto sin fecha aparece en la sección 2026, ordenado por su fecha de creación junto a los demás

#### Scenario: Misma fecha de sesión
- **WHEN** dos proyectos comparten fecha de sesión y el orden de proyectos es de más antiguo a más reciente
- **THEN** aparece primero el creado antes

#### Scenario: Archivados mostrados en orden
- **WHEN** se muestran los archivados y un proyecto archivado es del 2026-03-02, entre dos activos del 2026-01-10 y del 2026-08-10
- **THEN** dentro de la sección 2026 aparece entre ambos, en el orden elegido

### Requirement: Orden configurable del dashboard
El usuario SHALL poder elegir desde el dashboard si las secciones de año se muestran de la más reciente a la más antigua o al revés, y lo mismo para los proyectos dentro de cada año. La elección SHALL aplicarse al momento y guardarse en la configuración de la instalación, no en los discos, de modo que se conserve entre reinicios. En una instalación nueva, los años SHALL ir de más reciente a más antiguo y los proyectos de más antiguo a más reciente.

#### Scenario: Instalación nueva
- **WHEN** el usuario abre el dashboard por primera vez
- **THEN** ve los años de más reciente a más antiguo y, dentro de cada año, los proyectos de más antiguo a más reciente

#### Scenario: Años de más antiguo a más reciente
- **WHEN** hay proyectos de 2024, 2025 y 2026 y el usuario elige ver primero los años más antiguos
- **THEN** las secciones aparecen en el orden 2024, 2025, 2026

#### Scenario: Proyectos más recientes primero
- **WHEN** un año tiene proyectos del 2026-03-02 y del 2026-08-10 y el usuario elige ver primero los proyectos más recientes
- **THEN** dentro de la sección 2026 aparece primero el del 2026-08-10

#### Scenario: Se conserva tras reiniciar
- **WHEN** el usuario cambia el orden y reinicia la app
- **THEN** el dashboard se muestra con el orden elegido

#### Scenario: Independiente de los discos
- **WHEN** el usuario cambia el orden y después conecta otro disco con proyectos
- **THEN** los proyectos de ese disco se muestran con el mismo orden

### Requirement: Archivar proyectos
El usuario SHALL poder archivar y desarchivar un proyecto desde su tarjeta, sin confirmación. El dashboard SHALL ocultar los archivados salvo que el usuario elija mostrarlos con un control que indica cuántos hay y que solo aparece si hay alguno. Esa elección SHALL guardarse en la configuración de la instalación. Archivar no SHALL cambiar nada más del proyecto.

#### Scenario: Archivar un proyecto
- **WHEN** el usuario elige "Archivar" en la tarjeta de un proyecto con los archivados ocultos
- **THEN** el proyecto desaparece del dashboard y el control muestra un archivado más

#### Scenario: Mostrar archivados
- **WHEN** hay proyectos archivados y el usuario activa el control de archivados
- **THEN** los archivados aparecen en su año y en su orden, atenuados y con la etiqueta "Archivado"

#### Scenario: Desarchivar
- **WHEN** el usuario elige "Desarchivar" en la tarjeta de un proyecto archivado
- **THEN** el proyecto vuelve a mostrarse como los demás aunque los archivados estén ocultos

#### Scenario: Preferencia persistente
- **WHEN** el usuario activa el control de archivados y reinicia la app
- **THEN** el dashboard sigue mostrando los archivados

#### Scenario: Sin archivados
- **WHEN** ningún proyecto está archivado
- **THEN** el control de archivados no se muestra

#### Scenario: Estadísticas y búsqueda
- **WHEN** los archivados están ocultos
- **THEN** el contador de proyectos y fotos del dashboard y el buscador no los incluyen

#### Scenario: Proyecto archivado en uso
- **WHEN** el usuario abre un proyecto archivado e importa fotos en él
- **THEN** la importación funciona igual que en un proyecto activo y el proyecto sigue archivado

### Requirement: Información del proyecto en el dashboard
Cada entrada del dashboard SHALL mostrar nombre, fecha de sesión, número de fotos, portada y estado de flujo agregado: `imported` si no hay fotos o alguna está importada sin editar, `edited` si alguna está editada y ninguna sin editar, y `delivered` en otro caso.

#### Scenario: Proyecto con fotos editadas
- **WHEN** todas las fotos de un proyecto están en estado `edited` o `delivered` y al menos una en `edited`
- **THEN** el proyecto se muestra con estado `edited`

### Requirement: Representación visual del estado de flujo
El estado de flujo de cada proyecto en el dashboard SHALL mostrarse como un badge con color, icono y etiqueta propios de cada estado, de modo que el estado sea identificable sin depender solo del color: `imported` en amarillo con un icono de importación y la etiqueta "Importado"; `edited` en azul con un icono de lápiz y la etiqueta "Editado"; `delivered` en gris con un icono de check y la etiqueta "Entregado". Las etiquetas SHALL seguir el idioma de la interfaz.

#### Scenario: Proyecto importado
- **WHEN** un proyecto tiene estado de flujo `imported`
- **THEN** su tarjeta muestra un badge amarillo con el icono de importación y la etiqueta "Importado"

#### Scenario: Proyecto editado
- **WHEN** un proyecto tiene estado de flujo `edited`
- **THEN** su tarjeta muestra un badge azul con el icono de lápiz y la etiqueta "Editado"

#### Scenario: Proyecto entregado
- **WHEN** un proyecto tiene estado de flujo `delivered`
- **THEN** su tarjeta muestra un badge gris con el icono de check y la etiqueta "Entregado"

#### Scenario: Interfaz en inglés
- **WHEN** el idioma de la interfaz es inglés y un proyecto tiene estado `edited`
- **THEN** la etiqueta del badge es "Edited"

#### Scenario: Mismo badge en desktop y portal web
- **WHEN** el mismo proyecto se ve en el dashboard de la app de escritorio y en el del portal web con el mismo estado de flujo
- **THEN** ambos muestran el badge con el mismo color, icono y etiqueta

### Requirement: Portada del proyecto
El usuario SHALL poder elegir una foto del proyecto como portada o quitarla; la portada se muestra con la miniatura de esa foto. Si la miniatura no existe, el proyecto se muestra sin portada.

#### Scenario: Elegir portada
- **WHEN** el usuario marca una foto como portada desde el visor
- **THEN** el dashboard muestra la miniatura de esa foto en la tarjeta del proyecto

#### Scenario: Miniatura ausente
- **WHEN** la miniatura de la foto de portada ya no existe en disco
- **THEN** el proyecto se muestra sin portada, sin error

### Requirement: Editar proyecto
El usuario SHALL poder editar el nombre, la fecha de sesión y la descripción de un proyecto. El nombre y la fecha SHALL ser obligatorios. Al guardar se SHALL actualizar el proyecto y, si cambian el nombre o la fecha, mover su carpeta a `{montaje}/lumik/{año}/{mes}/{día}_{slug}`; sus fotos SHALL seguir abriéndose desde la nueva carpeta. Si ya existe una carpeta con la ruta destino, la edición SHALL fallar sin cambios.

#### Scenario: Renombrado correcto
- **WHEN** el proyecto en `lumik/2024/06/15_Boda` se renombra a "Boda Ana"
- **THEN** la carpeta pasa a `lumik/2024/06/15_Boda Ana` y las fotos siguen abriéndose

#### Scenario: Cambiar la fecha
- **WHEN** el proyecto en `lumik/2024/06/15_Boda` cambia su fecha de sesión a `2024-09-02`
- **THEN** la carpeta pasa a `lumik/2024/09/02_Boda` y los nombres de las fotos no cambian

#### Scenario: Editar la descripción
- **WHEN** el usuario cambia solo la descripción
- **THEN** se guarda la descripción nueva y la carpeta no se mueve

#### Scenario: Nombre vacío
- **WHEN** el nuevo nombre está vacío o solo tiene espacios
- **THEN** la edición se rechaza

#### Scenario: Fecha vacía
- **WHEN** el usuario borra la fecha de sesión
- **THEN** la edición se rechaza

#### Scenario: Colisión de carpeta
- **WHEN** ya existe una carpeta con la ruta destino
- **THEN** la edición falla y el proyecto permanece en su ubicación original y con sus datos anteriores

### Requirement: Fecha de sesión al editar
El campo de fecha SHALL precargarse con la fecha de sesión del proyecto o, si no tiene, con la fecha de su carpeta. Si el proyecto no tenía fecha de sesión y se guarda con la fecha precargada, SHALL seguir sin fecha de sesión. Cualquier otra fecha SHALL guardarse como fecha de sesión.

#### Scenario: Guardar sin tocar la fecha
- **WHEN** un proyecto sin fecha de sesión creado el 2026-10-19 se edita solo para cambiar el nombre
- **THEN** el proyecto sigue sin fecha de sesión y una importación posterior puede inferirla

#### Scenario: Fecha elegida por el usuario
- **WHEN** un proyecto sin fecha de sesión creado el 2026-10-19 se guarda con la fecha `2026-09-30`
- **THEN** su fecha de sesión pasa a ser `2026-09-30`

### Requirement: Renombrar archivos al editar el nombre
Si cambia el nombre y el usuario lo pide, las fotos de `_media/` y `_culled/` cuyo nombre lleve el nombre de proyecto anterior según la regla del renombrado al importar SHALL renombrarse con el nombre nuevo saneado, junto con sus sidecars. Los demás archivos y `_exported/` no SHALL modificarse. Si un renombrado falla, los ya hechos SHALL deshacerse.

#### Scenario: Renombrar archivos
- **WHEN** el proyecto "Boda" se renombra a "Boda Ana" con la opción de renombrar archivos
- **THEN** `2024-06-15__14-30-22__Boda.cr3` pasa a `2024-06-15__14-30-22__Boda_Ana.cr3` y `2024-06-15__14-30-22__Boda-001.cr3` pasa a `2024-06-15__14-30-22__Boda_Ana-001.cr3`

#### Scenario: Archivos no renombrados al importar
- **WHEN** el proyecto tiene la foto `IMG_4821.CR3` y se renombra con la opción de renombrar archivos
- **THEN** `IMG_4821.CR3` conserva su nombre

#### Scenario: Sin la opción
- **WHEN** el proyecto se renombra sin la opción de renombrar archivos
- **THEN** los archivos conservan su nombre y solo cambia la carpeta

#### Scenario: Exportados intactos
- **WHEN** se renombran los archivos de un proyecto con JPEG en `_exported/`
- **THEN** los archivos de `_exported/` conservan su nombre y las fotos ya enlazadas siguen enlazadas

#### Scenario: Fallo a mitad
- **WHEN** falla el renombrado de un archivo después de renombrar otros
- **THEN** los archivos ya renombrados vuelven a su nombre anterior y se informa del error

### Requirement: Actualizar metadatos de las fotos existentes
Si cambian la descripción o el año de la sesión y el usuario lo pide, cada foto del proyecto SHALL pasar a llevar la descripción `{descripción}@{año}`, o ninguna si la descripción queda vacía, con la misma ubicación que al importar (dentro del archivo en Linux y Windows, en el sidecar `.xmp` en Android). El sistema SHALL informar cuántas fotos se actualizaron y cuáles fallaron.

#### Scenario: Corregir la descripción
- **WHEN** el usuario cambia la descripción a "Sesión en la playa" en un proyecto de 2024 con la opción de actualizar metadatos
- **THEN** las fotos del proyecto llevan la descripción `Sesión en la playa@2024`

#### Scenario: Sin la opción
- **WHEN** el usuario cambia la descripción sin la opción de actualizar metadatos
- **THEN** las fotos existentes conservan su descripción y las próximas importaciones usan la nueva

#### Scenario: Fotos que fallan
- **WHEN** algunas fotos no se pueden escribir durante la actualización
- **THEN** el resto se actualiza y el resultado lista las que fallaron

### Requirement: Eliminar proyecto
Eliminar un proyecto SHALL borrar de forma permanente su carpeta y todo su contenido del disco, y retirarlo del dashboard. La operación SHALL requerir que el dispositivo del proyecto esté montado.

#### Scenario: Eliminación confirmada
- **WHEN** el usuario confirma eliminar un proyecto
- **THEN** su carpeta desaparece del disco y el proyecto deja de listarse

### Requirement: Abrir carpeta del proyecto
En plataformas de escritorio el usuario SHALL poder abrir la carpeta del proyecto en el explorador de archivos del sistema operativo.

#### Scenario: Abrir en el explorador
- **WHEN** el usuario elige "abrir carpeta" en Linux o Windows
- **THEN** se abre el explorador del sistema en la carpeta del proyecto

### Requirement: Identidad portable del proyecto
Un proyecto SHALL identificarse por su `project.id`, sin importar en qué disco ni en qué plataforma esté. Ni el proyecto ni sus fotos SHALL depender del UUID del disco donde viven: las fotos nuevas no SHALL guardar ese UUID, y los valores ya guardados no SHALL usarse. Copiar o mover la carpeta de un proyecto a cualquier ruta bajo `lumik/`, en el mismo disco o en otro, SHALL bastar para que se liste y funcione completo desde ahí.

#### Scenario: Mover un proyecto a otro disco
- **WHEN** el usuario copia `lumik/2025/03/29_Chepe/` del disco A al disco B, conecta solo el disco B y abre Lumik
- **THEN** el proyecto aparece en el dashboard con el mismo id, sus fotos y su estado de respaldo, desde el disco B

#### Scenario: Mover un proyecto a otra ruta dentro de `lumik/`
- **WHEN** el usuario copia `lumik/2026/10/10_sesion/` a `lumik/10_sesion/` de otro disco y lo abre en Lumik
- **THEN** el proyecto muestra sus miniaturas y sus previews, y sus fotos se pueden rotar, aprobar, descartar, mandar a la papelera y mostrar en la carpeta

#### Scenario: Proyecto abierto en otra plataforma
- **WHEN** un proyecto importado en Linux se abre en macOS, donde el disco tiene otro UUID
- **THEN** el proyecto y sus fotos se muestran y se editan igual que en Linux

#### Scenario: Importar fotos
- **WHEN** el usuario importa fotos en un proyecto
- **THEN** las fotos nuevas se guardan sin el UUID del disco

### Requirement: Proyecto duplicado en dos discos
Si el mismo proyecto (mismo `project.id`) está en dos discos montados a la vez, el sistema SHALL listarlo una sola vez, desde el disco que descubra primero, y no SHALL avisar del duplicado. Esta es una limitación conocida: qué copia se lista no está garantizado.

#### Scenario: Copia en dos discos conectados
- **WHEN** los discos A y B contienen la misma carpeta de proyecto y ambos están montados
- **THEN** el dashboard muestra el proyecto una sola vez, desde uno de los dos discos

### Requirement: Rutas de fotos relativas al proyecto
El sistema SHALL guardar la ubicación de los archivos de cada foto, el original y el exportado, como ruta relativa a la carpeta del proyecto (la que contiene su `project.db`), y SHALL resolverla con la carpeta donde se descubrió el proyecto. Las rutas guardadas no SHALL incluir el punto de montaje ni la ubicación del proyecto dentro de `lumik/`.

#### Scenario: Foto importada
- **WHEN** se importa `IMG_0001.dng` en el proyecto de `lumik/2026/10/10_sesion/`
- **THEN** la ruta guardada de la foto es `_media/IMG_0001.dng`

#### Scenario: Foto descartada o en la papelera
- **WHEN** el usuario descarta una foto y después manda otra a la papelera
- **THEN** sus rutas guardadas son `_culled/{archivo}` y `_trash/{archivo}`

#### Scenario: Exportado enlazado
- **WHEN** se enlaza `_exported/IMG_0001.jpg` con su foto
- **THEN** la ruta guardada del exportado es `_exported/IMG_0001.jpg`, y es la que se publica en el catálogo

#### Scenario: Mover la carpeta del proyecto
- **WHEN** Lumik mueve la carpeta del proyecto al cambiar su nombre o su fecha
- **THEN** las rutas guardadas de las fotos no cambian y las fotos siguen abriéndose

### Requirement: Conversión de rutas de proyectos existentes
Al abrir un proyecto cuyas fotos guardan rutas en el formato anterior, relativas al punto de montaje, el sistema SHALL convertirlas a rutas relativas al proyecto quitando lo que precede a la primera subcarpeta del proyecto (`_media`, `_culled`, `_trash` o `_exported`). La conversión SHALL ser idempotente y SHALL dejar sin cambios las rutas que no contengan ninguna de esas subcarpetas.

#### Scenario: Proyecto existente
- **WHEN** se abre un proyecto cuyas fotos tienen `lumik/2024/06/15_boda/_media/IMG_0001.cr2`
- **THEN** la ruta pasa a `_media/IMG_0001.cr2` y la foto se abre igual que antes

#### Scenario: Proyecto movido antes de la conversión
- **WHEN** un proyecto importado en `lumik/2026/10/10_sesion/` se copió a `lumik/10_sesion/` con una versión anterior de Lumik y se abre ahora
- **THEN** sus rutas se convierten a `_media/…` y sus previews y su edición vuelven a funcionar

#### Scenario: Abrir dos veces
- **WHEN** se abre un proyecto ya convertido
- **THEN** ninguna ruta cambia

#### Scenario: Ruta no reconocida
- **WHEN** una foto tiene una ruta que no contiene `_media`, `_culled`, `_trash` ni `_exported`
- **THEN** su ruta se conserva sin cambios y el resto de las fotos se convierten

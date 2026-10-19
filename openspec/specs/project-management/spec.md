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
El dashboard SHALL agrupar los proyectos en secciones por el año de su fecha de sesión o, si no la tienen, de su fecha de creación. Las secciones y los proyectos de cada sección SHALL ordenarse en la dirección elegida por el usuario. Dentro de una sección, los proyectos SHALL ordenarse por esa misma fecha y desempatarse por fecha de creación en la misma dirección. Los proyectos archivados o eliminados lógicamente SHALL quedar fuera del dashboard.

#### Scenario: Mezcla de proyectos con y sin fecha
- **WHEN** existe un proyecto sin fecha de sesión creado el 2026-05-20 y proyectos con fecha de sesión en 2026
- **THEN** el proyecto sin fecha aparece en la sección 2026, ordenado por su fecha de creación junto a los demás

#### Scenario: Misma fecha de sesión
- **WHEN** dos proyectos comparten fecha de sesión y el orden de proyectos es de más antiguo a más reciente
- **THEN** aparece primero el creado antes

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

### Requirement: Renombrar proyecto
Renombrar SHALL cambiar el nombre almacenado y mover la carpeta del proyecto conservando el prefijo de día y sustituyendo el slug, actualizando las rutas de todas las fotos. El nombre no SHALL quedar vacío. Si ya existe una carpeta con el nombre destino, el renombrado SHALL fallar sin cambios.

#### Scenario: Renombrado correcto
- **WHEN** el proyecto en `lumik/2024/06/15_Boda` se renombra a "Boda Ana"
- **THEN** la carpeta pasa a `lumik/2024/06/15_Boda Ana` y las fotos siguen abriéndose

#### Scenario: Nombre vacío
- **WHEN** el nuevo nombre está vacío o solo tiene espacios
- **THEN** el renombrado se rechaza

#### Scenario: Colisión de carpeta
- **WHEN** ya existe una carpeta con el nombre destino
- **THEN** el renombrado falla y el proyecto permanece en su ubicación original

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

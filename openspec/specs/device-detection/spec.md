# device-detection Specification

## Purpose

Detecta los discos externos y tarjetas donde viven los proyectos, los identifica de forma estable por UUID aunque cambie su punto de montaje, y permite expulsarlos de forma segura sin cerrar la app.

## Requirements

### Requirement: Detección de dispositivos externos
El sistema SHALL listar los dispositivos de almacenamiento externos montados con UUID de sistema de archivos, nombre, punto de montaje, tipo de sistema de archivos y espacio total y disponible. Los discos del sistema y virtuales SHALL excluirse: raíz y particiones del sistema, filesystems virtuales, loop devices, la unidad de Windows, los volúmenes del sistema de macOS y el almacenamiento interno de Android.

#### Scenario: Disco USB conectado en Linux
- **WHEN** se monta una memoria USB en `/media/usuario/FOTOS`
- **THEN** aparece en la lista con su UUID y su espacio disponible

#### Scenario: Disco USB conectado en macOS
- **WHEN** se monta un disco externo en `/Volumes/KINGSTON`
- **THEN** aparece en la lista con el UUID de volumen que reporta macOS y su espacio disponible

#### Scenario: Disco NTFS montado por macOS
- **WHEN** macOS monta con su driver nativo la partición NTFS `Photos` de un disco externo en `/Volumes/Photos`
- **THEN** aparece en la lista con el UUID de volumen que reporta macOS, marcada como de solo lectura

#### Scenario: Disco del sistema
- **WHEN** se escanean dispositivos en una máquina con `/` y `/home` montados
- **THEN** ninguno de esos puntos de montaje aparece en la lista

#### Scenario: Volúmenes del sistema en macOS
- **WHEN** se escanea en macOS con `/` y los volúmenes de `/System/Volumes/` montados
- **THEN** ninguno de ellos aparece en la lista

#### Scenario: Almacenamiento interno de Android
- **WHEN** se escanea en Android con una tarjeta SD insertada
- **THEN** aparece la tarjeta SD y no el almacenamiento interno

### Requirement: Identidad estable por UUID
Un dispositivo SHALL identificarse por el UUID de su sistema de archivos que reporta el sistema operativo de esa máquina, no por su punto de montaje. Ese UUID no SHALL usarse para identificar proyectos ni fotos. Para nombrarlo solo SHALL escribirse `lumik/.disk.json`, en discos con `lumik/`. Cada dispositivo visto SHALL registrarse en el registro global de dispositivos conocidos con su nombre, última ruta de montaje (solo como pista) y fecha de última vez visto.

#### Scenario: Cambio de punto de montaje
- **WHEN** un disco que se montó en `/media/a` se vuelve a montar en `/media/b`
- **THEN** se reconoce como el mismo dispositivo y sus proyectos se abren desde la nueva ruta

#### Scenario: Mismo disco en dos plataformas
- **WHEN** un disco exFAT que en Linux tiene UUID `ECD0-9D14` se conecta a una Mac, donde macOS reporta `2B7D2056-C914-3E3F-9332-95EC595EE039`
- **THEN** en la Mac el disco se registra con el UUID de macOS, distinto del de Linux, y sus proyectos se listan igual que en Linux

#### Scenario: Conectar un disco no escribe en él
- **WHEN** se conecta un disco sin carpeta `lumik/`
- **THEN** el disco aparece en la lista y no se crea ningún archivo en él

### Requirement: Nombre amigable en Android
En la app de escritorio para Android, cuando un volumen no tiene etiqueta ni `lumik/.disk.json`, el nombre SHALL derivarse del formato de UUID: `XXXX-XXXX` se muestra como "SD Card" y cualquier otro como "USB Drive".

#### Scenario: Tarjeta SD FAT32
- **WHEN** se detecta en Android un volumen con UUID `1A2B-3C4D` sin `lumik/.disk.json`
- **THEN** se muestra con el nombre "SD Card"

#### Scenario: Disco con nombre guardado
- **WHEN** se detecta en Android un volumen con UUID `0D6F-0A43` cuyo `lumik/.disk.json` tiene el nombre "SEAGATE"
- **THEN** se muestra con el nombre "SEAGATE"

### Requirement: Refresco ante conexión y desconexión
En Linux, Windows y macOS la lista de dispositivos y el dashboard SHALL refrescarse en cuanto el sistema operativo notifique un cambio de volúmenes, con un sondeo de respaldo cada 60 segundos. En Android, que no tiene notificación nativa, SHALL sondearse cada 10 segundos.

#### Scenario: Conectar un disco en escritorio
- **WHEN** el usuario conecta un disco en Linux
- **THEN** el disco y sus proyectos aparecen sin esperar al siguiente sondeo

#### Scenario: Conectar un disco en macOS
- **WHEN** el usuario conecta un disco en macOS y el sistema lo monta en `/Volumes/`
- **THEN** el disco y sus proyectos aparecen sin esperar al siguiente sondeo

#### Scenario: Desconectar un disco en macOS
- **WHEN** el usuario expulsa un disco desde Finder o lo desconecta
- **THEN** el disco y sus proyectos desaparecen sin esperar al siguiente sondeo

#### Scenario: Conectar una SD en Android
- **WHEN** se inserta una tarjeta SD en Android
- **THEN** aparece en la lista en un máximo de unos 10 segundos

### Requirement: Expulsión segura
Expulsar un dispositivo SHALL cerrar primero todas las bases de datos de proyectos abiertas en él, impedir que el sondeo las reabra durante la expulsión, y después pedir al sistema operativo que lo desmonte, sin cerrar la app. En macOS SHALL desmontarse todo el disco físico y expulsarse para que sea seguro retirarlo. Si la expulsión tiene éxito, el dispositivo y sus proyectos SHALL desaparecer de la interfaz de inmediato.

#### Scenario: Expulsión correcta
- **WHEN** el usuario expulsa un disco con proyectos abiertos
- **THEN** el disco se desmonta y sus proyectos desaparecen del dashboard al instante

#### Scenario: Expulsión correcta en macOS
- **WHEN** el usuario expulsa desde Lumik un disco montado en `/Volumes/KINGSTON`
- **THEN** el volumen deja de estar montado, el disco queda listo para retirarse y no vuelve a listarse en el siguiente escaneo

#### Scenario: Dispositivo ya desmontado
- **WHEN** el usuario expulsa un dispositivo que ya no está montado
- **THEN** la operación se considera exitosa

#### Scenario: Fallo del sistema operativo
- **WHEN** el sistema operativo rechaza el desmontaje
- **THEN** se informa el error y el dispositivo vuelve a listarse en el siguiente escaneo

#### Scenario: Proceso que bloquea en macOS
- **WHEN** en macOS otro proceso tiene archivos abiertos en el disco y el sistema rechaza el desmontaje
- **THEN** el error informado incluye el proceso que lo bloquea, si el sistema lo reporta, y el disco sigue listado

#### Scenario: Disco con varias particiones en macOS
- **WHEN** el usuario expulsa un disco cuyo disco físico tiene otra partición montada
- **THEN** se intenta desmontar todas las particiones del disco físico y, si alguna no se puede desmontar, se informa el error y el disco sigue listado

### Requirement: Discos de solo lectura
En escritorio, el sistema SHALL marcar como de solo lectura cada dispositivo que el sistema operativo monte sin permiso de escritura. Un dispositivo de solo lectura SHALL listarse con una marca y un aviso de que Lumik no puede abrir ni crear proyectos en él. Sus proyectos no SHALL descubrirse ni abrirse, y el sistema no SHALL escribir en él. Expulsarlo SHALL seguir disponible.

#### Scenario: Disco de solo lectura con proyectos
- **WHEN** se conecta un disco montado en solo lectura que contiene `lumik/2026/10/10_sesion/project.db`
- **THEN** el disco aparece en la barra lateral marcado como de solo lectura, con el aviso, y ese proyecto no aparece en el dashboard

#### Scenario: Nada se escribe en el disco
- **WHEN** Lumik escanea un disco de solo lectura que contiene proyectos
- **THEN** no abre ningún `project.db` de ese disco ni intenta crear archivos en él

#### Scenario: Expulsar un disco de solo lectura
- **WHEN** el usuario expulsa desde Lumik un disco de solo lectura
- **THEN** se expulsa igual que cualquier otro disco

#### Scenario: Mismo disco con escritura en otra plataforma
- **WHEN** el disco NTFS que macOS monta en solo lectura se conecta en Linux, donde se monta con escritura
- **THEN** en Linux se lista sin la marca y sus proyectos se abren

### Requirement: Nombre del disco guardado en el disco
Un disco con carpeta `lumik/` SHALL poder guardar su nombre en `lumik/.disk.json`, junto con su origen: `label` (copiado de la etiqueta del sistema de archivos) o `user` (puesto a mano). Un archivo ilegible o con un formato desconocido SHALL tratarse como si no existiera y no SHALL impedir listar el disco ni sus proyectos.

#### Scenario: Archivo dañado
- **WHEN** se conecta un disco cuyo `lumik/.disk.json` no es JSON válido
- **THEN** el disco se lista con el nombre que tendría sin ese archivo y sus proyectos aparecen igual

### Requirement: La etiqueta se copia al disco
En escritorio, al detectar un disco con escritura y carpeta `lumik/` cuya etiqueta de sistema de archivos reporta el sistema operativo, el sistema SHALL escribir esa etiqueta en `lumik/.disk.json` con origen `label` cuando el archivo falte o tenga otro nombre, aunque el nombre guardado venga de `user`. Si el nombre ya coincide, no SHALL reescribirlo. En discos de solo lectura o sin etiqueta no SHALL escribir.

#### Scenario: Primer escaneo de un disco de Lumik
- **WHEN** se conecta en Linux un disco con etiqueta `SEAGATE`, con escritura, con carpeta `lumik/` y sin `lumik/.disk.json`
- **THEN** se crea `lumik/.disk.json` con el nombre "SEAGATE" y origen `label`

#### Scenario: La etiqueta gana al nombre puesto a mano
- **WHEN** se conecta en el desktop un disco con etiqueta `SEAGATE` cuyo `lumik/.disk.json` tiene el nombre "SSD Bodas" con origen `user`
- **THEN** el archivo pasa a tener el nombre "SEAGATE" con origen `label`

#### Scenario: Nombre al día
- **WHEN** se escanea un disco con etiqueta `SEAGATE` cuyo `lumik/.disk.json` ya tiene el nombre "SEAGATE"
- **THEN** el archivo no se reescribe

#### Scenario: Disco sin etiqueta
- **WHEN** se conecta en el desktop un disco con carpeta `lumik/`, sin etiqueta y sin `lumik/.disk.json`
- **THEN** no se crea ningún archivo en él

### Requirement: Prioridad del nombre mostrado
Toda app de Lumik SHALL mostrar como nombre de un disco, en este orden, el primero disponible: la etiqueta del sistema de archivos que reporta el sistema operativo, el nombre de `lumik/.disk.json` y una descripción genérica propia de la plataforma.

#### Scenario: Disco sin etiqueta con nombre guardado en el desktop
- **WHEN** el desktop detecta un disco sin etiqueta cuyo `lumik/.disk.json` tiene el nombre "SSD Bodas"
- **THEN** el disco se muestra como "SSD Bodas"

#### Scenario: Tablet que no recibe la etiqueta
- **WHEN** la tablet Android lista un disco cuya etiqueta Android no reporta y cuyo `lumik/.disk.json` tiene el nombre "SEAGATE"
- **THEN** el disco se muestra como "SEAGATE"

### Requirement: Nombrar un disco en la tablet
Al dar acceso en la tablet a un disco con escritura que no tiene `lumik/.disk.json`, la app SHALL pedir un nombre, con una sugerencia basada en la descripción genérica, y guardarlo con origen `user`, creando `lumik/` si hace falta. Omitir el paso SHALL dejar el disco sin archivo. Desde "Discos", el nombre SHALL poder cambiarse si su origen es `user`, y no si es `label`.

#### Scenario: Primer acceso a un disco sin nombre
- **WHEN** el fotógrafo da acceso en la tablet a un disco sin `lumik/.disk.json` y escribe "SSD Bodas"
- **THEN** se crea `lumik/.disk.json` con el nombre "SSD Bodas" y origen `user`, y el disco se muestra como "SSD Bodas"

#### Scenario: Disco que ya tiene nombre
- **WHEN** el fotógrafo da acceso en la tablet a un disco cuyo `lumik/.disk.json` tiene el nombre "SEAGATE"
- **THEN** no se pide nombre y el disco se muestra como "SEAGATE"

#### Scenario: Nombre que viene de la etiqueta
- **WHEN** el fotógrafo abre en "Discos" el menú de un disco cuyo nombre tiene origen `label`
- **THEN** no se ofrece cambiar el nombre y se indica que es la etiqueta del disco

### Requirement: Descripción genérica en la tablet
Cuando un disco no tiene nombre, la tablet SHALL describirlo con su tipo y su capacidad. Si tiene `lumik/` es "Disco" y si tiene `DCIM/` es "Tarjeta de cámara"; si no se puede leer su contenido, solo "Disco". La capacidad y el espacio libre se muestran cuando se pueden obtener, y el número de proyectos en los discos de Lumik. El uuid corto SHALL mostrarse junto a cualquier nombre.

#### Scenario: Disco de Lumik sin nombre
- **WHEN** la tablet lista un disco con acceso, sin nombre, con carpeta `lumik/`, 12 proyectos, 1.8 TB de capacidad y 1.2 TB libres
- **THEN** se muestra como "Disco · 1.8 TB (1.2 TB libres) · 12 proyectos" junto a su uuid

#### Scenario: Tarjeta de cámara
- **WHEN** la tablet lista un volumen con acceso, sin nombre, con carpeta `DCIM/` y sin `lumik/`
- **THEN** se muestra como "Tarjeta de cámara" con su capacidad, junto a su uuid

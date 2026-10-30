# Spec Delta

## MODIFIED Requirements

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

## ADDED Requirements

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

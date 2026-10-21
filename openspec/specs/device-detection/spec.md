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
Un dispositivo SHALL identificarse por el UUID de su sistema de archivos que reporta el sistema operativo de esa máquina, no por su punto de montaje. Ese UUID no SHALL usarse para identificar proyectos ni fotos, y el sistema no SHALL escribir archivos en el disco para identificarlo. Cada dispositivo visto SHALL registrarse en el registro global de dispositivos conocidos con su nombre, última ruta de montaje (solo como pista) y fecha de última vez visto.

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
En Android, a falta de etiqueta de volumen, el nombre del dispositivo SHALL derivarse del formato de UUID: `XXXX-XXXX` se muestra como "SD Card" y cualquier otro como "USB Drive".

#### Scenario: Tarjeta SD FAT32
- **WHEN** se detecta en Android un volumen con UUID `1A2B-3C4D`
- **THEN** se muestra con el nombre "SD Card"

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

# Spec Delta

## MODIFIED Requirements

### Requirement: Detección de dispositivos externos
El sistema SHALL listar los dispositivos de almacenamiento externos montados con UUID de sistema de archivos, nombre, punto de montaje, tipo de sistema de archivos y espacio total y disponible. Los discos del sistema y virtuales SHALL excluirse: raíz y particiones del sistema, filesystems virtuales, loop devices, la unidad de Windows, los volúmenes del sistema de macOS y el almacenamiento interno de la tablet Android.

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
- **WHEN** la app de tablet Android lista los discos con una tarjeta SD insertada
- **THEN** aparece la tarjeta SD y no el almacenamiento interno

### Requirement: Refresco ante conexión y desconexión
En Linux, Windows y macOS la lista de dispositivos y el dashboard SHALL refrescarse en cuanto el sistema operativo notifique un cambio de volúmenes, con un sondeo de respaldo cada 60 segundos. En la app de tablet Android SHALL refrescarse con los avisos de montaje del sistema y al volver la app a primer plano.

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
- **WHEN** se inserta una tarjeta SD con la app de tablet abierta
- **THEN** aparece en la lista sin esperar a un sondeo

## REMOVED Requirements

### Requirement: Nombre amigable en Android
**Reason**: Era de la app Tauri para Android, que se retira. La app de tablet nombra los discos sin nombre con su descripción genérica ("Descripción genérica en la tablet").
**Migration**: Ninguna: los discos con `lumik/.disk.json` siguen mostrando ese nombre en la tablet.

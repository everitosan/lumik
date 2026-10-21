# Spec Delta

## MODIFIED Requirements

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

## ADDED Requirements

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

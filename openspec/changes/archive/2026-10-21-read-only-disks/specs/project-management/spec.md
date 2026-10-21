# Spec Delta

## MODIFIED Requirements

### Requirement: Crear proyecto en un dispositivo montado
El sistema SHALL crear un proyecto solo en un dispositivo de almacenamiento actualmente montado y con permiso de escritura, identificado por su UUID. El proyecto SHALL recibir un identificador UUID estable y su base de datos `project.db` SHALL residir dentro de la carpeta del proyecto en ese dispositivo, sin depender de la base de datos global.

#### Scenario: Dispositivo montado
- **WHEN** el usuario crea el proyecto "Boda" con fecha de sesión `2024-06-15` en el dispositivo montado en `/mnt/disk`
- **THEN** se crea `/mnt/disk/lumik/2024/06/15_Boda/project.db` con el proyecto y un id UUID

#### Scenario: Dispositivo no montado
- **WHEN** el usuario crea un proyecto en un dispositivo cuyo UUID no aparece entre los montados
- **THEN** la creación falla con un error indicando que el dispositivo no está montado y no se escribe nada en disco

#### Scenario: Dispositivo de solo lectura
- **WHEN** el único disco conectado está montado en solo lectura y el usuario abre el modal de crear proyecto
- **THEN** ese disco no se ofrece como destino, y una creación pedida con su UUID falla con un error de solo lectura sin escribir nada en disco

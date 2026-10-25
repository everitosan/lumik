# Spec Delta

## MODIFIED Requirements

### Requirement: Expulsión segura
Expulsar un dispositivo SHALL soltar primero los candados de los proyectos que esta instalación tiene en él, cerrar después todas las bases de datos de proyectos abiertas en él, impedir que el sondeo las reabra durante la expulsión, y después pedir al sistema operativo que lo desmonte, sin cerrar la app. En macOS SHALL desmontarse todo el disco físico y expulsarse para que sea seguro retirarlo. Si la expulsión tiene éxito, el dispositivo y sus proyectos SHALL desaparecer de la interfaz de inmediato.

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

#### Scenario: Expulsar con un proyecto abierto
- **WHEN** el usuario expulsa un disco mientras tiene abierto un proyecto de ese disco
- **THEN** el candado del proyecto se quita antes de desmontar y otro dispositivo lo abre sin aviso

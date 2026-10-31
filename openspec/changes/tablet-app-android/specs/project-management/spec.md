# Spec Delta

## MODIFIED Requirements

### Requirement: Portada del proyecto
El usuario SHALL poder elegir una foto del proyecto como portada o quitarla; la portada se muestra con la miniatura de esa foto. Si no hay portada elegida, SHALL mostrarse la miniatura de la primera foto del proyecto por fecha de captura, igual en el desktop y en la tablet. Si la miniatura no existe, el proyecto se muestra sin portada.

#### Scenario: Elegir portada
- **WHEN** el usuario marca una foto como portada desde el visor
- **THEN** el dashboard muestra la miniatura de esa foto en la tarjeta del proyecto

#### Scenario: Miniatura ausente
- **WHEN** la miniatura de la foto de portada ya no existe en disco
- **THEN** el proyecto se muestra sin portada, sin error

#### Scenario: Sin portada elegida
- **WHEN** un proyecto con fotos no tiene portada elegida
- **THEN** el desktop y la tablet muestran en su tarjeta la miniatura de la foto con la fecha de captura más antigua

## ADDED Requirements

### Requirement: Formulario de crear proyecto
El formulario de crear proyecto SHALL nombrar sus campos "Nombre del proyecto" y "Disco destino" en el desktop y en la tablet. En la tablet, la fecha de la sesión SHALL elegirse con el selector de fecha del sistema, no escribiéndola.

#### Scenario: Elegir la fecha en la tablet
- **WHEN** el fotógrafo toca la fecha al crear un proyecto en la tablet
- **THEN** se abre el selector de fecha de Android y la fecha elegida aparece en el formulario

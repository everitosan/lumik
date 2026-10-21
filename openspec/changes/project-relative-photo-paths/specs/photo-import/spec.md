# Spec Delta

## MODIFIED Requirements

### Requirement: Registro de fotos y metadatos EXIF
Cada foto importada SHALL registrarse en el proyecto con su ruta relativa a la carpeta del proyecto y los datos EXIF disponibles: cámara, formato, fecha de captura, dimensiones, tamaño, ISO, apertura, velocidad, compensación de exposición, focal, objetivo y rotación. Las fotos se SHALL registrar todas juntas; si el registro falla, se SHALL informar como fallidas.

#### Scenario: Import correcto
- **WHEN** se importan 120 fotos
- **THEN** el proyecto muestra 120 fotos con ISO, apertura y velocidad visibles en el panel de información

#### Scenario: Fallo al registrar
- **WHEN** falla la escritura en la base de datos del proyecto
- **THEN** el resultado del import lista todas las fotos del lote como fallidas con el motivo

# Spec Delta

## MODIFIED Requirements

### Requirement: Modal de respaldo
El desktop SHALL ofrecer un modal por proyecto que liste los archivos de `_exported/` aún no subidos y deje elegir entre los proveedores conectados (todos por defecto) y la calidad: original, alta (lado mayor de 3840 px, JPEG 90), media (2048 px, 82) o baja (1280 px, 75). Sin proveedores conectados, el servidor SHALL rechazar la subida. Al recomprimir, el servidor SHALL girar los píxeles según la orientación del original y SHALL conservar sus metadatos (autor, copyright, datos de cámara y perfil de color) con la orientación en normal, para que la foto se vea derecha y con la firma del fotógrafo en la nube.

#### Scenario: Elegir calidad media
- **WHEN** el fotógrafo respalda con calidad media un JPEG de 6000 px
- **THEN** en la nube queda un JPEG de 2048 px en su lado mayor

#### Scenario: Calidad original
- **WHEN** el fotógrafo respalda con calidad original
- **THEN** el archivo sube sin recomprimir

#### Scenario: JPG de cámara vertical recomprimido
- **WHEN** el fotógrafo respalda con calidad media el JPG de cámara de una foto vertical, que guarda los píxeles en horizontal y la etiqueta de orientación
- **THEN** en la nube la foto se ve vertical y conserva el autor y el copyright del original

#### Scenario: Sin proveedores
- **WHEN** el fotógrafo no tiene proveedores conectados e intenta respaldar
- **THEN** la subida se rechaza indicando que no hay proveedores conectados

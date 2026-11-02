# Spec Delta

## ADDED Requirements

### Requirement: Motor de la búsqueda por contenido descargado
En macOS y en Linux, el motor de la búsqueda por contenido SHALL descargarse junto con su modelo, verificarse y borrarse con él, en lugar de venir dentro de la app. La app SHALL abrir y funcionar sin ese motor en todos los sistemas que soporta. En Windows el motor SHALL seguir viniendo en la app.

#### Scenario: Descargar en Linux
- **WHEN** el fotógrafo descarga el modelo de búsqueda por contenido en Ubuntu 22.04
- **THEN** se descargan el modelo y su motor, y la búsqueda por contenido funciona sin reiniciar la app

#### Scenario: Descargar en macOS 14
- **WHEN** el fotógrafo descarga el modelo de búsqueda por contenido en una Mac con Apple Silicon y macOS 14
- **THEN** se descargan el modelo y su motor, y la búsqueda por contenido funciona sin reiniciar la app

#### Scenario: Borrar el modelo
- **WHEN** el fotógrafo borra el modelo en macOS o en Linux
- **THEN** se borran también los archivos del motor

#### Scenario: Windows sin cambios
- **WHEN** el fotógrafo descarga el modelo en Windows
- **THEN** se descarga solo el modelo, porque el motor ya viene en la app

### Requirement: Búsqueda por contenido en macOS
La búsqueda por contenido SHALL requerir, en macOS, una Mac con Apple Silicon y macOS 14 o posterior. En una Mac Intel o con una versión anterior, Ajustes SHALL indicar lo que falta y no SHALL ofrecer la descarga, y el resto de la app SHALL funcionar igual.

#### Scenario: Monterey
- **WHEN** el fotógrafo abre Lumik en una Mac con macOS 12
- **THEN** la app arranca y funciona, y en Ajustes la búsqueda por contenido indica que requiere macOS 14 o posterior, sin botón de descarga

#### Scenario: Mac Intel
- **WHEN** el fotógrafo abre Lumik en una Mac Intel, con cualquier versión de macOS
- **THEN** la app arranca y funciona, y en Ajustes la búsqueda por contenido indica que requiere una Mac con Apple Silicon, sin botón de descarga

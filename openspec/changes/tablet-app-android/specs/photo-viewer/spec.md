# Spec Delta

## MODIFIED Requirements

### Requirement: Gestos táctiles
En pantallas táctiles el visor SHALL permitir zoom con dos dedos, desplazarse arrastrando con un dedo y cambiar de foto deslizando en horizontal: un gesto rápido (menos de 400 ms), de más de 60 px y claramente horizontal.

#### Scenario: Deslizar en tablet
- **WHEN** en una pantalla táctil el usuario desliza rápido de derecha a izquierda
- **THEN** se abre la siguiente foto

#### Scenario: Pellizcar
- **WHEN** el usuario separa dos dedos sobre la imagen
- **THEN** la imagen se amplía

### Requirement: Mostrar en carpeta
En el desktop, el menú de la foto SHALL ofrecer "Mostrar en carpeta", que abre el explorador de archivos del sistema operativo en la carpeta donde está el archivo original de la foto, con ese archivo seleccionado. En la app de tablet Android SHALL abrir esa carpeta en la app de Archivos del sistema, sin seleccionar el archivo. Si el archivo no existe en disco SHALL informarse el error.

#### Scenario: Foto aprobada en Linux
- **WHEN** el usuario elige "Mostrar en carpeta" en una foto aprobada
- **THEN** el explorador se abre en `_culled/` con el archivo de la foto seleccionado

#### Scenario: Android
- **WHEN** el usuario elige "Mostrar en carpeta" en una foto aprobada en la app de tablet Android
- **THEN** la app de Archivos del sistema se abre en `_culled/` del proyecto

#### Scenario: Archivo ausente
- **WHEN** el usuario elige "Mostrar en carpeta" y el archivo ya no está en disco
- **THEN** se muestra un error y no se abre el explorador


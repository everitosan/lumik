# Spec Delta

## MODIFIED Requirements

### Requirement: Etiquetas de color visibles en iPad y Mac
Si el ajuste correspondiente está activo (por defecto sí), en Linux y Windows cada cambio de color SHALL escribir junto a la foto, y junto a su JPG exportado si existe, un sidecar `._{nombre}` con las etiquetas en formato Finder, con nombres en español. Quitar todos los colores SHALL borrar el sidecar. Un fallo al escribirlo no SHALL impedir guardar la calificación.

#### Scenario: Marcar en rojo
- **WHEN** el usuario marca `IMG_0001.cr2` en rojo en un disco exFAT
- **THEN** existe `._IMG_0001.cr2` y un iPad conectado al disco muestra la foto con la etiqueta "Rojo"

#### Scenario: Quitar el color
- **WHEN** el usuario quita todos los colores de la foto
- **THEN** el sidecar `._IMG_0001.cr2` se elimina

#### Scenario: Ajuste desactivado
- **WHEN** el ajuste de sidecars para iPad está desactivado
- **THEN** cambiar el color no crea ni modifica sidecars

#### Scenario: Android
- **WHEN** se cambia un color en la app de tablet Android
- **THEN** no se escribe ningún sidecar


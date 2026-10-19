# photo-rating Specification

## Purpose

Permite calificar y clasificar cada foto con estrellas, etiquetas de color y tags libres, y hace visibles las etiquetas de color en un iPad o Mac que lea el mismo disco.

## Requirements

### Requirement: Calificación en estrellas
Cada foto SHALL tener una calificación de 0 a 5 estrellas, 0 por defecto. Un valor fuera de ese rango SHALL rechazarse sin modificar la foto.

#### Scenario: Calificar con teclado
- **WHEN** en el visor el usuario pulsa `4`
- **THEN** la foto queda con 4 estrellas

#### Scenario: Valor fuera de rango
- **WHEN** se intenta guardar 6 estrellas
- **THEN** la operación falla y la calificación anterior se conserva

### Requirement: Etiquetas de color
Cada foto SHALL poder tener cero o más etiquetas de color de un conjunto fijo de cinco: 1 rojo, 2 amarillo, 3 verde, 4 azul y 5 morado.

#### Scenario: Varias etiquetas
- **WHEN** el usuario marca una foto en rojo y en verde
- **THEN** la foto conserva ambas etiquetas

### Requirement: Tags normalizados
Cada foto SHALL poder tener tags de texto libre. Al guardar, los tags SHALL pasar a minúsculas, recortarse de espacios y eliminar duplicados, conservando el orden en que aparecen. Si no queda ningún tag, la foto SHALL quedar sin tags.

#### Scenario: Normalización
- **WHEN** se guardan los tags `" Perfil, grupal ,perfil,, "`
- **THEN** la foto queda con los tags `perfil` y `grupal`

### Requirement: Sugerencia de tags
Al escribir un tag, la interfaz SHALL sugerir hasta 8 tags existentes que contengan el texto escrito y que la foto no tenga ya, y SHALL permitir crear uno nuevo si no existe. Un atajo de teclado (por defecto `t`) SHALL abrir la entrada de tags.

#### Scenario: Autocompletar
- **WHEN** el proyecto ya usa el tag `perfil` y el usuario escribe `per`
- **THEN** se sugiere `perfil`

### Requirement: Guardado optimista
Los cambios de estrellas, color y tags SHALL verse en la interfaz al instante y guardarse en la base de datos del proyecto, que es la fuente de verdad de la calificación.

#### Scenario: Cambio rápido entre fotos
- **WHEN** el usuario califica una foto y pasa enseguida a la siguiente
- **THEN** la calificación de la primera se ve aplicada al volver a ella

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
- **WHEN** se cambia un color en Android
- **THEN** no se escribe ningún sidecar

### Requirement: Reindexado en el iPad
Cuando el disco contiene un índice de Spotlight, tras escribir etiquetas el sistema SHALL invalidarlo en segundo plano para que el iPad reindexe y pueda filtrar por las nuevas etiquetas.

#### Scenario: Disco con índice previo
- **WHEN** se cambia un color en un disco que tiene `.Spotlight-V100`
- **THEN** el índice se invalida sin bloquear la interfaz

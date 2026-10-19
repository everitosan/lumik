# photo-rotation Specification

## Purpose

Permite enderezar fotos en ángulos rectos desde el visor, con respuesta inmediata en pantalla, y deja la orientación escrita en el archivo para que otras herramientas la respeten.

## Requirements

### Requirement: Rotación en ángulos rectos
La rotación de una foto SHALL ser 0, 90, 180 o 270 grados en sentido horario. Cualquier otro valor SHALL rechazarse sin cambiar la foto. Al importar, la rotación inicial SHALL tomarse de la orientación EXIF del archivo (6→90, 3→180, 8→270, cualquier otra→0).

#### Scenario: Rotar a la derecha
- **WHEN** el usuario rota a la derecha una foto a 0°
- **THEN** la foto queda a 90°

#### Scenario: Valor inválido
- **WHEN** se intenta guardar una rotación de 45°
- **THEN** la operación falla y la rotación anterior se conserva

#### Scenario: Orientación de cámara vertical
- **WHEN** se importa una foto con orientación EXIF 6
- **THEN** la foto aparece con 90° de rotación

### Requirement: Rotación inmediata en el visor
El visor SHALL aplicar la rotación al instante, sin esperar a que se guarde, mediante atajos configurables (por defecto `[` a la izquierda y `]` a la derecha).

#### Scenario: Atajo de teclado
- **WHEN** el usuario pulsa `]` en el visor
- **THEN** la imagen gira 90° de inmediato

### Requirement: Miniatura actualizada
Al cambiar la rotación, la miniatura de la foto SHALL rotarse la diferencia entre la rotación anterior y la nueva, para que el grid muestre la orientación vigente.

#### Scenario: Volver al grid
- **WHEN** el usuario rota una foto 90° y vuelve al grid
- **THEN** la miniatura aparece rotada 90°

### Requirement: Orientación escrita en el archivo con espera
La orientación final SHALL escribirse en el archivo unos 600 ms después del último cambio, sin bloquear la interfaz. Si en ese intervalo llegan más rotaciones de la misma foto, SHALL hacerse una sola escritura con el valor final. En Linux y Windows se SHALL escribir dentro del archivo; en Android, en su sidecar `.xmp`, creándolo si no existe.

#### Scenario: Rotaciones seguidas
- **WHEN** el usuario pulsa `]` tres veces seguidas
- **THEN** el archivo recibe una sola escritura con orientación de 270°

#### Scenario: Android
- **WHEN** se rota una foto en Android
- **THEN** la orientación se guarda en el `.xmp` de la foto y el RAW no se modifica

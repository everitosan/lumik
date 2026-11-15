# Spec Delta

## ADDED Requirements

### Requirement: Marcar elementos en la papelera
En la sección "Papelera" SHALL poder marcarse varios elementos (proyectos y fotos sueltas) de la misma manera que en el grid del proyecto: Ctrl/Cmd+clic, Shift+clic y casilla en el desktop, y toque largo, toques y "Listo" en la tablet. Mientras haya elementos marcados, SHALL mostrarse una barra con el número de elementos marcados y las acciones "Restaurar", "Eliminar definitivamente" y quitar las marcas. Los rangos de Shift+clic SHALL quedarse dentro de un mismo disco.

#### Scenario: Marcar en dos discos
- **WHEN** el usuario marca una foto de KINGSTON y un proyecto de SEAGATE
- **THEN** la barra dice "2 elementos marcados"

### Requirement: Restaurar en lote
"Restaurar" de la barra SHALL restaurar cada elemento marcado como si se restaurara solo, sin confirmación. Un elemento cuyo lugar esté ocupado no SHALL abrir el modal de colisión: SHALL quedarse en la papelera, y al terminar SHALL informarse cuáles no se restauraron y por qué, para restaurarlos uno por uno con otro nombre.

#### Scenario: Lote con una colisión
- **WHEN** el usuario restaura 5 fotos marcadas y ya existe `_media/IMG_0003.cr2`
- **THEN** 4 vuelven a su proyecto, `IMG_0003.cr2` sigue en la papelera y se informa que su lugar está ocupado

### Requirement: Eliminar definitivamente en lote
"Eliminar definitivamente" de la barra SHALL pedir una sola confirmación con el número de proyectos y de fotos marcados, que diga que no se puede deshacer y que el respaldo en la nube no se toca. Al confirmar, cada elemento SHALL eliminarse como si se eliminara solo. Si uno falla, los demás SHALL eliminarse igual, y al terminar SHALL informarse cuáles fallaron y por qué.

#### Scenario: Eliminar un lote
- **WHEN** el usuario marca 1 proyecto y 6 fotos y confirma "Eliminar definitivamente"
- **THEN** se borran el proyecto y las 6 fotos, y dejan de verse en la papelera

#### Scenario: Foto de un proyecto en uso
- **WHEN** una de las fotos marcadas es de un proyecto que otro dispositivo tiene en uso
- **THEN** se eliminan las demás, esa foto sigue en la papelera y se informa quién tiene su proyecto

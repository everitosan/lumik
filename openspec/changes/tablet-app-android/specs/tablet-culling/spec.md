# Spec Delta

## Purpose

El culling táctil en la app de tablet Android: visor con gestos y una guía para aprenderlos, deshacer siempre, estrellas y etiquetas de color al alcance del pulgar, y aprobar con las mismas reglas de archivos que el desktop.

## ADDED Requirements

### Requirement: Gestos del visor
En el visor, deslizar a los lados SHALL cambiar de foto, deslizar hacia arriba SHALL aprobar, deslizar hacia abajo SHALL quitar la aprobación y tocar dos veces con dos dedos SHALL alternar la aprobación. Pellizcar SHALL hacer zoom siguiendo los dedos y tocar dos veces con un dedo SHALL acercar en ese punto o volver a ajustar la foto. El visor no SHALL tener botones de aprobar ni de quitar la aprobación.

#### Scenario: Aprobar deslizando
- **WHEN** el fotógrafo desliza hacia arriba sobre una foto sin aprobar
- **THEN** la foto queda aprobada y su original pasa a `_culled/`

#### Scenario: Zoom con doble toque
- **WHEN** el fotógrafo toca dos veces con un dedo sobre el ojo de un retrato
- **THEN** la foto se acerca centrada en ese punto

### Requirement: Guía de gestos
La primera vez que se abre el visor SHALL mostrarse una guía que enseña cada gesto con dedos animados, paso a paso, y SHALL poder abrirse de nuevo desde el visor. Que ya se vio SHALL recordarse en la tablet.

#### Scenario: Segunda vez
- **WHEN** el fotógrafo abre el visor después de haber cerrado la guía
- **THEN** la guía no aparece, y el botón de ayuda la vuelve a mostrar

### Requirement: Deshacer y háptica
Aprobar, quitar la aprobación y calificar SHALL mostrar durante unos segundos un aviso con "Deshacer" que devuelve la foto a su estado anterior, y SHALL dar una vibración corta. Por eso estas acciones no SHALL pedir confirmación.

#### Scenario: Aprobar por error
- **WHEN** el fotógrafo aprueba una foto y toca "Deshacer" en el aviso
- **THEN** la foto vuelve a estar sin aprobar y su original vuelve a `_media/`

### Requirement: Estrellas y etiquetas de color
Sobre la foto SHALL estar las estrellas (a la izquierda) y las etiquetas de color del desktop (a la derecha), con objetivos de al menos 48 dp, encima de una tira de miniaturas que ocupa el ancho de la pantalla. Un toque en la foto SHALL alternar el modo limpio, que oculta la tira, las estrellas, las etiquetas y las barras del sistema.

#### Scenario: Calificar con el pulgar
- **WHEN** el fotógrafo toca la cuarta estrella con la tablet sujeta con las dos manos
- **THEN** la foto queda con 4 estrellas y aparece el aviso con "Deshacer"

#### Scenario: Modo limpio
- **WHEN** el fotógrafo toca una vez la foto
- **THEN** se ocultan la tira, las estrellas, las etiquetas y las barras del sistema, y otro toque las regresa

### Requirement: Aprobar con la entrega del proyecto
Aprobar y quitar la aprobación en la tablet SHALL mover los archivos como en el desktop (`photo-culling`), incluido el sidecar `.xmp`, y SHALL aplicar la opción "Al aprobar, entregar directamente" del proyecto (`camera-jpeg-delivery`) al JPG de cámara.

#### Scenario: Par con entrega de pares
- **WHEN** el fotógrafo aprueba en la tablet un par RAW+JPG en un proyecto con la entrega "el JPG de los pares"
- **THEN** el RAW pasa a `_culled/` y el JPG a `_exported/`, igual que en el desktop

### Requirement: Datos de la foto
El visor SHALL ofrecer un panel ocultable con el histograma RGB de la foto y sus datos de toma: velocidad, diafragma, ISO, compensación, cámara, óptica, focal y fecha.

#### Scenario: Abrir el panel
- **WHEN** el fotógrafo toca el botón de información
- **THEN** aparece sobre la foto el panel con el histograma y los datos de la toma

### Requirement: Rotar en el visor
El visor SHALL ofrecer rotar la foto a la izquierda y a la derecha. La rotación SHALL verse al instante en la foto y en su miniatura, guardarse en el proyecto y escribirse en el sidecar `.xmp` de la foto, sin modificar el archivo original (`photo-rotation`).

#### Scenario: Rotar una foto acostada
- **WHEN** el fotógrafo rota a la derecha una foto que se ve acostada
- **THEN** la foto y su miniatura se ven derechas, y al abrir el proyecto en el desktop la foto aparece con la misma orientación

### Requirement: Orientación de la cámara
Las fotos SHALL mostrarse en el visor, en la tira y en el grid con la orientación que guardó la cámara, también en los RAW de Canon (`.cr2`, `.cr3`), igual que en el desktop.

#### Scenario: Foto vertical de una Canon
- **WHEN** se importa en la tablet un `.cr2` tomado con la cámara en vertical
- **THEN** la foto se ve vertical en el grid, en la tira y en el visor


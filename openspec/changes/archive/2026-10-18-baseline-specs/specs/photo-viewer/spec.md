# Spec Delta

## Purpose

Permite recorrer las fotos de un proyecto en un grid de miniaturas y revisarlas una a una a tamaño completo, con la información técnica necesaria para decidir cuáles aprobar.

## ADDED Requirements

### Requirement: Grid de miniaturas
La vista de proyecto SHALL mostrar las fotos en un grid de miniaturas que se carga de forma progresiva, de modo que proyectos de miles de fotos se puedan desplazar con fluidez. Cada miniatura SHALL reflejar la rotación vigente y los indicadores de aprobada, estrellas y color.

#### Scenario: Proyecto grande
- **WHEN** se abre un proyecto con 3000 fotos
- **THEN** el grid aparece sin esperar a cargar todas las miniaturas y se desplaza con fluidez

### Requirement: Visor a tamaño completo
Al abrir una foto, el visor SHALL mostrar una previsualización a resolución completa sacada del propio archivo y SHALL guardarla en `.previews/{id}.jpg` para las siguientes aperturas. Al pasar de una foto a otra no SHALL verse un parpadeo en blanco.

#### Scenario: Abrir una foto RAW
- **WHEN** el usuario abre un CR3 desde el grid
- **THEN** el visor muestra la imagen completa ajustada a la pantalla

#### Scenario: Navegación rápida
- **WHEN** el usuario pasa de una foto a la siguiente
- **THEN** la imagen anterior se mantiene hasta que aparece la nueva, sin parpadeo en blanco

### Requirement: Zoom y desplazamiento
El visor SHALL permitir hacer zoom entre el 5 % y el 3000 % con la rueda del ratón, centrado en el cursor, o con atajos (por defecto `+` y `-`); desplazarse arrastrando; y volver a ajustar a la pantalla (por defecto `f`). Al cambiar de foto o de rotación, la imagen SHALL volver a ajustarse a la pantalla.

#### Scenario: Zoom con rueda
- **WHEN** el usuario gira la rueda hacia arriba sobre un ojo del retrato
- **THEN** la imagen se amplía manteniendo ese punto bajo el cursor

#### Scenario: Ajustar a pantalla
- **WHEN** el usuario pulsa `f` con la imagen ampliada
- **THEN** la imagen completa vuelve a caber en el visor

### Requirement: Navegación entre fotos
El visor SHALL permitir ir a la foto anterior y a la siguiente (por defecto `←` y `→`) y cerrarse para volver al grid (por defecto `Escape`).

#### Scenario: Siguiente con teclado
- **WHEN** el usuario pulsa `→`
- **THEN** se abre la siguiente foto del grid

### Requirement: Gestos táctiles
En pantallas táctiles el visor SHALL permitir zoom con dos dedos, desplazarse arrastrando con un dedo y cambiar de foto deslizando en horizontal: un gesto rápido (menos de 400 ms), de más de 60 px y claramente horizontal.

#### Scenario: Deslizar en tablet
- **WHEN** en Android el usuario desliza rápido de derecha a izquierda
- **THEN** se abre la siguiente foto

#### Scenario: Pellizcar
- **WHEN** el usuario separa dos dedos sobre la imagen
- **THEN** la imagen se amplía

### Requirement: Histograma RGB
El panel de información SHALL mostrar un histograma de los canales rojo, verde y azul de la foto visible. Calcularlo no SHALL bloquear la interfaz.

#### Scenario: Cambio de foto
- **WHEN** el usuario pasa a una foto sobreexpuesta
- **THEN** el histograma se actualiza y muestra los valores acumulados a la derecha

### Requirement: Panel de información
El visor SHALL tener un panel lateral, que se puede ocultar, con el histograma, los parámetros de toma (ISO, apertura, velocidad, compensación, focal), cámara y objetivo, y los controles de estrellas, color, tags y aprobación. Si está abierto o cerrado SHALL recordarse por proyecto.

#### Scenario: Ocultar panel
- **WHEN** el usuario oculta el panel y vuelve a abrir el proyecto más tarde
- **THEN** el visor se abre con el panel oculto

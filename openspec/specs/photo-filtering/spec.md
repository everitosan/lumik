# photo-filtering Specification

## Purpose

Permite al fotógrafo acotar y ordenar las fotos de un proyecto por estado, calificación, tags y color, para revisar o entregar subconjuntos de la sesión, y recuerda esa vista por proyecto.

## Requirements

### Requirement: Filtro de aprobadas
La vista de proyecto SHALL ofrecer un filtro que muestre solo las fotos aprobadas (culled), activable con un atajo configurable (por defecto `Ctrl+c`). Sin el filtro SHALL mostrarse todas las fotos del proyecto, aprobadas o no.

#### Scenario: Ver solo aprobadas
- **WHEN** el usuario activa el filtro de aprobadas
- **THEN** el grid muestra únicamente las fotos aprobadas

#### Scenario: Sin filtro
- **WHEN** el filtro de aprobadas está desactivado
- **THEN** el grid muestra todas las fotos del proyecto

### Requirement: Filtro por estrellas
La vista SHALL permitir filtrar por un número de estrellas de 1 a 5 en modo "al menos" (por defecto) o en modo "exacto".

#### Scenario: Modo al menos
- **WHEN** el filtro es 3 estrellas en modo "al menos"
- **THEN** se muestran las fotos con 3, 4 o 5 estrellas

#### Scenario: Modo exacto
- **WHEN** el filtro es 3 estrellas en modo "exacto"
- **THEN** solo se muestran las fotos con exactamente 3 estrellas

### Requirement: Filtro por tags
La vista SHALL permitir elegir uno o más de los tags usados en el proyecto y SHALL mostrar las fotos que tengan al menos uno de ellos.

#### Scenario: Varios tags
- **WHEN** el usuario selecciona `perfil` y `grupal`
- **THEN** se muestran las fotos que tienen `perfil`, `grupal` o ambos, y no las que no tienen tags

### Requirement: Filtro por color
La vista SHALL permitir elegir una o más etiquetas de color y SHALL mostrar las fotos que tengan al menos una de ellas.

#### Scenario: Rojo o verde
- **WHEN** el usuario selecciona rojo y verde
- **THEN** se muestran las fotos con alguna de esas etiquetas

### Requirement: Filtros combinables
Los filtros de aprobadas, estrellas, tags, color y papelera SHALL combinarse entre sí: una foto se muestra solo si cumple todos los filtros activos.

#### Scenario: Estrellas y tag
- **WHEN** están activos "al menos 4 estrellas" y el tag `perfil`
- **THEN** solo se muestran las fotos con 4 o más estrellas que tienen el tag `perfil`

#### Scenario: Papelera y color
- **WHEN** están activos "Papelera" y el color rojo
- **THEN** solo se muestran las fotos en la papelera con etiqueta roja

### Requirement: Orden de las fotos
La vista SHALL permitir ordenar las fotos por fecha de captura ascendente (por defecto), por nombre de archivo o por estrellas descendente.

#### Scenario: Orden por estrellas
- **WHEN** el usuario ordena por estrellas
- **THEN** las fotos con 5 estrellas aparecen primero

### Requirement: Modos de vista
La vista SHALL ofrecer un modo agrupado por día (por defecto), con un separador por fecha de captura del día más antiguo al más reciente y las fotos sin fecha al final, y un modo de grid continuo sin separadores.

#### Scenario: Sesión de dos días
- **WHEN** un proyecto tiene fotos del 14 y del 15 de junio en el modo agrupado por día
- **THEN** primero aparece el separador del 14 con sus fotos y después el del 15

### Requirement: Preferencias de vista por proyecto
Los filtros activos (aprobadas, estrellas y su modo, tags, colores), el modo de vista y el estado del panel de información del visor SHALL guardarse en el propio proyecto y restaurarse al volver a abrirlo, también desde otra máquina con el mismo disco.

#### Scenario: Reabrir proyecto
- **WHEN** el usuario filtra por el tag `perfil`, cierra el proyecto y lo vuelve a abrir
- **THEN** el filtro por `perfil` sigue activo

### Requirement: Navegación del visor sobre la selección filtrada
El visor SHALL recorrer con anterior y siguiente solo las fotos visibles con los filtros y el orden actuales.

#### Scenario: Siguiente con filtro
- **WHEN** hay un filtro de 5 estrellas y el usuario pulsa "siguiente" en el visor
- **THEN** se abre la siguiente foto de 5 estrellas

### Requirement: Filtro de papelera
La barra de filtros SHALL ofrecer un filtro "Papelera" que muestra solo las fotos del proyecto que están en la papelera, y SHALL indicar cuántas hay. Sin el filtro, las fotos en la papelera no SHALL mostrarse. El filtro no SHALL guardarse con las preferencias de vista: al abrir un proyecto la vista SHALL empezar sin él.

#### Scenario: Ver la papelera
- **WHEN** el usuario activa el filtro "Papelera" en un proyecto con 3 fotos en la papelera
- **THEN** el grid muestra solo esas 3 fotos

#### Scenario: Reabrir el proyecto
- **WHEN** el usuario cierra el proyecto con el filtro "Papelera" activo y lo vuelve a abrir
- **THEN** el grid muestra las fotos que no están en la papelera

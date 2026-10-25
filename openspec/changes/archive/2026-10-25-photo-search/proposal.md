# Proposal

## Why

Las fotos de un fotógrafo viven repartidas en muchos proyectos y en varios discos, y hoy solo se pueden filtrar dentro de un proyecto abierto. Para casos como responder a una convocatoria temática o encontrar "las mejores fotos de calle de 2025", hace falta buscar en todos los proyectos a la vez, aunque sus discos no estén conectados, con la potencia de una búsqueda avanzada al estilo de Gmail.

## What Changes

- Un índice de fotos en la base global de cada equipo, que se llena y se refresca cuando un disco se conecta y cuando cambia un proyecto. Es una copia: la fuente de verdad sigue siendo cada `project.db`.
- Un cuadro de búsqueda con operadores (`tag:`, `-tag:`, `estrellas>=`, `color:`, `camara:`, `lente:`, `en:`, `antes:`, `despues:`, `proyecto:`, `es:`) y texto libre, y un panel de búsqueda avanzada que arma esos operadores con campos.
- Resultados de todos los proyectos agrupados por proyecto, con miniatura aunque el disco no esté conectado, y aviso de qué disco conectar para abrir una foto.
- En este change el texto libre busca por coincidencia en tags, nombre del archivo y del proyecto y descripción. El índice reserva el lugar del vector de cada foto para la búsqueda semántica, que llega en un change posterior (`semantic-search`) sin cambiar la interfaz.

## Capabilities

### New Capabilities
- `photo-search`: búsqueda de fotos en todos los proyectos del fotógrafo: el índice local, los operadores y el panel avanzado, los resultados y su comportamiento con discos desconectados.

### Modified Capabilities

_Ninguna: el filtro dentro de un proyecto (`photo-filtering`) no cambia._

## Impact

- Núcleo (`crates/lumik-core`): tablas del índice en la base global, refresco por generación del proyecto, interpretación de la consulta y caso de uso de búsqueda. Lo hereda la app de tablet.
- Desktop: comandos de búsqueda y de estado del índice, refresco al escanear discos y tras cada escritura, vista de búsqueda y panel avanzado, textos `es`/`en`.
- Base global (`lumik.db`): crece con el índice (miniatura de unos KB por foto). Se reconstruye conectando los discos si se borra.
- Base del proyecto: sin cambios en este change.

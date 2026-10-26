# Proposal

## Why

El fotógrafo junta fotos de varias sesiones para un mismo fin (una convocatoria temática, un portafolio, lo mejor del año), pero en Lumik cada foto vive en su proyecto y cada proyecto en su disco. Hace falta poder agruparlas en un álbum y curarlo desde antes de editar, para que el álbum quede listo cuando las fotos estén terminadas.

## What Changes

- Álbumes locales al equipo, guardados en la base global: crear, renombrar, borrar.
- Agregar fotos de cualquier proyecto desde el visor (con atajo), desde el menú de la foto en el grid y en los resultados de búsqueda, y todos los resultados de una búsqueda de una vez.
- Un álbum acepta originales y exportados. Cada elemento muestra su mejor versión: la edición de `_exported/` si la hay, si no el original. El álbum madura solo conforme se exporta.
- Reemplazar un elemento por otra foto o exportado del mismo proyecto, y quitar elementos.
- Ver el álbum aunque sus discos no estén conectados, con el aviso de qué discos faltan y cuántas fotos hay en cada uno. Un elemento cuya foto ya no existe se conserva marcado.
- Abrir un elemento en el visor de su proyecto (y volver al álbum al cerrarlo) y mostrarlo en la carpeta: el exportado si es final, el original si no.
- El índice de búsqueda incluye los archivos de `_exported/` de cada proyecto, con miniatura de la edición.
- Fuera de alcance: exportar un álbum, la nube, el portal y la sincronización entre dispositivos.

## Capabilities

### New Capabilities
- `photo-albums`: álbumes de fotos de varios proyectos y discos: crearlos, agregar y quitar fotos, su mejor versión, reemplazar, discos faltantes, fotos que ya no existen, abrir y mostrar en carpeta.

### Modified Capabilities

_Ninguna._

## Impact

- Núcleo (`crates/lumik-core`): tablas `album`, `album_item` y `search_export` en `lumik.db`; indexado de `_exported/` con miniatura de la edición; casos de uso de álbumes. Lo hereda la app de tablet.
- Desktop: comandos de álbumes, sección "Álbumes" en la barra lateral, detalle del álbum, "Agregar a álbum" en el visor (atajo `a`), en el menú de la foto y en la búsqueda, "Reemplazar por…", mostrar en carpeta de un exportado, textos `es`/`en`.
- `@lumik/ui`: `PhotoCard` acepta las opciones del menú para álbumes.

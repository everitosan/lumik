# Proposal

## Why

Un álbum es una curación: el fotógrafo decide qué fotos van y en qué secuencia (una convocatoria, un portafolio, la entrega de una boda). Hoy el álbum solo se ordena por fecha en que se agregaron o por fecha de captura, y ninguno de los dos es el orden que el fotógrafo quiere contar. Además, el modo elegido no se guarda: al reabrir el álbum vuelve a "agregadas".

## What Changes

- **Orden personalizado como orden propio del álbum.** "Personalizado" sustituye a "Agregadas" en "Ordenar por". Al principio es el orden en que se agregaron las fotos, así que ningún álbum existente cambia al actualizar.
- **Reordenar arrastrando** en el grid del álbum: la foto se lleva pegada al cursor y, al soltarla, se **inserta** en ese lugar; las demás se recorren y se apartan en vivo para hacerle espacio. Se arrastra una foto a la vez.
- **"Mover al principio" y "Mover al final"** en el menú de cada foto del álbum, para mover fotos lejos en álbumes largos sin arrastrar por todo el grid.
- **Lo nuevo entra al final** del orden personalizado; reemplazar un elemento conserva su lugar.
- **"Fecha de captura" queda como una vista**: muestra el álbum por fecha y ahí no se arrastra ni aparecen las opciones de mover.
- **El modo de orden se guarda por álbum.**
- El visor recorre el álbum en el orden personalizado, porque ya sigue el orden en que se ve.
- **Fuera de alcance:** mover varias fotos a la vez (no hay selección múltiple en los grids); la tablet y el portal web, que no tienen álbumes.

## Capabilities

### New Capabilities

(ninguna)

### Modified Capabilities

- `photo-albums`: el álbum tiene un orden personalizado que el fotógrafo cambia arrastrando o desde el menú de la foto, y el modo de orden se guarda por álbum.

## Impact

- **`crates/lumik-core`:**
  - `lumik.db`: columna `position` en `album_item` y `sort` en `album`, con una migración idempotente que siembra `position` desde `added_at`;
  - `db/albums.rs`: agregar al final, mover un elemento (a un índice, al principio, al final), leer en orden personalizado y guardar el modo;
  - `AlbumOrder`: `Custom` en lugar de `Added`.
- **`apps/desktop`:**
  - comandos Tauri para mover un elemento y guardar el modo de orden;
  - `AlbumDetail`: arrastrar para insertar sobre el grid virtualizado, con auto-scroll y eventos de puntero;
  - las opciones del menú de la foto y las traducciones (`es.json`, `en.json`).
- **`docs/useCases/flujo-albumes.md`:** el diagrama del flujo con el reordenamiento.
- Sin cambios en la tablet, el servidor ni el portal web.

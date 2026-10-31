# Design

## Context

- Los álbumes viven en `lumik.db` (`crates/lumik-core/src/db/global_schema.sql`): `album` (`id`, `name`, fechas) y `album_item` (`kind`, `ref_id`, nombres copiados, `added_at`, `UNIQUE(album_id, kind, ref_id)`). No hay columna de posición.
- `GlobalDatabase::album_entries(album_id, AlbumOrder)` ordena por `added_at, id` y, con `AlbumOrder::Captured`, reordena en Rust por fecha de captura. `ReadAlbum` (`application/use_cases/albums.rs`) lo envuelve y el comando `get_album` recibe el orden como opcional (por defecto, `Added`).
- `AlbumDetail.tsx` guarda el orden en un `useState('added')`, no persistido. El grid está virtualizado por filas con `@tanstack/react-virtual`. Su geometría es determinista: `cols`, `cardWidth` y `rowHeight` salen del ancho del contenedor, con `COL_GAP` y `ROW_GAP` fijos. El menú de cada tarjeta se extiende con `extraMenuItems` de `PhotoCard`.
- La ventana de Tauri tiene `dragDropEnabled: true` (soltar archivos del sistema). En Windows (WebView2) eso impide el drag and drop de HTML5 dentro de la página.
- El visor recorre `view.entries` en el orden en que llegan; la spec ya pide "el orden en que se ven".
- Las migraciones de `lumik.db` son funciones idempotentes que revisan si la columna existe (`migrations.rs`), sin `user_version`.

## Goals / Non-Goals

**Goals:**
- Un orden por álbum guardado en la base, que no cambie ningún álbum existente al actualizar.
- Una sola operación de backend para mover, que sirva para el arrastre y para el menú.
- Arrastrar que funcione igual en Linux, macOS y Windows, y con el grid virtualizado.

**Non-Goals:**
- Mover varias fotos a la vez.
- Deshacer (Ctrl+Z) un movimiento; se corrige moviendo de nuevo.
- Reordenar con el teclado. "Mover al principio" y "Mover al final" cubren los saltos largos; se puede sumar después.

## Decisions

### D1. `position` entera, renumerada al mover

- **Esquema:** `album_item.position INTEGER NOT NULL DEFAULT 0` y `album.sort TEXT NOT NULL DEFAULT 'custom'` (`'custom'` | `'captured'`).
- **Migración** (`migrate_album_order_columns`, idempotente): si falta `position`, se agrega y se siembra por álbum con el orden actual (`added_at, id`) como `0..n-1`. Si falta `sort`, se agrega con `'custom'`. Así, la primera vez el orden personalizado es el de agregadas.
- **Mover** (`move_album_item(item_id, to_index)`): en una transacción, se leen los ids del álbum por `position`, se saca el elemento, se inserta en `to_index` (acotado a `0..n-1`) y se reescribe `position` de todos. Un álbum tiene de decenas a unos pocos miles de elementos; renumerar es trivial y deja siempre una secuencia densa y sin empates.
- **Agregar:** `position = MAX(position) + 1` del álbum, en el mismo `INSERT OR IGNORE`. **Reemplazar** actualiza la fila en su lugar y conserva `position`. **Quitar** deja un hueco que no altera el orden relativo; el siguiente movimiento renumera.
- **Leer:** `album_entries` ordena por `position, id`, y con `Captured` reordena por fecha como hoy.

**Alternativas descartadas:**
- Posiciones fraccionarias o cadenas LexoRank, para no renumerar. Ahorran escrituras que aquí no importan, y complican la migración y los empates.
- Guardar el orden como una lista JSON en `album`. Rompe la integridad con `album_item` al agregar, quitar y reemplazar.

### D2. API: un comando para mover, y el modo dentro del álbum

- `AlbumOrder` pasa a `Custom | Captured`: `Custom` sustituye a `Added` y se serializa como `"custom"`.
- **`move_album_item(item_id, to_index)`:** comando Tauri que usan el arrastre (índice destino), "Mover al principio" (`0`) y "Mover al final" (un índice mayor o igual a `n`, acotado).
- **`set_album_sort(album_id, sort)`** guarda el modo. `get_album(album_id, order)`: si `order` llega vacío, usa `album.sort`. `AlbumView` agrega `sort`, para que `AlbumDetail` abra en el modo guardado.

### D3. Arrastrar con eventos de puntero y aritmética del grid

Se implementa en `AlbumDetail` sin librería de drag and drop:
- **Inicio:** `pointerdown` en la tarjeta (botón principal, solo en `custom`), con `setPointerCapture`. El arrastre empieza al pasar un umbral de ~6 px. Así un clic sigue abriendo el visor y el menú de la tarjeta sigue funcionando.
- **Destino:** se calcula con la geometría del grid, sin medir el DOM: `col = floor((x + COL_GAP/2) / (cardWidth + COL_GAP))` y `row = floor((y + scrollTop) / rowHeight)`, dando `index = row * cols + col`, acotado. La mitad derecha de una tarjeta apunta a "después". Funciona aunque la fila de destino no esté montada.
- **Vista en vivo:** mientras arrastra, el grid se dibuja con la lista "sin la foto arrastrada y con un hueco en el destino", así las demás se apartan. La foto va como miniatura semitransparente en `position: fixed`, siguiendo el puntero.
- **Auto-scroll:** un bucle con `requestAnimationFrame` desplaza el contenedor cuando el puntero está a menos de ~64 px del borde superior o inferior, más rápido cuanto más cerca. Al desplazar se recalcula el destino.
- **Soltar:** se aplica el nuevo orden en la vista (optimista) y se llama a `move_album_item`. Si falla, se recarga el álbum y se avisa. `Esc`, `pointercancel` o soltar fuera del grid cancelan sin cambios.

**Alternativas descartadas:**
- HTML5 drag and drop (`draggable`). No funciona en WebView2 con `dragDropEnabled`, y su imagen de arrastre no se controla.
- `dnd-kit` sortable. Su modelo mide los elementos montados, y con un grid virtualizado por filas obliga a sortear los elementos que no existen en el DOM. La aritmética del grid lo resuelve en menos código y sin una dependencia nueva.

### D4. Menú y vista por fecha

- Con el álbum en `custom`, `extraMenuItems` de la tarjeta agrega "Mover al principio" y "Mover al final" antes de "Reemplazar por…" y "Quitar". No aparecen en el primer y en el último elemento, respectivamente.
- Con el álbum en `captured`, el grid no arranca arrastres y el menú no muestra las opciones de mover.
- El selector "Ordenar por" muestra "Personalizado" y "Fecha de captura" (`albums.order.custom`, que sustituye a `albums.order.added`, en `es.json` y `en.json`). Al cambiarlo, llama a `set_album_sort` y recarga.

## Risks / Trade-offs

- **[Conflicto entre clic y arrastre]** Un clic con un temblor mínimo podría iniciar un arrastre. → Umbral de ~6 px y solo con el botón principal. El menú de la tarjeta (botón ⋮) no arranca arrastres.
- **[`setPointerCapture` y el auto-scroll en WebKitGTK]** Con la captura del puntero, al desplazar el contenedor puede no llegar un `pointermove` nuevo. → El bucle de auto-scroll recalcula el destino con la última posición conocida del puntero, sin depender de nuevos eventos.
- **[Álbumes grandes]** Rehacer la lista en cada `pointermove` en un álbum de miles. → El destino solo cambia cuando cambia el índice; la lista derivada se memoriza por índice, y el virtualizador solo pinta las filas visibles.
- **[Escrituras concurrentes]** Agregar fotos al álbum desde otra vista mientras se arrastra. → El movimiento se aplica por id en una transacción; tras soltar se recarga el álbum desde la base.

## Migration Plan

La migración agrega dos columnas con valores que reproducen el comportamiento actual (orden de agregadas y modo personalizado). No hay vuelta atrás que hacer: una versión anterior ignora las columnas nuevas y sigue ordenando por `added_at`.

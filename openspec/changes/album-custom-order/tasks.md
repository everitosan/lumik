# Tasks

## 1. Orden en la base (`lumik-core`)

- [x] 1.1 Columnas `album_item.position` y `album.sort` en `global_schema.sql`, y `migrate_album_order_columns` en `migrations.rs` (D1): idempotente, siembra `position` por álbum con el orden de `added_at, id` y `sort = 'custom'`. Test en `db`: una base con un álbum creado sin las columnas conserva su orden de agregadas tras migrar, y correr la migración dos veces no cambia nada. Verificar con `cargo test -p lumik-core album`.
- [x] 1.2 `AlbumOrder::Custom` en lugar de `Added`; `album_entries` ordena por `position, id`; `add_album_items` agrega al final (`MAX + 1`); `replace_album_item` conserva `position`. Tests: agregar deja al final, reemplazar conserva el lugar, quitar no altera el orden relativo y `Captured` sigue ordenando por fecha. Verificar con `cargo test -p lumik-core album`.
- [x] 1.3 `move_album_item(item_id, to_index)` (D1: transacción y renumeración densa) y `set_album_sort` / el modo guardado en `album`, con `AlbumView.sort` y `ReadAlbum` usando el modo del álbum cuando no llega uno. Tests: los escenarios de la spec (`a b c d e` → `a e b c d`, mover al principio y al final, índice fuera de rango acotado, recordar el modo). Verificar con `cargo test -p lumik-core`.

## 2. Comandos del desktop

- [x] 2.1 Comandos Tauri `move_album_item` y `set_album_sort` en `commands/albums.rs`, registrados en `lib.rs`; `get_album` con el orden opcional (por defecto, el del álbum). Funciones en `lib/api.ts` y tipos en `lib/types.ts` (`AlbumOrder = 'custom' | 'captured'`, `AlbumView.sort`). Verificar con `cargo check` en `src-tauri` y `pnpm --filter @lumik/desktop build`.

## 3. Vista del álbum

- [x] 3.1 Selector "Ordenar por" con "Personalizado" y "Fecha de captura", que abre en el modo guardado y lo guarda al cambiarlo; claves `albums.order.custom` en `es.json` y `en.json`, sin `albums.order.added`. Verificar con `pnpm --filter @lumik/desktop build`, `jq empty` en los dos JSON y en la app: cambiar a "Fecha de captura", salir y volver al álbum.
- [x] 3.2 "Mover al principio" y "Mover al final" en el menú de la tarjeta, solo en "Personalizado" y sin la opción que no aplica en el primer y el último elemento (D4), con sus traducciones. Verificar en la app con un álbum de varias fotos: la foto queda primera o última y el visor recorre ese orden.
- [x] 3.3 Arrastrar para insertar en `AlbumDetail` (D3): eventos de puntero con umbral, destino por la geometría del grid, miniatura que sigue al cursor, hueco en vivo, auto-scroll en los bordes, `Esc` y soltar fuera cancelan, actualización optimista con recarga si falla, y desactivado en "Fecha de captura". Verificar en la app en Linux con un álbum de más de dos pantallas: insertar entre dos fotos, llevar una foto del principio al final con auto-scroll, cancelar con `Esc`, que un clic siga abriendo el visor, y que el orden se conserve al reabrir.
- [x] 3.4 Actualizar `docs/useCases/flujo-albumes.md` (reordenar: arrastre y menú → `move_album_item`; modo por álbum) y la tabla de comandos de `apps/desktop/CLAUDE.md`. Verificar leyendo los dos archivos.

## 4. Integración

- [ ] 4.1 Probar el arrastre en Windows o macOS con un build de la app (sin drag de HTML5, por `dragDropEnabled`). Correr `cargo test -p lumik-core`, `pnpm --filter @lumik/desktop build` y `openspec validate album-custom-order --strict`.

## Workflow follow-up

- Archivar el change después de probarlo en la app.

# Design

## Context

- `lumik.db` ya tiene el índice de búsqueda (`search_project`, `search_photo`, `search_media`), que conserva las fotos de discos desconectados con una miniatura de 128 px.
- Cada foto tiene un UUID estable (`photo.id`) y cada proyecto su `project.id`.
- Los archivos de `_exported/` se ligan a su foto por nombre (`sync_exported`, al abrir el proyecto) y tienen un id determinista `uuid5(project_id/nombre)`, el mismo que usan el respaldo y el portal (`exportedId`).
- Ver proposal.md para la motivación.

## Goals / Non-Goals

**Goals:**
- Álbumes de curación con originales y exportados que maduran solos al exportar.
- Funcionar sin los discos conectados para ver; con el disco, abrir y mostrar en carpeta.

**Non-Goals:**
- Exportar un álbum, bajar finales de la nube, compartirlo en el portal y sincronizarlo entre equipos (llegará ligado a la cuenta).
- Orden manual (se ordena por fecha en que se agregó o por fecha de captura).
- Selección múltiple en los grids (su propio change).
- Elegir portada (se usa el primer elemento).

## Decisions

### D1. Tablas en `lumik.db`

```
album(id PK, name, created_at, updated_at)
album_item(id PK, album_id FK ON DELETE CASCADE, kind 'photo'|'export',
           project_id, ref_id, file_name, project_name, added_at,
           UNIQUE(album_id, kind, ref_id))
search_export(export_id PK, project_id FK search_project ON DELETE CASCADE,
              file_name, photo_id NULL, signature, thumb BLOB)
```

- `album_item` guarda solo la referencia (`kind` + `ref_id`) y una copia del nombre del archivo y del proyecto, para poder mostrar un elemento cuya foto ya no existe. No guarda rutas.
- `search_export` es parte del índice (desechable): un renglón por archivo de `_exported/`, con su foto ligada si la hay. `signature` es tamaño + fecha de modificación, para rehacer la miniatura solo si el archivo cambió.
- Se descartó guardar los álbumes en un disco: un álbum cruza discos.

### D2. Indexar `_exported/`

`IndexProject` lista `_exported/` con un método nuevo del puerto `FileStore` (`list_files`, que devuelve nombre, tamaño y fecha de modificación). Liga cada archivo a la foto cuyo `jpg_path` lo nombra y saca la miniatura de la edición con `ImageProcessor::index_thumbnail`, conservando la anterior si `signature` no cambió. Corre cuando cambia la generación del proyecto, igual que las fotos.

Una exportación hecha con el proyecto cerrado aparece al abrirlo: `sync_exported` liga el archivo, esa escritura sube la generación y el proyecto se reindexa.

### D3. Mejor versión al leer el álbum

La versión se resuelve al leer, no al agregar, para que el álbum madure solo:

```
kind = photo  -> search_export con photo_id = ref_id ?  -> final (miniatura de la edición)
                 si no, search_photo                    -> sin editar (miniatura del original)
kind = export -> search_export por export_id            -> final
nada en el índice y su proyecto sin esa foto            -> "ya no está en su proyecto"
```

Un disco desconectado no hace que un elemento "ya no esté": el índice conserva sus filas. Solo cuenta como desaparecido si su proyecto está en el índice y la foto ya no, o si el proyecto se eliminó.

### D4. Abrir y mostrar en carpeta

- Abrir muestra el visor encima del álbum o de los resultados (`CollectionViewer`), sin entrar al proyecto: recibe la lista de fotos que se pueden abrir en el orden en que se ven, carga cada una de su proyecto y las flechas recorren esa lista aunque crucen proyectos. Como la vista de abajo no se desmonta, cerrar vuelve a la misma posición. Se descartó abrir el proyecto en esa foto (lo que hacía la búsqueda): las flechas recorrían el proyecto y no el conjunto.
- Mostrar en carpeta: comando `reveal_album_item`. Con el proyecto abierto, para un final revela `_exported/{archivo}`; para un sin editar delega en `reveal_photo` (original, o el JPG de cámara de un par). Sin proyecto abierto devuelve el disco que hay que conectar.

### D5. Agregar a un álbum

- `AddToAlbumMenu`: lista de álbumes más "Nuevo álbum…". Se abre desde el botón del visor y el atajo `photo_detail.add_to_album` (por defecto `a`), desde la opción "Agregar a álbum" del menú de `PhotoCard` (grid del proyecto y resultados de búsqueda) y desde "Agregar todos a un álbum" en el resumen de los resultados.
- Agregar varias es una sola transacción con `INSERT OR IGNORE`: lo repetido no se duplica.

### D6. Interfaz

- Sección "Álbumes" en la barra lateral: grid de álbumes (portada = primer elemento, nombre, total, finales y sin editar) y "Nuevo álbum".
- Detalle del álbum: nombre editable, contadores, aviso de discos faltantes, orden (agregado o captura) y grid virtualizado. El menú de cada elemento tiene abrir, mostrar en carpeta, reemplazar por… y quitar del álbum.
- "Reemplazar por…": modal con las fotos y los exportados del mismo proyecto que hay en el índice. Conserva la fecha en que se agregó, así el elemento no cambia de lugar.

## Risks / Trade-offs

- [En el visor de un conjunto no se toma el candado de vista de cada proyecto] → Cada escritura (calificar, aprobar, rotar) toma su sesión y falla con `project_locked` si el proyecto es de otra instalación.

- [Un exportado renombrado cambia de id] → El elemento pasa a "ya no está" y se reemplaza con "Reemplazar por…". Ligar por contenido queda para después.
- [Exportar con el proyecto cerrado no se refleja hasta abrirlo] → Documentado. Una revisión de `_exported/` al escanear discos podría agregarse si molesta.
- [La miniatura de un exportado se calcula del JPG completo (varios MB)] → Solo cuando el archivo cambia (`signature`), en segundo plano.

# Tasks

## 1. Índice de exportados (núcleo)

- [x] 1.1 Tabla `search_export` y `FileStore::list_files` (nombre, tamaño, fecha de modificación) con su implementación en el desktop. Verificar con un test de esquema y uno de infraestructura.
- [x] 1.2 `IndexProject` indexa `_exported/`: liga cada archivo a su foto por `jpg_path`, miniatura de la edición conservada mientras no cambie `signature`, y los archivos que ya no están salen. Verificar con tests: un exportado ligado, uno suelto, uno cambiado que rehace la miniatura y uno borrado.

## 2. Álbumes (núcleo)

- [x] 2.1 Tablas `album` y `album_item`, y consultas para crear, renombrar, borrar, agregar (varios, sin duplicar), quitar y reemplazar. Verificar con tests de cada operación y de que borrar un álbum borra sus elementos.
- [x] 2.2 Leer un álbum: mejor versión de cada elemento (final o sin editar), elementos que ya no están, disco conectado o no, discos faltantes con su conteo, y orden por agregado o por captura. Verificar con tests de cada caso de D3.
- [x] 2.3 Fotos candidatas para "Reemplazar por…": las fotos y exportados del mismo proyecto en el índice. Verificar con un test.

## 3. Comandos (desktop)

- [x] 3.1 Comandos de álbumes (lista, crear, renombrar, borrar, agregar, agregar resultados de búsqueda, quitar, reemplazar, leer, candidatos, miniatura de un elemento) y `reveal_album_item`. Verificar con `cargo test` y la compilación.

## 4. Interfaz

- [ ] 4.1 *(Código hecho: sección "Álbumes" con `AlbumsPage` y `AlbumDetail` (nombre editable, contadores, aviso de discos faltantes, orden, grid virtualizado, borrar con confirmación); falta la comprobación en `dev`.)* Sección "Álbumes" con el grid de álbumes y "Nuevo álbum"; detalle con nombre editable, contadores, aviso de discos faltantes, orden y grid virtualizado. Verificar en `dev`.
- [ ] 4.2 *(Código hecho: `AddToAlbumDialog` con crear álbum, botón y atajo `a` en el visor, opción en el menú de `PhotoCard` en el proyecto y en la búsqueda, "Agregar todas a un álbum" en los resultados; falta la comprobación en `dev`.)* `AddToAlbumMenu` y sus entradas: botón y atajo `a` en el visor (`photo_detail.add_to_album`, en Ajustes → Atajos), opción en el menú de `PhotoCard` en el proyecto y en la búsqueda, y "Agregar todos a un álbum" en los resultados. Verificar en `dev` los escenarios de "Agregar fotos".
- [ ] 4.3 *(Código hecho: abrir con vuelta al álbum (`PhotoReturn` en `App`), mostrar en carpeta (`reveal_album_item`), `ReplaceDialog` y quitar; falta la comprobación en `dev`.)* Menú de cada elemento: abrir (y volver al álbum al cerrar el visor), mostrar en carpeta, reemplazar por… y quitar. Verificar en `dev` con un disco conectado y otro desconectado.
- [x] 4.4 Textos `es`/`en` e historia del Storybook para las opciones nuevas de `PhotoCard`. Verificar con `tsc` del UI kit y del desktop.

## 5. Documentación y verificación

- [x] 5.1 Diagrama del flujo en `docs/useCases/` y comandos en `apps/desktop/CLAUDE.md`. Verificar que el Mermaid parsea.
- [ ] 5.2 Prueba en `dev`: armar un álbum con originales de dos proyectos, exportar una foto desde el editor y ver que pasa a final, mandar una a la papelera, desconectar un disco. Correr los tests del núcleo y del desktop, el build y `openspec validate photo-albums --strict`.

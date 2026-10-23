# Tasks

## 1. Preparación

- [ ] 1.1 Comprobar con `openspec list` si `import-feedback-and-errors` e `import-retry-failed` ya están archivados. Si no, rebasar esta rama sobre sus cambios del pipeline del import antes de empezar el grupo 3, para no reescribir el mismo código dos veces (design: Risks). Verificar que `cargo test` en `apps/desktop/src-tauri` pasa antes de tocar nada.

## 2. Modelo de datos y opción de entrega

- [ ] 2.1 Agregar a `photo` las columnas `camera_jpg_path`, `source_stem`, `delivered_size`, `delivered_sha256` y `delivery_kind` con `DEFAULT`, y a `project_settings` la columna `delivery_mode` (`none` por defecto), en `project_schema.sql` y en la migración aditiva (D1). Verificar con un test de migración que un `project.db` anterior abre y queda con `delivery_mode = 'none'`.
- [ ] 2.2 Agregar a la configuración global el valor de fábrica de la entrega (`none` en una instalación nueva) y copiarlo a `project_settings` al crear un proyecto. Verificar con tests: un proyecto nuevo toma el valor global, y cambiar el global después no cambia proyectos existentes.
- [ ] 2.3 Mostrar el valor de fábrica en Ajustes → Importación y la opción del proyecto en "Editar proyecto", como selección de tres valores con textos `es`/`en`. Verificar en `pnpm --filter @lumik/desktop tauri dev`: reiniciar conserva el global, y guardar en "Editar proyecto" no mueve la carpeta.

## 3. Detección de pares en el import

- [ ] 3.1 Implementar la clave de emparejamiento (stem de origen en minúsculas + fecha de captura al segundo) y el agrupamiento en pares y sueltos, con lectura de fechas en lote (exiftool `-fast2` en escritorio, rawler en Android) (D2). Verificar con tests unitarios los escenarios de `photo-pairs` "Detección del par al importar": misma tarjeta, doble ranura, mismo nombre en otra sesión, sin pareja y más de dos archivos con la misma clave.
- [ ] 3.2 Implementar `free_stem` y mover cada par con el mismo nombre, y registrar el par como una fila (`dng_path` = RAW, `camera_jpg_path` = JPG, `source_stem`) (D1, D3). Verificar con tests: ráfaga de tres pares en el mismo segundo (nombres limpio, `-001` y `-002`, cada par con el suyo) y renombrado desactivado.
- [ ] 3.3 Unir un JPG o un RAW suelto que llega a la foto del proyecto con su misma clave, conservando estrellas, color, tags, aprobación y rotación, y heredando el stem del archivo existente; aplicar lo mismo al reintento de fallidos (D2, D3). Verificar con tests: segunda tarjeta en otro import, JPG que se une a un RAW renombrado con `-001`, y JPG fallido que se reintenta.
- [ ] 3.4 Firmar los dos archivos del par con los metadatos del fotógrafo y generar la miniatura y la vista previa desde el JPG (D8). Verificar importando un par real de Fuji: los dos archivos llevan autor y la miniatura muestra la receta.
- [ ] 3.5 Mostrar en el paso de origen y en el resumen el desglose por tomas (pares, solo RAW, solo JPG, y cuántos JPG se unirán a fotos existentes) y contar tomas en el resultado; el registro sigue por archivo. Separar en el paso de ajustes el bloque "De este proyecto" con la opción de entrega, que restablecer no cambia. Verificar en `dev` con 300 pares, 3 RAW y 1 JPG: origen y resultado dicen 304 fotos y el registro lista 604 archivos.

## 4. El par como una sola foto

- [ ] 4.1 Escribir estrellas, color y tags en los dos archivos del par, y generar el AppleDouble de Finder tags para los dos (D7). Verificar con tests de la cola de escritura y leyendo con exiftool los dos archivos tras calificar un par en `dev`.
- [ ] 4.2 Aplicar la rotación a los dos archivos y regenerar la miniatura una vez (D7). Verificar con el test de rotación extendido a pares.
- [ ] 4.3 Mover a la papelera y restaurar los dos archivos del par con sus sidecars. Verificar con tests de los escenarios de `photo-pairs` "Papelera de un par".
- [ ] 4.4 "Mostrar en carpeta" de un par selecciona su JPG, esté en `_media/`, `_culled/` o `_exported/`. Verificar en `dev` con un par entregado.

## 5. Aprobar y entregar

- [ ] 5.1 Extender `CullPhoto` con los destinos según tipo de foto y `delivery_mode`: mover el JPG del par a `_exported/` o copiar el JPG suelto; guardar huella y `delivery_kind`, enlazar `jpg_path` y pasar a `edited`; no sobrescribir un archivo existente en `_exported/`; rollback en orden inverso si falla la BD (D4). Verificar con tests con fakes de todos los escenarios de "Destino de los archivos al aprobar" y de los fallos de BD a mitad.
- [ ] 5.2 Quitar la aprobación según la huella: regresar el JPG del par, borrar la copia del JPG suelto, o dejar en `_exported/` una edición del fotógrafo (D4). Verificar con tests de los escenarios de "Regreso del JPG entregado", incluido un JPG ya respaldado que no se toca en la nube.
- [ ] 5.3 Al unirse un JPG a una foto aprobada en el import, ejecutar la entrega con el valor vigente (D6). Verificar con el test "JPG de la segunda tarjeta para un RAW aprobado".
- [ ] 5.4 Calcular el plan al cambiar la opción de entrega con fotos aprobadas y aplicarlo foto por foto si se acepta; mostrar el aviso con los conteos (entregar, regresar, ediciones que se quedan, en la nube) en Editar proyecto y en el paso de ajustes del import (D5). Verificar con tests del plan y en `dev` los escenarios de "Aviso al cambiar la opción de entrega", en los dos sentidos y sin fotos afectadas.
- [ ] 5.5 Comprobar que un proyecto con pares entregados queda "Respaldado" tras respaldar, sin cambios en el respaldo. Verificar en `dev` aprobando y respaldando 5 pares con entrega de pares.

## 6. Grid y visor

- [ ] 6.1 Mostrar el indicador `RAW+JPG` / `RAW` / `JPG` en la esquina superior derecha de cada miniatura y del visor, legible sobre fotos claras y oscuras. Verificar en `dev` con un proyecto mixto y una foto con cielo blanco, en `es` y `en`.
- [ ] 6.2 En el visor, abrir los pares con su JPG y permitir cambiar al RAW desde el indicador con ratón y teclado; cachear la vista del RAW en `.previews/{id}-raw.jpg` y volver al JPG al cambiar de foto (D8). Verificar en `dev` los escenarios de "Ver el RAW o el JPG de un par".

## 7. Documentación y verificación

- [ ] 7.1 Actualizar los diagramas de `docs/useCases/` afectados (`flujo-importar-imagenes.md`, `flujo-culling.md`, `flujo-editar-proyecto.md`, `flujo-papelera-foto.md`, `flujo-rating-estrellas.md`, `flujo-rotacion.md`) con pares y entrega, y validar los bloques Mermaid con el parser. Verificar que cada diagrama refleja el comportamiento implementado.
- [ ] 7.2 Prueba de punta a punta con una tarjeta real de doble ranura en un disco mecánico: importar la tarjeta RAW, calificar y aprobar algunas, importar la tarjeta JPG en otro import, comprobar uniones y entregas, cambiar la opción a *nada* y de vuelta, y respaldar. Correr `cargo test`, `pnpm --filter @lumik/desktop build` y `openspec validate raw-jpeg-pairs --strict`.

## Workflow follow-up

- Archivar el change con `/opsx:archive` cuando la prueba 7.2 esté hecha.
- Decidir si la cifra de la landing (~150 RAW por minuto) cuenta tomas o archivos.

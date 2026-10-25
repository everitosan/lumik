# Tasks

## 1. Consulta (dominio)

- [ ] 1.1 `domain::search`: `SearchQuery` con operadores y texto libre, `parse` y `to_string` (ida y vuelta), comillas, operadores inválidos como texto libre y normalización sin mayúsculas ni acentos. Verificar con tests de dominio de cada operador y de la ida y vuelta.

## 2. Índice (núcleo)

- [ ] 2.1 Tablas `search_project` y `search_photo` en la base global, con columnas normalizadas para el texto libre y `embedding` vacío. Verificar con un test de esquema y de idempotencia.
- [ ] 2.2 Reindexar un proyecto desde su `project.db`: fotos fuera de la papelera, miniatura de 128 px desde `.thumbs/`, generación indexada; y quitar del índice los proyectos eliminados. Verificar con tests: una foto en la papelera sale, una calificación cambia el índice, otro proyecto no se toca.
- [ ] 2.3 Caso de uso de búsqueda: `SearchQuery` → SQL, orden por fecha, archivados fuera salvo que se pidan, y disco conectado o no por resultado. Verificar con tests de cada operador combinado sobre un índice de prueba.

## 3. Refresco (desktop)

- [ ] 3.1 Reindexar por generación al arrancar y en cada escaneo de discos, en segundo plano, proyecto por proyecto. Verificar con un test de que un proyecto con la misma generación no se reindexa.
- [ ] 3.2 Marcar el proyecto para reindexar al cerrar una `WriteSession`, con debounce. Verificar con un test de que dos escrituras seguidas reindexan una vez.

## 4. Interfaz

- [ ] 4.1 Comando `search_photos` y vista "Buscar" en la barra lateral, con el cuadro, resultados agrupados por proyecto y virtualizados, y estado vacío. Verificar con `tsc` y en `dev`.
- [ ] 4.2 Panel de búsqueda avanzada con sus campos, ida y vuelta con el cuadro. Verificar en `dev` los escenarios de "Búsqueda avanzada".
- [ ] 4.3 Abrir un resultado: visor del proyecto si el disco está conectado; miniatura y aviso del disco si no. Textos `es`/`en`. Verificar en `dev` con un disco conectado y otro desconectado.

## 5. Documentación y verificación

- [ ] 5.1 Diagrama del flujo en `docs/useCases/` y comandos en `apps/desktop/CLAUDE.md`. Verificar que el Mermaid parsea.
- [ ] 5.2 Prueba con el disco real: indexar todos sus proyectos, medir el tiempo de una búsqueda con 3 operadores y texto libre (objetivo < 200 ms) y el tamaño de `lumik.db`, desconectar el disco y repetir. Correr los tests del núcleo y del desktop, el build y `openspec validate photo-search --strict`.

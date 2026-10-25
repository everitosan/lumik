# Design

## Context

Ver proposal.md (Why). Estado actual relevante (tras `import-feedback-and-errors`, aún sin archivar):

- `start_import` recibe `ImportRequest { session_id, source_files, project_id, device_uuid, mount_point, project_name, settings? }` y devuelve siempre `Ok(ImportResult)` durante la importación. Tras el caso de uso, si hubo fotos importadas y el proyecto no tiene fecha, infiere la fecha de sesión y mueve la carpeta (`relocate_project_folder`).
- `FailedFile { name, path, code, error }`: para los fallos de copia, preparación, firma y movimiento, `path` es la **ruta de origen**. Para `register_failed` (`ImportPhotos::register`, `import_photos.rs:120-125`), `path` es la ruta **ya dentro de `_media/`**, y como `create_batch` es una sola transacción, falla el lote entero de ese intento.
- `ImportPage` guarda la selección (`importState.sourceFiles`, con `sizeBytes`), `sessionSettings`, el proyecto y el disco. El espacio libre sale de `devices`, que se leyeron antes de importar.
- `useImport()` limpia el resultado y el registro en cada `startImport`, y filtra los eventos por `session_id`.

## Goals / Non-Goals

**Goals:**
- Reintentar desde el resultado sin cambiar el contrato de `start_import`.
- Un camino aparte solo para registrar, que no copie, no borre y no duplique.

**Non-Goals:**
- Persistir el estado del reintento: si se cierra el wizard, el resultado se pierde, igual que hoy.
- Reintentos automáticos o con espera.

## Decisions

### D1. Los fallos de copia se reintentan con `start_import`

Para cualquier código distinto de `register_failed`, el frontend llama a `start_import` con `source_files` = las rutas de origen de esos fallidos, un `session_id` nuevo y los mismos `project_id`, `device_uuid`, `mount_point`, `project_name` y `settings`. Videos y fotos van mezclados: el backend ya los separa. El backend no cambia: el pipeline ya resuelve nombres sin sobrescribir y localiza el proyecto por `project_id`, aunque la carpeta se haya movido tras el primer intento.

*Alternativa:* un comando `retry_import` que reciba el `ImportResult` anterior. Duplica la lógica de `start_import` sin aportar nada.

### D2. Solo registrar: comando `retry_import_register`

```
retry_import_register({ session_id, project_id, project_name, file_names: string[] })
  -> ImportResult
```

- Recibe **nombres de archivo**, no rutas absolutas. Se resuelven como `{project_dir}/_media/{nombre}` contra la carpeta **actual** del proyecto. Así funciona aunque un reintento de copia del mismo lote haya disparado el renombrado de la carpeta por fecha: las rutas absolutas del primer intento ya no existirían. `FailedFile.name` ya es el nombre en `_media/`, así que no cambia el tipo.
- Un nuevo método del caso de uso, `ImportPhotos::register_existing(params, files)`, filtra antes de llamar al `register` actual (EXIF batch, `create_batch` y miniaturas):
  - si el archivo ya no está en `_media/` → fallido `register_failed` con detalle "no encontrado";
  - si su `dng_path` (`_media/{nombre}`) ya está en la BD → cuenta como importado, sin insertar;
  - el resto se registra.
- Para eso, `PhotoRepository` gana `registered_dng_paths(&[String]) -> Result<HashSet<String>, String>` (un `SELECT dng_path FROM photo WHERE dng_path IN (...)`, en tandas si hace falta).
- Emite los mismos eventos (`register` y `thumbnail`, y `photos_registered` en el registro) con el `session_id` nuevo.

*Alternativa (propuesta inicial):* borrar el archivo de `_media/` si "es el mismo" y reimportarlo desde el origen. Descartada: con firma y renombrado, el archivo de `_media/` nunca coincide con el de origen en nombre, tamaño ni metadatos, la ruta de origen no llega a `register_failed` y borrar es irreversible.

### D3. Lo que pasa tras importar se comparte

La inferencia de la fecha de sesión y el renombrado de la carpeta, hoy dentro de `start_import`, pasan a una función del adaptador (`finish_import(state, reporter, project_id, project_name, session_id, successful)`) que llaman los dos comandos. Si el primer intento solo tuvo `register_failed` (0 fotos importadas, no se infirió fecha), el reintento de registro es el que la infiere.

### D4. Orden y fusión en el frontend

"Reintentar" agrupa los fallidos en dos tandas y las ejecuta **en serie**: primero `retry_import_register` (rápido, no lee el origen), después `start_import` con el resto. Cada tanda tiene su `session_id`.

`useImport()` gana `retry(steps)`, que no limpia el registro ni el resultado. Una función pura `mergeRetryResult(prev, retried, next)` calcula el total:

| Campo | Total |
|-------|-------|
| `successful`, `videos_copied` | `prev + next` |
| `failed_files` | los de `prev` que no se reintentaron + los de `next` |
| `skipped_files`, `total_files` | los de `prev` (un reintento no agrega ignorados) |
| `fatal` | `next.fatal ?? prev.fatal` |

Si un comando devuelve `Err` (precondición: disco desmontado, sin fotógrafo), el resultado anterior se conserva intacto, se muestra el error y el botón sigue disponible. El tiempo total suma la duración de cada intento.

### D5. Espacio antes de reintentar

Antes de un reintento con copia, `ImportPage` vuelve a leer los dispositivos (`refetch`) y compara el espacio libre con la suma de `sizeBytes` de los orígenes que se van a reintentar (desde `importState.sourceFiles`, por ruta). Si no cabe, el botón queda desactivado con el mismo aviso de espacio que el paso de confirmación. La tanda de solo registrar no necesita espacio.

### D6. UI

El botón "Reintentar N" va en la cabecera de la lista de fallidos. "Aceptar" se queda en el pie. Mientras corre el reintento, el paso muestra la vista de progreso normal (con el registro acumulado debajo) y el pie vuelve a "Importando…". Todos los códigos de `ImportErrorCode` son reintentables con esta división, así que el botón aparece siempre que haya fallidos.

## Risks / Trade-offs

- [Un fallo determinista (archivo dañado) se reintenta en vano] → Es barato, vuelve a mostrarse con su motivo y el usuario decide si sigue.
- [El reintento oculta el bug de la firma pasajera (`sign_staged`)] → Fuera de alcance. El detalle técnico sigue en `FailedFile.error` y en el log para investigarlo.
- [El origen se remontó en otra ruta] → El reintento falla con `source_unreadable`. Aceptado: hay que volver a abrir el wizard.
- [Dos archivos en `_media/` con el mismo nombre que la BD] → Imposible: `free_name` garantiza nombres únicos en `_media/` y `dng_path` se deriva del nombre.

## Migration Plan

Sin migración. Comando y método de repositorio nuevos; el contrato de `start_import` y de los eventos no cambia.

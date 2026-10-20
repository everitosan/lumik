# Design

## Context

Ver proposal.md (Why) y los specs `backup-activity`, `app-sidebar`, `cloud-backup` y `backup-accounts`.

Lo que ya existe y condiciona el enfoque:

- Los respaldos ya corren a nivel de app: `BackupProvider` (`lib/BackupContext.tsx`) envuelve todo `App` y guarda `jobs: Record<projectId, { running, files[] }>`. El modal solo lee ese estado. Al arrancar, el provider reanuda la cola persistente (`get_pending_backups`), que guarda `project_id`, `targets` y `quality`, pero no el nombre del proyecto.
- `startBackup` recibe proveedores y calidad, los guarda en la cola y no los conserva en el job. Un job que termina se queda en memoria con `running: false` hasta cerrar la app.
- `BackupModal` vive dentro de `ProjectDetail` y decide al abrirse si muestra el progreso (`job?.running`) o la configuración.
- La sesión no es un estado compartido: `BackupAccountSection` (Ajustes → Respaldo) la carga con `get_backup_session` y la guarda en su propio estado, junto con la URL del servidor y los formularios de login y registro.
- La configuración de la instalación sigue un patrón de comandos por preferencia (`get/set_dashboard_show_archived` sobre `global_db`).
- `Sidebar` (`components/Sidebar.tsx`) usa `SectionButton` y `DriveInfo` de `@lumik/ui`. No hay ningún manejo del cierre de la ventana. La capability de Tauri incluye `core:default` y `dialog:default`.

## Goals / Non-Goals

**Goals:**
- Que la barra lateral sea una vista más del mismo estado de jobs, sin duplicar la lógica de subida.
- Que el modal de detalle sea uno solo, se abra desde el proyecto o desde la barra lateral.

**Non-Goals:**
- Cambiar la API del servidor o el flujo de subida archivo por archivo.
- Indicador de respaldos con la barra lateral colapsada en Android (ver proposal).
- Cancelar o pausar un respaldo.

## Decisions

### 1. El job se enriquece en `BackupProvider`
Cada job pasa a guardar `projectName`, `targets` y `quality`, además de `running` y `files`. `startBackup` recibe el nombre del proyecto. Al reanudar la cola al arrancar, el nombre se obtiene con `get_project(project_id)`, que ya se llama implícitamente porque el proyecto tiene que estar montado para listar sus pendientes.
- *Alternativa:* guardar el nombre en la cola persistente (`backup_queue`). Se descarta porque el nombre puede cambiar al renombrar el proyecto y duplicaría un dato que ya vive en `project.db`.

### 2. Ciclo de vida del job: retirar, reintentar y descartar en el provider
- Al terminar (`finish`), si no hubo errores el job se **elimina** de `jobs`. Si hubo errores, queda con `running: false` y sus archivos `error`.
- `retryBackup(projectId)` reutiliza el job: pasa a `pending` solo los archivos `error`, conserva los `done` y vuelve a ejecutar el pool con los `targets` y la `quality` del job. Así la tarjeta pasa de `21/23` a `23/23` en lugar de reiniciarse en `0/2`. Antes de subir llama a `get_project(projectId)`. Si falla porque el disco no está montado, no cambia nada y devuelve ese motivo para mostrarlo.
- `dismissBackup(projectId)` elimina el job de `jobs` sin tocar la cola persistente. El proyecto sigue en `backup_queue` y se reanuda en el próximo arranque.
- *Alternativa:* reintentar con `startBackup` y una lista nueva de archivos. Se descarta porque el avance de la tarjeta perdería los archivos ya subidos.

### 3. Contexto de sesión compartido; la cuenta se queda en Ajustes
Nuevo `BackupSessionProvider` (envolviendo a `BackupProvider` en `App`) que expone `session`, `loading`, `refresh`, `login`, `register` y `logout`. Lo usan Ajustes → Respaldo, el aviso de respaldar sin sesión y `BackupProvider` (para reanudar la cola), así que iniciar o cerrar sesión se refleja en todos sin recargar.
- La cuenta se gestiona solo en Ajustes → Respaldo: el campo de URL siempre y, debajo, el `BackupAccountForm` (login / registro) sin sesión o el email con "Cerrar sesión" con sesión. La barra lateral no muestra nada de la cuenta: el email ocuparía una línea fija en 260 px sin aportar durante una subida.
- *Alternativa:* login, registro y logout en la barra lateral, con un modal de cuenta. Se probó y se descartó: la sección tendría que estar siempre visible solo para invitar a crear la cuenta, y ocupa espacio el resto del tiempo.
- *Alternativa:* emitir un evento global al cambiar la sesión y que cada vista recargue. Se descarta porque es más frágil que un estado único y el patrón de contexto ya está en uso (`BackupProvider`).

### 3b. Respaldar sin sesión: aviso y salto a Ajustes → Respaldo
`BackupModalHost` lee la sesión: si no hay sesión ni job para el proyecto, en lugar de `BackupModal` muestra un aviso (`BackupNoSessionModal`) con "Ir a Ajustes". Si hay job (p. ej. se cerró sesión a mitad de la subida), se muestra el progreso igual.
- Para llevar a la pestaña, `App` gana `openSettings(tab)`, que cambia a la sección Ajustes, cierra el proyecto abierto y pasa `initialTab` a `SettingsPage` (hoy la pestaña es estado interno con `'import'` por defecto). El host recibe `onOpenSettings` de `App`.
- Tras iniciar sesión el fotógrafo vuelve al proyecto a mano: no se guarda a qué proyecto volver.
- *Alternativa:* incluir `BackupAccountForm` dentro del aviso y pasar directo a elegir proveedores. Se descarta por decisión de producto: la cuenta se gestiona en un único sitio.

### 3c. Visibilidad de Respaldos y reanudación con sesión
- La sección Respaldos de la barra lateral se renderiza solo si `jobs` no está vacío (en curso o con errores), sin mirar la sesión: así una tarjeta con errores mantiene "Reintentar" y "Descartar" a mano, y cerrar sesión a mitad de una subida no esconde su avance.
- `BackupProvider` reanuda la cola la primera vez que hay sesión en la ejecución (al arrancar si la hay, o cuando el fotógrafo inicia sesión). Sin sesión no intenta nada: las subidas fallarían con 401 y aparecerían tarjetas en error sin motivo.

### 4. Un solo host para el modal de detalle
El provider expone `openBackupDetail(projectId)` y `closeBackupDetail()`, y un `BackupModalHost` montado una sola vez en `App` renderiza `BackupModal` para el proyecto abierto. `ProjectDetail` deja de montar su propio modal y llama a `openBackupDetail`.
- `BackupModal` gana una forma explícita de abrirse en la vista de progreso: desde la barra lateral siempre hay un job, incluido uno terminado con errores. En ese caso la vista de progreso muestra "Reintentar" en el pie.
- *Alternativa:* montar otro `BackupModal` en `Sidebar`. Se descarta porque dos instancias del mismo modal podrían abrirse a la vez y tendrían que sincronizar su estado.

### 5. Secciones colapsables y su persistencia
- Se añade a `@lumik/ui` un `CollapsibleSection` (encabezado con título, resumen opcional y chevron) y un `BackupJobCard` (nombre, avance, barra, estado de error, "Reintentar" y ×), con stories en `apps/uikit`. La barra lateral ya consume sus piezas (`SectionButton`, `DriveInfo`) de la UI kit, así que se sigue el mismo criterio.
- El estado colapsado se guarda en `global_db` con un par de comandos, `get_sidebar_sections` y `set_sidebar_section_collapsed(section, collapsed)`, siguiendo el patrón de `dashboard_show_archived`. En una instalación nueva ambas secciones están expandidas.
- La lista de respaldos tiene una altura máxima y scroll propio, y Almacenamiento conserva su lugar al pie.
- *Alternativa:* `localStorage`. Se descarta porque el resto de preferencias de la instalación viven en `global_db` y el spec pide que sobrevivan a reinicios de forma fiable.

### 6. Expandir Respaldos solo ante un respaldo lanzado por el usuario
El provider distingue los jobs que lanza el usuario de los reanudados al arrancar, y expone un contador que solo sube con los primeros. La barra lateral expande Respaldos cuando ese contador cambia. Los reanudados no fuerzan la expansión, para respetar al fotógrafo que la dejó colapsada.

### 7. Confirmación al cerrar desde el frontend
El estado de los jobs vive en React, así que el cierre se intercepta allí: `getCurrentWindow().onCloseRequested` cancela el evento si hay algún job `running` y pregunta con `ask` de `tauri-plugin-dialog` (ya incluido en `dialog:default`). Si el fotógrafo confirma, se cierra con `destroy()`, lo que requiere añadir `core:window:allow-destroy` a la capability.
- *Alternativa:* `on_window_event(CloseRequested)` en Rust con `prevent_close`. Se descarta porque obligaría a duplicar en Rust el estado de los jobs o a un ida y vuelta por eventos solo para saber si hay subidas.

## Risks / Trade-offs

- [`onCloseRequested` no se dispara si la app se mata desde fuera (kill, apagado)] → Aceptado: el spec habla de cerrar la ventana. La cola persistente cubre esos casos y reanuda al arrancar.
- [Un job descartado vuelve a aparecer al reiniciar] → Es lo que pide el spec: descartar quita la tarjeta, no cancela el respaldo. El texto de la acción lo deja claro.
- [Reintentar con un disco que se desmonta a mitad] → Los archivos vuelven a `error` y la tarjeta sigue visible, igual que hoy.
- [Mover el modal a un host global cambia cómo lo abre `ProjectDetail`] → Es un cambio pequeño y se verifica que el flujo de configuración (elegir proveedores y calidad) sigue igual.
- [Con la barra lateral estrecha (260 px), nombres de proyecto largos] → La tarjeta trunca el nombre con elipsis y el avance siempre se ve.

## Migration Plan

Sin migración de datos: las preferencias nuevas se crean al primer uso con su valor por defecto (expandidas), y la cola de respaldos no cambia de formato. Para hacer rollback basta con revertir el código.

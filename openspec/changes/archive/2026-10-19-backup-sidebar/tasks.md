# Tasks

## 1. Estado de los respaldos en `BackupProvider`

- [x] 1.1 Ampliar el job con `projectName`, `targets` y `quality`, y hacer que `startBackup` reciba el nombre del proyecto (`ProjectDetail` / `BackupModal` lo pasan). Al reanudar la cola al arrancar, obtener el nombre con `get_project`. Verificar con `tsc --noEmit` del desktop y a mano que, al reiniciar la app con un respaldo en cola, el job reanudado tiene el nombre del proyecto.
- [x] 1.2 Al terminar un job sin errores, eliminarlo de `jobs`; con errores, dejarlo con `running: false`. Verificar a mano con un respaldo correcto (el job desaparece) y con uno que falla, por ejemplo cortando la red (el job se queda con sus archivos en error).
- [x] 1.3 Añadir `retryBackup(projectId)` (solo pasa a `pending` los archivos en error, conserva los subidos y reutiliza `targets` y `quality`; antes comprueba con `get_project` que el disco está montado y devuelve el motivo si no) y `dismissBackup(projectId)` (quita el job sin tocar la cola persistente). Verificar a mano: reintentar con el disco conectado sube solo los fallidos; reintentar con el disco desconectado no cambia nada y devuelve el motivo; descartar y reiniciar vuelve a reanudar el respaldo.
- [x] 1.4 Exponer en el provider un contador de respaldos lanzados por el usuario (no lo suben los reanudados al arrancar) y el par `openBackupDetail(projectId)` / `closeBackupDetail()`. Verificar con `tsc --noEmit`.
- [x] 1.5 Reanudar la cola solo con sesión: la primera vez que `BackupSessionProvider` tiene sesión en la ejecución (al arrancar o al iniciar sesión). Sin sesión no se intenta ninguna subida. Verificar a mano: arrancar sin sesión con un respaldo en cola (no aparece la sección ni hay errores); iniciar sesión en Ajustes (se reanuda y aparece su tarjeta).

## 2. Sesión de respaldo compartida

- [x] 2.1 Crear `BackupSessionProvider` (`session`, `loading`, `refresh`, `login`, `register`, `logout`) envolviendo a `BackupProvider` en `App`, y extraer los formularios de login y registro a un `BackupAccountForm` que usa el provider. Verificar con `tsc --noEmit` y a mano que el login y el registro siguen funcionando.
- [x] 2.2 Ajustes → Respaldo: el campo de URL del servidor siempre; sin sesión, `BackupAccountForm`; con sesión, el email y el servidor con "Cerrar sesión". Verificar a mano: cambiar la URL e iniciar sesión usa el servidor nuevo; cerrar sesión vuelve al formulario.
- [x] 2.3 Actualizar `docs/useCases/wip-flujo-login-cuenta.md`: login, registro y logout desde Ajustes → Respaldo, y el aviso al respaldar sin sesión que lleva ahí. Verificar que el diagrama se renderiza en la vista previa de Markdown.

## 3. Componentes de la UI kit

- [x] 3.1 Añadir a `packages/ui` un `CollapsibleSection` (título, resumen opcional en el encabezado, chevron, `collapsed` y `onToggle` controlados) con su story en `apps/uikit` (expandida, colapsada con resumen, colapsada sin resumen). Verificar que `pnpm --filter @lumik/ui build` pasa y que la story alterna con clic y con teclado.
- [x] 3.2 Añadir a `packages/ui` un `BackupJobCard` (nombre truncado con elipsis, avance `n/total` con barra, estado de error con número de fallidos, "Reintentar" y ×, clic en la tarjeta) con textos i18n `es`/`en` y story (en curso, con errores, nombre largo). Verificar en la story que "Reintentar" y × no disparan el clic de la tarjeta y que `pnpm --filter @lumik/ui build` pasa.

## 4. Preferencias de la barra lateral (Rust)

- [x] 4.1 Añadir a `global_db` el estado colapsado de las secciones `backups` y `storage` (expandidas por defecto), con los comandos `get_sidebar_sections` y `set_sidebar_section_collapsed(section, collapsed)` registrados en `lib.rs` y en `lib/api.ts`. Verificar con un test de `global_db` sobre una BD temporal: valores por defecto, guardado y lectura, y rechazo de una sección desconocida. `cargo test` y `cargo clippy` pasan.

## 5. Barra lateral

- [x] 5.1 Envolver Almacenamiento en `CollapsibleSection`, con el número de discos como resumen y el estado leído y guardado con los comandos de 4.1. Verificar a mano que, tras colapsarla y reiniciar, sigue colapsada.
- [x] 5.2 Añadir la sección Respaldos con `CollapsibleSection`, renderizada solo si hay jobs (en curso o con errores), sin mirar la sesión: una `BackupJobCard` por job, con altura máxima y scroll propio, sin datos de la cuenta. Resumen colapsado: en curso + avance total, o número con errores si no hay ninguno en curso. Se expande cuando sube el contador de 1.4. Textos i18n `es`/`en`. Verificar a mano los escenarios de `backup-activity` y `app-sidebar`: sin respaldos la sección no aparece, dos respaldos a la vez, la sección se oculta al terminar el último, cerrar sesión a mitad no la oculta, resumen colapsado, expandir al lanzar y scroll con muchos respaldos sin tapar Almacenamiento.

## 6. Modal de detalle global

- [x] 6.1 Montar un `BackupModalHost` una sola vez en `App` que renderiza `BackupModal` para el proyecto de `openBackupDetail`, y hacer que `ProjectDetail` lo use en lugar de su propio modal. `BackupModal` se abre en la vista de progreso cuando hay job, incluido uno terminado con errores, y en ese caso muestra "Reintentar". Conectar el clic de `BackupJobCard` a `openBackupDetail`. Verificar a mano: abrir el detalle desde Ajustes y cerrarlo sin salir de Ajustes; el flujo de configuración desde el proyecto (proveedores y calidad) sigue igual; "Reintentar" en el modal sube solo los fallidos.
- [x] 6.3 Respaldar sin sesión: `BackupModalHost` muestra, si no hay sesión ni job del proyecto, un `BackupNoSessionModal` con "Ir a Ajustes" en lugar de `BackupModal`. `App` gana `openSettings(tab)` (sección Ajustes, cierra el proyecto) y `SettingsPage` recibe `initialTab`. Textos i18n `es`/`en`. Verificar a mano: sin sesión, "Respaldar" muestra el aviso y "Ir a Ajustes" abre la pestaña Respaldo con el formulario; con sesión, el modal de respaldo sigue igual; entrar a Ajustes desde la barra sigue abriendo la pestaña Importación.
- [x] 6.2 Actualizar `docs/useCases/wip-flujo-respaldar-exported.md` con la barra lateral (sección visible solo con jobs, tarjetas, reintentar, descartar, fin del job, reanudar con sesión y confirmación al cerrar). Verificar que el diagrama se renderiza en la vista previa de Markdown.

## 7. Confirmación al cerrar la ventana

- [x] 7.1 Interceptar el cierre con `getCurrentWindow().onCloseRequested`: si hay algún job `running`, cancelar y preguntar con `ask` de `tauri-plugin-dialog` indicando cuántos respaldos están en curso; si se confirma, cerrar con `destroy()`. Añadir `core:window:allow-destroy` a la capability. Textos i18n `es`/`en`. Verificar a mano en Linux: cerrar con un respaldo en curso y cancelar (sigue subiendo); confirmar (se cierra y al reabrir se reanuda); cerrar con solo una tarjeta con errores (se cierra sin preguntar).

## 8. Verificación integrada

- [x] 8.1 En Linux con el disco KINGSTON: lanzar respaldos de dos proyectos, navegar por dashboard y Ajustes siguiendo el avance en la barra, forzar un error, reintentarlo desde la tarjeta, descartar otro y comprobar que se reanuda al reiniciar.
- [x] 8.2 Actualizar `apps/desktop/CLAUDE.md` (barra lateral, contextos de respaldo y sesión, comandos nuevos) y ejecutar `openspec validate backup-sidebar --strict`, verificando que pasa sin errores.

## Workflow follow-up

- Indicador de respaldos para Android con la barra lateral colapsada: se resolverá al trabajar esa plataforma.
- Archivar el change con `/opsx:archive` cuando la verificación integrada esté completa.

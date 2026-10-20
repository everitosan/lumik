# Proposal

## Why

Un respaldo se lanza desde el proyecto y sigue en segundo plano, pero su avance solo se ve en el modal de ese proyecto: para saber cómo va hay que volver a entrar en él y reabrir el modal. Sin ningún indicador fuera de esa pantalla es fácil cerrar la app a mitad de una subida sin darse cuenta, y los archivos que fallan solo se reintentan en el siguiente arranque.

## What Changes

- La barra lateral gana una sección **Respaldos**, visible desde cualquier pantalla solo mientras haya respaldos en curso o con errores; el resto del tiempo no ocupa espacio:
  - Una tarjeta por cada respaldo con el nombre del proyecto y su avance (`1/20`). Sin datos de la cuenta.
  - Clic en una tarjeta: abre el modal de detalle archivo por archivo que hoy se abre dentro del proyecto.
  - Un respaldo que termina sin errores sale de la lista. Uno con errores se queda visible con **Reintentar** (relanza solo los archivos fallidos, y avisa si el disco no está montado) y **Descartar** (quita la tarjeta; el respaldo sigue en la cola y se reintenta al arrancar).
- Las secciones **Respaldos** y **Almacenamiento** de la barra lateral se pueden colapsar. El estado se guarda entre reinicios y el encabezado colapsado sigue mostrando un resumen (respaldos en curso y avance total, errores, número de discos). Al lanzar un respaldo nuevo, Respaldos se expande.
- Si hay respaldos en curso, cerrar la ventana pide confirmación.
- La cuenta se sigue gestionando en **Ajustes → Respaldo** (URL del servidor, registro, login y "Cerrar sesión" con el email de la cuenta). Al pulsar "Respaldar" en un proyecto sin sesión, aparece un aviso con un enlace que abre Ajustes en la pestaña Respaldo.
- La cola de respaldos solo se reanuda con sesión: al arrancar si ya la hay, o al iniciar sesión.

### Fuera de alcance

- Android: con la barra lateral colapsada por defecto, la sección no se ve. El indicador para Android se resolverá al trabajar esa plataforma.
- Cancelar o pausar un respaldo en curso.

## Capabilities

### New Capabilities
- `backup-activity`: seguimiento de los respaldos en curso desde la barra lateral — tarjetas de avance, acceso al detalle, reintentar y descartar fallidos y confirmación al cerrar la app con subidas pendientes.
- `app-sidebar`: organización de la barra lateral del desktop en secciones colapsables, con su estado persistente y un resumen visible en el encabezado colapsado.

### Modified Capabilities
- `cloud-backup`: el modal de progreso de un respaldo deja de abrirse solo desde su proyecto; también se abre desde la barra lateral, desde cualquier pantalla. Respaldar sin sesión muestra un aviso con acceso a Ajustes → Respaldo, y la cola solo se reanuda con sesión.
- `backup-accounts`: Ajustes → Respaldo muestra el email de la sesión y la acción de cerrarla, además del registro y el login.

## Impact

- `apps/desktop` (React):
  - `lib/BackupContext.tsx`: cada job guarda nombre del proyecto, proveedores y calidad; acciones de reintentar y descartar; limpieza de jobs terminados sin errores.
  - Nuevo contexto de sesión de respaldo compartido entre Ajustes, el aviso sin sesión y la reanudación de la cola.
  - `components/Sidebar.tsx`: secciones colapsables y sección Respaldos visible solo con respaldos.
  - `pages/BackupAccountSection.tsx`: URL del servidor, formulario de login y registro, y email con "Cerrar sesión".
  - `BackupModal` se abre también desde la barra lateral, fuera de `ProjectDetail`; sin sesión, en su lugar se muestra el aviso.
  - `App` / `SettingsPage`: abrir Ajustes directamente en la pestaña Respaldo.
  - i18n `es` / `en`.
- `apps/desktop` (Rust): guardar y leer el estado colapsado de las secciones en la configuración global; nombre del proyecto disponible al reanudar la cola; confirmación al cerrar la ventana con respaldos activos.
- `packages/ui`: posible componente de sección colapsable y de tarjeta de respaldo, con sus stories en `apps/uikit`.
- Sin cambios en `apps/server` ni `apps/web`.

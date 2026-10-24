# web-portal Specification

## Purpose

Da al fotógrafo acceso desde el navegador (incluido el iPad en campo) a sus proyectos del catálogo, su estado de respaldo y sus proveedores, y le permite respaldar fotos sin tener el desktop ni el disco a mano.

## Requirements

### Requirement: Rutas protegidas
El portal SHALL tener páginas públicas de inicio de sesión y registro, y páginas protegidas de dashboard (`/`) y configuración (`/config`). Al entrar sin sesión a una página protegida, SHALL redirigir al inicio de sesión. Cualquier ruta desconocida SHALL llevar al dashboard.

#### Scenario: Entrar sin sesión
- **WHEN** un visitante sin sesión abre `/config`
- **THEN** se le redirige al inicio de sesión

#### Scenario: Ruta desconocida
- **WHEN** un usuario con sesión abre `/cualquier-cosa`
- **THEN** se le lleva al dashboard

### Requirement: Dashboard de proyectos
El dashboard SHALL mostrar los proyectos del catálogo del fotógrafo agrupados por año, de más reciente a más antiguo. Dentro de cada año SHALL ordenarlos por fecha ascendente (de sesión, o de creación si no hay). Cada tarjeta SHALL mostrar portada, número de fotos, cuántas están respaldadas, estado e iconos de los proveedores usados.

#### Scenario: Proyectos de dos años
- **WHEN** el fotógrafo tiene proyectos de 2025 y 2026
- **THEN** primero aparece el grupo 2026 y después el 2025

### Requirement: Estado del proyecto en web
La tarjeta SHALL usar el estado de flujo fijado por el desktop si existe. Si no, SHALL derivarlo de los contadores: `delivered` si todas las fotos están respaldadas, `edited` si hay alguna, e `imported` si no hay ninguna.

#### Scenario: Sin estado del desktop
- **WHEN** un proyecto sin estado fijado tiene 10 fotos y 4 respaldadas
- **THEN** la tarjeta muestra el estado `edited`

### Requirement: Abrir la ubicación del respaldo
Al pulsar el icono de un proveedor en una tarjeta, el portal SHALL abrir en una pestaña nueva el enlace con caducidad a la carpeta del proyecto, si el proveedor lo admite, o la web del proveedor en caso contrario o si falla.

#### Scenario: Icono de Dropbox
- **WHEN** el fotógrafo pulsa el icono de Dropbox de un proyecto respaldado
- **THEN** se abre una pestaña con la carpeta del proyecto en Dropbox

### Requirement: Subida desde el navegador
Al abrir un proyecto, el portal SHALL permitir elegir imágenes del dispositivo, los proveedores destino y la calidad (alta por defecto), y subirlas con hasta 6 subidas en paralelo. Cada subida fallida SHALL reintentarse hasta 3 veces con espera creciente, y el progreso SHALL mostrarse por archivo. Las fotos SHALL subir contra el id determinista del archivo, igual que en el desktop.

#### Scenario: Subir desde el iPad
- **WHEN** el fotógrafo elige 12 fotos en el iPad y pulsa subir
- **THEN** cada foto muestra su progreso y al terminar el proyecto cuenta 12 fotos respaldadas más

#### Scenario: Fallo transitorio
- **WHEN** la subida de una foto falla una vez por la red
- **THEN** se reintenta sola y termina como subida

### Requirement: Evitar duplicados al subir
El portal SHALL marcar como ya subidas, y no volver a subir, las fotos cuyo nombre corresponda a un archivo ya respaldado en ese proyecto. Tampoco SHALL añadir dos veces a la lista un archivo con el mismo nombre.

#### Scenario: Foto ya respaldada desde el desktop
- **WHEN** el fotógrafo elige `foto1.jpg` y el desktop ya la respaldó en ese proyecto
- **THEN** la foto aparece como ya subida y no se envía

### Requirement: Reflejo en el desktop
Lo subido desde el portal SHALL contar como respaldado al abrir el proyecto en el desktop, que no SHALL ofrecer volver a subirlo. Las fotos que solo existen en el iPad, sin archivo en `_exported/` del disco, SHALL respaldarse en la nube pero no aparecer en el grid del desktop.

#### Scenario: Volver al estudio
- **WHEN** el fotógrafo subió desde el iPad un archivo que también está en `_exported/` y abre el modal de respaldo en el desktop
- **THEN** ese archivo no aparece entre los pendientes

### Requirement: Configuración de proveedores
La página de configuración SHALL listar los proveedores conectados con opción de desconectarlos, y SHALL ofrecer un formulario para conectar Nextcloud o Mega con sus campos. También SHALL indicar que los proveedores OAuth llegarán más adelante.

#### Scenario: Conectar Mega desde web
- **WHEN** el fotógrafo elige Mega, escribe email y contraseña y envía
- **THEN** Mega aparece en la lista de proveedores conectados

### Requirement: Borrar el respaldo de un proyecto
El dashboard del portal SHALL ofrecer "Borrar respaldo" en el menú de la tarjeta de los proyectos con al menos una foto respaldada, y SHALL pedir escribir el nombre del proyecto para confirmar. El desktop no SHALL ofrecer esta acción.

#### Scenario: Confirmación con el nombre
- **WHEN** el fotógrafo abre "Borrar respaldo" y no escribe el nombre exacto del proyecto
- **THEN** el botón de borrar sigue desactivado

#### Scenario: Proyecto sin fotos respaldadas
- **WHEN** un proyecto del catálogo no tiene fotos respaldadas
- **THEN** su tarjeta no ofrece "Borrar respaldo"

### Requirement: Efecto de borrar el respaldo
Al confirmar el borrado, el servidor SHALL borrar la carpeta del proyecto en cada proveedor conectado y después las fotos del proyecto en el catálogo, y SHALL quitar el estado fijado por el desktop. Una carpeta que ya no exista en la nube no SHALL contar como error, y si el borrado falla en algún proveedor el catálogo no SHALL cambiar. El proyecto SHALL seguir en el catálogo y el siguiente respaldo SHALL subir de nuevo todas sus fotos.

#### Scenario: Volver a respaldar tras borrar en la nube a mano
- **WHEN** el fotógrafo borró en Mega la carpeta de un proyecto, borra su respaldo desde el portal y luego respalda el proyecto desde el desktop
- **THEN** el borrado termina sin error y el desktop ofrece subir todas las fotos de `_exported/`

#### Scenario: Respaldo de otro fotógrafo
- **WHEN** alguien intenta borrar el respaldo de un proyecto que no es suyo
- **THEN** el servidor responde que no existe y no borra nada

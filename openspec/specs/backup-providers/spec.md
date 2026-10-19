# backup-providers Specification

## Purpose

Permite a cada fotógrafo conectar sus propias cuentas de nube como destinos de respaldo, de modo que el servidor de Lumik pueda subir en su nombre sin que las credenciales se expongan ni se compartan entre usuarios.

## Requirements

### Requirement: Proveedores conectables por credenciales
El servidor SHALL permitir conectar Nextcloud (URL, usuario y contraseña de aplicación) y Mega (email y contraseña). Si falta algún campo o está vacío, la conexión SHALL rechazarse indicando qué campo falta. Un tipo de proveedor desconocido SHALL rechazarse.

#### Scenario: Conectar Nextcloud
- **WHEN** el fotógrafo envía URL, usuario y contraseña de aplicación de su Nextcloud
- **THEN** el proveedor queda conectado y aparece en su lista

#### Scenario: Falta un campo
- **WHEN** se intenta conectar Mega sin contraseña
- **THEN** se rechaza con el mensaje "falta el campo 'password'"

#### Scenario: Proveedor desconocido
- **WHEN** se intenta conectar un proveedor `icloud`
- **THEN** se rechaza como proveedor desconocido

### Requirement: Proveedores OAuth reservados
Google Drive, Dropbox y OneDrive SHALL reconocerse como proveedores válidos, pero mientras no estén configuradas las aplicaciones OAuth de Lumik, intentar conectarlos SHALL responder 501 con un mensaje que lo explique. La interfaz SHALL indicar que llegarán más adelante.

#### Scenario: Intentar conectar Google Drive
- **WHEN** el fotógrafo intenta conectar Google Drive
- **THEN** el servidor responde 501 indicando que el proveedor está pendiente de configurar

### Requirement: Un remote por proveedor y fotógrafo
Cada fotógrafo SHALL tener como máximo una conexión por tipo de proveedor. Volver a conectar un proveedor ya conectado SHALL reemplazar sus credenciales.

#### Scenario: Cambiar la contraseña de Mega
- **WHEN** el fotógrafo vuelve a conectar Mega con una contraseña nueva
- **THEN** sigue habiendo una sola conexión Mega, con la contraseña nueva

### Requirement: Credenciales cifradas y nunca expuestas
Las credenciales SHALL guardarse cifradas con AES-256-GCM, con una clave de servidor y un nonce aleatorio por cada valor. Ninguna respuesta de la API SHALL incluirlas. La configuración para subir SHALL generarse en el momento a partir de ellas, en un archivo temporal por operación, sin una configuración global compartida entre fotógrafos.

#### Scenario: Listar proveedores
- **WHEN** el fotógrafo lista sus proveedores
- **THEN** recibe el id, el tipo, el nombre y la fecha de conexión, sin credenciales

#### Scenario: Inspección de la base de datos
- **WHEN** se inspecciona la tabla de remotes
- **THEN** las credenciales solo aparecen cifradas

### Requirement: Desconectar proveedor
El fotógrafo SHALL poder desconectar uno de sus proveedores. Intentar desconectar uno que no existe o que pertenece a otro fotógrafo SHALL responder 404.

#### Scenario: Desconectar
- **WHEN** el fotógrafo desconecta su Nextcloud
- **THEN** deja de aparecer en su lista y no se usa en los respaldos siguientes

#### Scenario: Proveedor ajeno
- **WHEN** el fotógrafo A intenta desconectar un proveedor del fotógrafo B
- **THEN** la respuesta es 404 y el proveedor de B sigue conectado

### Requirement: Gestión desde el portal web
La conexión y desconexión de proveedores SHALL hacerse desde la página de configuración del portal web. El desktop SHALL listar los proveedores conectados para elegir destinos al respaldar.

#### Scenario: Respaldar tras conectar en web
- **WHEN** el fotógrafo conecta Mega en el portal y después abre el modal de respaldo en el desktop
- **THEN** Mega aparece como destino seleccionable

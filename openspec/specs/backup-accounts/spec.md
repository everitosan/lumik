# backup-accounts Specification

## Purpose

Da a cada fotógrafo una cuenta propia en el servidor de respaldo de Lumik, compartida entre el desktop y el portal web, que aísla sus proyectos, fotos y credenciales de nube de los de cualquier otro usuario.

## Requirements

### Requirement: Registro de cuenta
El servidor SHALL permitir registrarse con email, contraseña y un nombre opcional. El email SHALL normalizarse (sin espacios y en minúsculas) y ser único. SHALL rechazarse un email sin `@` o de menos de 3 caracteres, y una contraseña de menos de 8. Un registro correcto SHALL iniciar sesión de inmediato.

#### Scenario: Registro correcto
- **WHEN** alguien se registra con `Ana@Estudio.mx ` y una contraseña de 10 caracteres
- **THEN** se crea la cuenta `ana@estudio.mx` y se devuelve una sesión activa

#### Scenario: Email ya registrado
- **WHEN** alguien se registra con un email que ya tiene cuenta
- **THEN** el registro se rechaza con un conflicto

#### Scenario: Contraseña corta
- **WHEN** la contraseña tiene 7 caracteres
- **THEN** el registro se rechaza indicando que debe tener al menos 8

### Requirement: Inicio de sesión
El servidor SHALL iniciar sesión con email y contraseña y devolver un token de acceso, un token de renovación y los datos del fotógrafo. Si el email no existe o la contraseña no coincide, SHALL responder con el mismo error de credenciales inválidas, sin revelar cuál falló.

#### Scenario: Credenciales correctas
- **WHEN** el fotógrafo inicia sesión con su email y contraseña
- **THEN** recibe un token de acceso y uno de renovación

#### Scenario: Contraseña incorrecta
- **WHEN** la contraseña no coincide
- **THEN** la respuesta es 401 con el mismo mensaje que para un email inexistente

### Requirement: Protección de contraseñas
Las contraseñas SHALL guardarse solo como hash argon2id y nunca en claro ni en los logs.

#### Scenario: Revisión de la base de datos
- **WHEN** se inspecciona la tabla de fotógrafos
- **THEN** solo hay hashes argon2id y ninguna contraseña legible

### Requirement: Límite de intentos
El registro y el inicio de sesión SHALL limitarse por IP. El límite por defecto es de 10 intentos por minuto y se puede configurar.

#### Scenario: Fuerza bruta
- **WHEN** una IP hace 11 intentos de login en un minuto
- **THEN** el undécimo se rechaza por exceso de peticiones

### Requirement: Sesión con renovación rotativa
El token de acceso SHALL ser de corta duración (15 minutos por defecto) y el de renovación de larga duración (30 días por defecto). Cada renovación SHALL invalidar el token usado y emitir un par nuevo. Un token de renovación revocado, caducado o desconocido SHALL rechazarse con 401.

#### Scenario: Renovación correcta
- **WHEN** el cliente renueva con un token válido
- **THEN** recibe un par nuevo y el token anterior deja de servir

#### Scenario: Reutilizar un token ya rotado
- **WHEN** el cliente intenta renovar con un token que ya usó
- **THEN** se rechaza con 401

### Requirement: Cierre de sesión
Cerrar sesión SHALL revocar el token de renovación en el servidor y borrar la sesión guardada en el cliente. Si el servidor no responde, la sesión local SHALL borrarse igualmente.

#### Scenario: Logout sin red
- **WHEN** el fotógrafo cierra sesión en el desktop sin conexión
- **THEN** la sesión local se borra y la app pide iniciar sesión para respaldar

### Requirement: Aislamiento por fotógrafo
Todas las rutas del servidor, salvo registro, login, renovación, logout y salud, SHALL exigir un token de acceso válido y SHALL limitar cada lectura y escritura a los recursos del fotógrafo del token. La identidad SHALL salir del token, nunca de datos enviados por el cliente.

#### Scenario: Sin token
- **WHEN** se pide `GET /v1/auth/me` sin token
- **THEN** la respuesta es 401

#### Scenario: Recurso de otro fotógrafo
- **WHEN** el fotógrafo A pide un proyecto que pertenece al fotógrafo B
- **THEN** el servidor responde como si el proyecto no existiera

### Requirement: Sesión persistente en el desktop
El desktop SHALL dejar configurar la URL del servidor (por defecto `http://localhost:8787`), registrarse e iniciar sesión desde Ajustes → Respaldo. La sesión SHALL guardarse entre reinicios y renovarse sola cuando el token de acceso caduque. Solo SHALL pedirse un nuevo login si la renovación falla.

#### Scenario: Reiniciar la app
- **WHEN** el fotógrafo con sesión iniciada reinicia Lumik
- **THEN** sigue con la sesión iniciada sin volver a escribir la contraseña

#### Scenario: Token de acceso caducado
- **WHEN** el desktop hace una petición con el token de acceso caducado
- **THEN** renueva el token y repite la petición sin intervención del usuario

### Requirement: Sesión en el portal web
El portal web SHALL permitir registrarse e iniciar sesión con la misma cuenta, guardar la sesión en el navegador y renovarla sola al recibir un 401.

#### Scenario: Misma cuenta en web y desktop
- **WHEN** el fotógrafo registrado desde el desktop inicia sesión en el portal web
- **THEN** ve los mismos proyectos que ha publicado desde el desktop

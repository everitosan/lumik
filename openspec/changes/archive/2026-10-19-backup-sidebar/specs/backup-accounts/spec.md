# Spec Delta

## MODIFIED Requirements

### Requirement: Sesión persistente en el desktop
El desktop SHALL dejar configurar la URL del servidor (por defecto `http://localhost:8787`), registrarse, iniciar sesión y cerrarla desde Ajustes → Respaldo. Con sesión iniciada, Ajustes → Respaldo SHALL mostrar el email de la cuenta y la acción para cerrar sesión, sin formulario de login. La sesión SHALL guardarse entre reinicios y renovarse sola cuando el token de acceso caduque. Solo SHALL pedirse un nuevo login si la renovación falla.

#### Scenario: Reiniciar la app
- **WHEN** el fotógrafo con sesión iniciada reinicia Lumik
- **THEN** sigue con la sesión iniciada sin volver a escribir la contraseña

#### Scenario: Token de acceso caducado
- **WHEN** el desktop hace una petición con el token de acceso caducado
- **THEN** renueva el token y repite la petición sin intervención del usuario

#### Scenario: Cambiar de servidor
- **WHEN** el fotógrafo sin sesión cambia la URL del servidor en Ajustes → Respaldo y luego inicia sesión
- **THEN** la sesión se inicia contra el servidor de la nueva URL

#### Scenario: Cerrar sesión desde Ajustes
- **WHEN** el fotógrafo con sesión iniciada pulsa "Cerrar sesión" en Ajustes → Respaldo
- **THEN** Ajustes → Respaldo vuelve a mostrar el formulario de login y respaldar un proyecto pide iniciar sesión

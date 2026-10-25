# Spec Delta

## ADDED Requirements

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

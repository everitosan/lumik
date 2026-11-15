# Spec Delta

## Purpose

Permite marcar varias fotos del grid de un proyecto, en el desktop y en la tablet, para actuar sobre todas a la vez. "Marcar" no es aprobar: no mueve archivos ni cambia nada de la foto.

## ADDED Requirements

### Requirement: Marcar fotos en el desktop
En el grid del proyecto en el desktop, Ctrl+clic (Cmd+clic en macOS) SHALL marcar o desmarcar una foto, y Shift+clic SHALL marcar todas las fotos visibles entre la última marcada y la que se pulsa, en el orden del grid. Cada tarjeta SHALL mostrar una casilla al pasar el ratón, y pulsarla SHALL marcar o desmarcar la foto. Marcar no SHALL cambiar la foto ni sus archivos.

#### Scenario: Marcar dos fotos
- **WHEN** el usuario hace Ctrl+clic en `IMG_0001` y en `IMG_0005`
- **THEN** las dos quedan marcadas y la barra dice "2 fotos marcadas"

#### Scenario: Marcar un rango
- **WHEN** con `IMG_0001` marcada, el usuario hace Shift+clic en `IMG_0005`
- **THEN** quedan marcadas las cinco fotos visibles de `IMG_0001` a `IMG_0005`

#### Scenario: Desmarcar
- **WHEN** el usuario hace Ctrl+clic en una foto marcada
- **THEN** la foto deja de estar marcada

#### Scenario: Casilla de la tarjeta
- **WHEN** el usuario pasa el ratón por una tarjeta y pulsa su casilla
- **THEN** la foto queda marcada sin abrir el visor

### Requirement: Marcar fotos en la tablet
En el grid del proyecto en la tablet, un toque largo en una foto SHALL entrar al modo marcar con esa foto marcada. En ese modo, cada toque SHALL marcar o desmarcar una foto, y "Listo" SHALL salir del modo y quitar las marcas. El menú de la tarjeta SHALL seguir disponible en su botón ⋮.

#### Scenario: Entrar al modo marcar
- **WHEN** el usuario mantiene el dedo sobre `IMG_0001`
- **THEN** la tablet entra al modo marcar con `IMG_0001` marcada

#### Scenario: Salir del modo
- **WHEN** en el modo marcar el usuario toca "Listo"
- **THEN** no queda ninguna foto marcada y un toque vuelve a abrir el visor

### Requirement: Barra de acciones
Mientras haya al menos una foto marcada SHALL mostrarse una barra con el número de fotos marcadas y las acciones "Agregar a álbum", "Mover a la papelera" y quitar las marcas. La tablet, que no tiene álbumes, no SHALL ofrecer "Agregar a álbum". En un proyecto abierto en solo lectura, "Mover a la papelera" no SHALL ofrecerse.

#### Scenario: Barra visible
- **WHEN** hay 3 fotos marcadas
- **THEN** la barra dice "3 fotos marcadas" y ofrece "Agregar a álbum" y "Mover a la papelera"

#### Scenario: Barra en la tablet
- **WHEN** en la tablet hay 3 fotos marcadas
- **THEN** la barra dice "3 fotos marcadas" y ofrece "Mover a la papelera", sin "Agregar a álbum"

#### Scenario: Proyecto en solo lectura
- **WHEN** el proyecto lo tiene otro dispositivo y el usuario marca fotos
- **THEN** la barra ofrece "Agregar a álbum" pero no "Mover a la papelera"

### Requirement: Abrir el visor con fotos marcadas
Mientras haya fotos marcadas, un clic normal (desktop) o un toque (tablet) en una foto SHALL marcarla o desmarcarla en lugar de abrir el visor. Sin fotos marcadas, abrir el visor SHALL funcionar como hoy.

#### Scenario: Clic con marcas
- **WHEN** hay 2 fotos marcadas y el usuario hace clic normal en otra foto
- **THEN** esa foto queda marcada y el visor no se abre

### Requirement: Cuándo se quitan las marcas
Las marcas SHALL quitarse al cambiar cualquier filtro o el orden, al cambiar de proyecto o de sección, al terminar una acción de la barra, y con Esc en el desktop. Con fotos marcadas, Esc SHALL quitar las marcas sin regresar; sin marcas, Esc SHALL regresar como hoy.

#### Scenario: Esc en dos pasos
- **WHEN** hay fotos marcadas y el usuario pulsa Esc dos veces
- **THEN** el primer Esc quita las marcas y el segundo regresa al dashboard

#### Scenario: Cambiar un filtro
- **WHEN** hay fotos marcadas y el usuario activa el filtro de 5 estrellas
- **THEN** no queda ninguna foto marcada

# Spec Delta

## MODIFIED Requirements

### Requirement: Atajos de teclado configurables
La app SHALL tener atajos para el visor (cerrar, anterior, siguiente, zoom, ajustar, rotar, aprobar, estrellas 0–5, añadir tag) y para las vistas de proyectos (nuevo proyecto, buscar, solo aprobadas, volver, importar). El usuario SHALL poder reasignar cada uno pulsando la nueva tecla, opcionalmente con el modificador de la plataforma: `Cmd` en macOS y `Ctrl` en las demás. Un atajo con modificador SHALL valer en todas las plataformas y mostrarse en Ajustes con el símbolo de la plataforma (`⌘` en macOS, `Ctrl+` en las demás). Las teclas modificadoras sueltas y una tecla vacía SHALL rechazarse.

#### Scenario: Reasignar aprobar
- **WHEN** el usuario reasigna "aprobar" a la tecla `p`
- **THEN** en el visor `p` alterna la aprobación y la barra espaciadora deja de hacerlo

#### Scenario: Tecla modificadora sola
- **WHEN** al reasignar el usuario solo pulsa `Shift`
- **THEN** no se guarda ningún cambio

#### Scenario: Atajo con modificador inesperado
- **WHEN** un atajo es la tecla `f` y el usuario pulsa `Alt+f`
- **THEN** el atajo no se dispara

#### Scenario: Modificador en macOS
- **WHEN** en macOS el atajo "solo aprobadas" tiene su valor por defecto y el usuario pulsa `Cmd+c` en la vista de proyecto
- **THEN** la vista alterna a solo aprobadas, `Ctrl+c` no la alterna, y Ajustes muestra el atajo como `⌘C`

#### Scenario: Reasignar con modificador
- **WHEN** en Linux el usuario reasigna "importar" pulsando `Ctrl+i`
- **THEN** se guarda con el modificador, `i` sola deja de importar y `Ctrl+i` importa

# Spec Delta

## ADDED Requirements

### Requirement: Menú de acciones de la foto
Cada miniatura del grid SHALL tener un botón de menú que despliega las acciones disponibles para esa foto en la plataforma y vista actuales. Abrir el menú o elegir una acción no SHALL abrir la foto en el visor. El menú SHALL cerrarse al hacer clic fuera, con `Escape`, al desplazar el grid o al elegir una acción.

#### Scenario: Abrir el menú
- **WHEN** el usuario pulsa el botón de menú de una miniatura
- **THEN** se despliegan las acciones de esa foto y el visor no se abre

#### Scenario: Cerrar con Escape
- **WHEN** el menú está abierto y el usuario pulsa `Escape`
- **THEN** el menú se cierra sin ejecutar ninguna acción

### Requirement: Mostrar en carpeta
En plataformas de escritorio, el menú de la foto SHALL ofrecer "Mostrar en carpeta", que abre el explorador de archivos del sistema operativo en la carpeta donde está el archivo original de la foto, con ese archivo seleccionado. En Android la opción no SHALL mostrarse. Si el archivo no existe en disco SHALL informarse el error.

#### Scenario: Foto aprobada en Linux
- **WHEN** el usuario elige "Mostrar en carpeta" en una foto aprobada
- **THEN** el explorador se abre en `_culled/` con el archivo de la foto seleccionado

#### Scenario: Android
- **WHEN** el usuario abre el menú de una foto en Android
- **THEN** la opción "Mostrar en carpeta" no aparece

#### Scenario: Archivo ausente
- **WHEN** el usuario elige "Mostrar en carpeta" y el archivo ya no está en disco
- **THEN** se muestra un error y no se abre el explorador

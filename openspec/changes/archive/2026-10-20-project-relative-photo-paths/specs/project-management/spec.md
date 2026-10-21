# Spec Delta

## MODIFIED Requirements

### Requirement: Identidad portable del proyecto
Un proyecto SHALL identificarse por su `project.id`, sin importar en qué disco ni en qué plataforma esté. Ni el proyecto ni sus fotos SHALL depender del UUID del disco donde viven: las fotos nuevas no SHALL guardar ese UUID, y los valores ya guardados no SHALL usarse. Copiar o mover la carpeta de un proyecto a cualquier ruta bajo `lumik/`, en el mismo disco o en otro, SHALL bastar para que se liste y funcione completo desde ahí.

#### Scenario: Mover un proyecto a otro disco
- **WHEN** el usuario copia `lumik/2025/03/29_Chepe/` del disco A al disco B, conecta solo el disco B y abre Lumik
- **THEN** el proyecto aparece en el dashboard con el mismo id, sus fotos y su estado de respaldo, desde el disco B

#### Scenario: Mover un proyecto a otra ruta dentro de `lumik/`
- **WHEN** el usuario copia `lumik/2026/10/10_sesion/` a `lumik/10_sesion/` de otro disco y lo abre en Lumik
- **THEN** el proyecto muestra sus miniaturas y sus previews, y sus fotos se pueden rotar, aprobar, descartar, mandar a la papelera y mostrar en la carpeta

#### Scenario: Proyecto abierto en otra plataforma
- **WHEN** un proyecto importado en Linux se abre en macOS, donde el disco tiene otro UUID
- **THEN** el proyecto y sus fotos se muestran y se editan igual que en Linux

#### Scenario: Importar fotos
- **WHEN** el usuario importa fotos en un proyecto
- **THEN** las fotos nuevas se guardan sin el UUID del disco

### Requirement: Editar proyecto
El usuario SHALL poder editar el nombre, la fecha de sesión y la descripción de un proyecto. El nombre y la fecha SHALL ser obligatorios. Al guardar se SHALL actualizar el proyecto y, si cambian el nombre o la fecha, mover su carpeta a `{montaje}/lumik/{año}/{mes}/{día}_{slug}`; sus fotos SHALL seguir abriéndose desde la nueva carpeta. Si ya existe una carpeta con la ruta destino, la edición SHALL fallar sin cambios.

#### Scenario: Renombrado correcto
- **WHEN** el proyecto en `lumik/2024/06/15_Boda` se renombra a "Boda Ana"
- **THEN** la carpeta pasa a `lumik/2024/06/15_Boda Ana` y las fotos siguen abriéndose

#### Scenario: Cambiar la fecha
- **WHEN** el proyecto en `lumik/2024/06/15_Boda` cambia su fecha de sesión a `2024-09-02`
- **THEN** la carpeta pasa a `lumik/2024/09/02_Boda` y los nombres de las fotos no cambian

#### Scenario: Editar la descripción
- **WHEN** el usuario cambia solo la descripción
- **THEN** se guarda la descripción nueva y la carpeta no se mueve

#### Scenario: Nombre vacío
- **WHEN** el nuevo nombre está vacío o solo tiene espacios
- **THEN** la edición se rechaza

#### Scenario: Fecha vacía
- **WHEN** el usuario borra la fecha de sesión
- **THEN** la edición se rechaza

#### Scenario: Colisión de carpeta
- **WHEN** ya existe una carpeta con la ruta destino
- **THEN** la edición falla y el proyecto permanece en su ubicación original y con sus datos anteriores

## ADDED Requirements

### Requirement: Rutas de fotos relativas al proyecto
El sistema SHALL guardar la ubicación de los archivos de cada foto, el original y el exportado, como ruta relativa a la carpeta del proyecto (la que contiene su `project.db`), y SHALL resolverla con la carpeta donde se descubrió el proyecto. Las rutas guardadas no SHALL incluir el punto de montaje ni la ubicación del proyecto dentro de `lumik/`.

#### Scenario: Foto importada
- **WHEN** se importa `IMG_0001.dng` en el proyecto de `lumik/2026/10/10_sesion/`
- **THEN** la ruta guardada de la foto es `_media/IMG_0001.dng`

#### Scenario: Foto descartada o en la papelera
- **WHEN** el usuario descarta una foto y después manda otra a la papelera
- **THEN** sus rutas guardadas son `_culled/{archivo}` y `_trash/{archivo}`

#### Scenario: Exportado enlazado
- **WHEN** se enlaza `_exported/IMG_0001.jpg` con su foto
- **THEN** la ruta guardada del exportado es `_exported/IMG_0001.jpg`, y es la que se publica en el catálogo

#### Scenario: Mover la carpeta del proyecto
- **WHEN** Lumik mueve la carpeta del proyecto al cambiar su nombre o su fecha
- **THEN** las rutas guardadas de las fotos no cambian y las fotos siguen abriéndose

### Requirement: Conversión de rutas de proyectos existentes
Al abrir un proyecto cuyas fotos guardan rutas en el formato anterior, relativas al punto de montaje, el sistema SHALL convertirlas a rutas relativas al proyecto quitando lo que precede a la primera subcarpeta del proyecto (`_media`, `_culled`, `_trash` o `_exported`). La conversión SHALL ser idempotente y SHALL dejar sin cambios las rutas que no contengan ninguna de esas subcarpetas.

#### Scenario: Proyecto existente
- **WHEN** se abre un proyecto cuyas fotos tienen `lumik/2024/06/15_boda/_media/IMG_0001.cr2`
- **THEN** la ruta pasa a `_media/IMG_0001.cr2` y la foto se abre igual que antes

#### Scenario: Proyecto movido antes de la conversión
- **WHEN** un proyecto importado en `lumik/2026/10/10_sesion/` se copió a `lumik/10_sesion/` con una versión anterior de Lumik y se abre ahora
- **THEN** sus rutas se convierten a `_media/…` y sus previews y su edición vuelven a funcionar

#### Scenario: Abrir dos veces
- **WHEN** se abre un proyecto ya convertido
- **THEN** ninguna ruta cambia

#### Scenario: Ruta no reconocida
- **WHEN** una foto tiene una ruta que no contiene `_media`, `_culled`, `_trash` ni `_exported`
- **THEN** su ruta se conserva sin cambios y el resto de las fotos se convierten

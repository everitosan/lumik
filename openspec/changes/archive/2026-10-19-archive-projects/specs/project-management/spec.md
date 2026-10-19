# Spec Delta

## ADDED Requirements

### Requirement: Archivar proyectos
El usuario SHALL poder archivar y desarchivar un proyecto desde su tarjeta, sin confirmación. El dashboard SHALL ocultar los archivados salvo que el usuario elija mostrarlos con un control que indica cuántos hay y que solo aparece si hay alguno. Esa elección SHALL guardarse en la configuración de la instalación. Archivar no SHALL cambiar nada más del proyecto.

#### Scenario: Archivar un proyecto
- **WHEN** el usuario elige "Archivar" en la tarjeta de un proyecto con los archivados ocultos
- **THEN** el proyecto desaparece del dashboard y el control muestra un archivado más

#### Scenario: Mostrar archivados
- **WHEN** hay proyectos archivados y el usuario activa el control de archivados
- **THEN** los archivados aparecen en su año y en su orden, atenuados y con la etiqueta "Archivado"

#### Scenario: Desarchivar
- **WHEN** el usuario elige "Desarchivar" en la tarjeta de un proyecto archivado
- **THEN** el proyecto vuelve a mostrarse como los demás aunque los archivados estén ocultos

#### Scenario: Preferencia persistente
- **WHEN** el usuario activa el control de archivados y reinicia la app
- **THEN** el dashboard sigue mostrando los archivados

#### Scenario: Sin archivados
- **WHEN** ningún proyecto está archivado
- **THEN** el control de archivados no se muestra

#### Scenario: Estadísticas y búsqueda
- **WHEN** los archivados están ocultos
- **THEN** el contador de proyectos y fotos de la cabecera y el buscador no los incluyen

#### Scenario: Proyecto archivado en uso
- **WHEN** el usuario abre un proyecto archivado e importa fotos en él
- **THEN** la importación funciona igual que en un proyecto activo y el proyecto sigue archivado

## MODIFIED Requirements

### Requirement: Orden del dashboard
El dashboard SHALL agrupar los proyectos en secciones por el año de su fecha de sesión o, si no la tienen, de su fecha de creación. Las secciones y los proyectos de cada sección SHALL ordenarse en la dirección elegida por el usuario. Dentro de una sección, los proyectos SHALL ordenarse por esa misma fecha y desempatarse por fecha de creación en la misma dirección. Los proyectos eliminados lógicamente SHALL quedar fuera del dashboard, y los archivados SHALL mostrarse u ocultarse según la elección del usuario.

#### Scenario: Mezcla de proyectos con y sin fecha
- **WHEN** existe un proyecto sin fecha de sesión creado el 2026-05-20 y proyectos con fecha de sesión en 2026
- **THEN** el proyecto sin fecha aparece en la sección 2026, ordenado por su fecha de creación junto a los demás

#### Scenario: Misma fecha de sesión
- **WHEN** dos proyectos comparten fecha de sesión y el orden de proyectos es de más antiguo a más reciente
- **THEN** aparece primero el creado antes

#### Scenario: Archivados mostrados en orden
- **WHEN** se muestran los archivados y un proyecto archivado es del 2026-03-02, entre dos activos del 2026-01-10 y del 2026-08-10
- **THEN** dentro de la sección 2026 aparece entre ambos, en el orden elegido

# Spec Delta

## MODIFIED Requirements

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
- **THEN** el contador de proyectos y fotos del dashboard y el buscador no los incluyen

#### Scenario: Proyecto archivado en uso
- **WHEN** el usuario abre un proyecto archivado e importa fotos en él
- **THEN** la importación funciona igual que en un proyecto activo y el proyecto sigue archivado

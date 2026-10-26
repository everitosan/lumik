# Spec Delta

## MODIFIED Requirements

### Requirement: Organización de Ajustes
La página de Ajustes SHALL organizarse en las pestañas Importación, Atajos, Apariencia, Búsqueda y Respaldo. Búsqueda SHALL tener el modelo de búsqueda por contenido (descargar, estado, borrar) y el modo de análisis de los proyectos nuevos (spec `semantic-search`).

#### Scenario: Abrir Ajustes
- **WHEN** el usuario abre Ajustes
- **THEN** ve las cinco pestañas, con Importación seleccionada

#### Scenario: Pestaña Búsqueda sin modelo
- **WHEN** el usuario abre la pestaña Búsqueda sin el modelo instalado
- **THEN** ve el tamaño de la descarga y el botón "Descargar", y el modo de análisis desactivado

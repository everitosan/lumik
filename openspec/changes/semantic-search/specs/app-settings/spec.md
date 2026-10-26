# Spec Delta

## MODIFIED Requirements

### Requirement: Organización de Ajustes
La página de Ajustes SHALL organizarse en las pestañas Importación, Atajos, Apariencia, IA y Respaldo. IA SHALL agrupar las funciones que usan modelos en el equipo: hoy, el modelo de búsqueda por contenido (descargar, estado, borrar) y, solo con el modelo instalado, el modo de análisis de los proyectos nuevos (spec `semantic-search`).

#### Scenario: Abrir Ajustes
- **WHEN** el usuario abre Ajustes
- **THEN** ve las cinco pestañas, con Importación seleccionada

#### Scenario: Pestaña IA sin modelo
- **WHEN** el usuario abre la pestaña IA sin el modelo instalado
- **THEN** ve qué hace la búsqueda por contenido, el tamaño de la descarga y el botón "Descargar", y no ve el modo de análisis

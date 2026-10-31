# Spec Delta

## ADDED Requirements

### Requirement: Buscador en la tablet
El inicio de la app de tablet SHALL tener un buscador que, al escribir, filtra los proyectos cuyo nombre o descripción contiene el texto, sin distinguir mayúsculas ni acentos. En la tablet el buscador no SHALL buscar fotos.

#### Scenario: Filtrar proyectos
- **WHEN** el fotógrafo escribe `boda` en el buscador de la tablet
- **THEN** ve solo los proyectos cuyo nombre o descripción contiene "boda", incluido "Bodas de Ana"

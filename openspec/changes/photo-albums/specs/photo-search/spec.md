# Spec Delta

## MODIFIED Requirements

### Requirement: Abrir un resultado
Abrir una foto de los resultados SHALL abrirla en el visor, encima de los resultados, si su disco está conectado. Si no, SHALL indicar qué disco conectar. En el visor, las flechas SHALL recorrer los resultados en el orden en que se ven, aunque sean de proyectos distintos, saltando los que no tienen su disco conectado. Cerrar el visor SHALL volver a los resultados, con la misma consulta y en la misma posición.

#### Scenario: Disco conectado
- **WHEN** el fotógrafo abre un resultado cuyo disco está conectado
- **THEN** se abre esa foto en el visor, sin pasar por el grid de su proyecto

#### Scenario: Siguiente resultado
- **WHEN** el fotógrafo ve en el visor un resultado de "Boda" y el siguiente resultado es de "XV Ana"
- **THEN** la flecha derecha muestra esa foto de "XV Ana"

#### Scenario: Volver a los resultados
- **WHEN** el fotógrafo abrió una foto desde los resultados y cierra el visor con Esc
- **THEN** vuelve al dashboard con los resultados de su búsqueda, en la misma posición

#### Scenario: Disco no conectado
- **WHEN** el fotógrafo abre un resultado cuyo disco no está conectado
- **THEN** ve la miniatura y un aviso con el nombre del disco que debe conectar

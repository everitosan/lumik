# Spec Delta

## Purpose

Organiza la barra lateral del desktop en secciones que el fotógrafo puede mostrar u ocultar según lo que necesite, sin perder de vista lo que pasa en las ocultas.

## ADDED Requirements

### Requirement: Secciones colapsables
Las secciones "Respaldos" y "Almacenamiento" de la barra lateral SHALL poder colapsarse y expandirse desde su encabezado, de forma independiente ("Respaldos" solo aparece mientras haya respaldos que mostrar). Colapsada, una sección SHALL mostrar solo su encabezado.

#### Scenario: Colapsar Almacenamiento
- **WHEN** el fotógrafo pulsa el encabezado de "Almacenamiento" con la sección expandida
- **THEN** la lista de discos se oculta y "Respaldos" no cambia

### Requirement: Estado colapsado persistente
El estado colapsado o expandido de cada sección SHALL guardarse en la configuración de la instalación, no en los discos, y conservarse entre reinicios. En una instalación nueva, ambas secciones SHALL empezar expandidas.

#### Scenario: Reiniciar con Almacenamiento colapsado
- **WHEN** el fotógrafo colapsa "Almacenamiento" y reinicia la app
- **THEN** "Almacenamiento" sigue colapsada

#### Scenario: Instalación nueva
- **WHEN** el fotógrafo abre la app por primera vez
- **THEN** "Respaldos" y "Almacenamiento" están expandidas

### Requirement: Resumen en el encabezado colapsado
Colapsada, cada sección SHALL mostrar un resumen en su encabezado. "Respaldos" SHALL indicar cuántos respaldos están en curso y el avance total en archivos, o cuántos terminaron con errores si no hay ninguno en curso. "Almacenamiento" SHALL indicar cuántos discos hay conectados.

#### Scenario: Respaldos colapsada con subidas
- **WHEN** "Respaldos" está colapsada y se respaldan dos proyectos con 4 de 43 archivos subidos en total
- **THEN** el encabezado indica 2 respaldos en curso y 4/43

#### Scenario: Respaldos colapsada con errores
- **WHEN** "Respaldos" está colapsada, no hay subidas en curso y un respaldo terminó con errores
- **THEN** el encabezado indica que hay 1 respaldo con errores

### Requirement: Expandir al lanzar un respaldo
Al lanzar un respaldo nuevo, la sección "Respaldos" SHALL expandirse si estaba colapsada. Si después el fotógrafo la colapsa, SHALL quedarse colapsada mientras ese respaldo sigue.

#### Scenario: Lanzar con la sección colapsada
- **WHEN** "Respaldos" está colapsada y el fotógrafo lanza el respaldo de un proyecto
- **THEN** la sección se expande y muestra la tarjeta del nuevo respaldo

### Requirement: Altura limitada de Respaldos
Con muchos respaldos visibles, la lista de "Respaldos" SHALL desplazarse dentro de su sección en lugar de crecer sin límite, de modo que "Almacenamiento" siga visible en la barra lateral.

#### Scenario: Muchos respaldos a la vez
- **WHEN** hay ocho respaldos en curso con ambas secciones expandidas
- **THEN** la lista de respaldos se desplaza dentro de su sección y "Almacenamiento" sigue visible

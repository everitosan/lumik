# Spec Delta

## ADDED Requirements

### Requirement: Mover varias fotos a la papelera
"Mover a la papelera" de la barra de acciones SHALL mostrar un solo modal de confirmación con el número de fotos marcadas, que diga que se moverán a la papelera de su proyecto y que se pueden restaurar desde la sección "Papelera". Al confirmar, cada foto SHALL moverse como si se moviera sola. Si una falla, las demás SHALL moverse igual, y al terminar SHALL informarse cuáles fallaron y por qué.

#### Scenario: Confirmar un lote
- **WHEN** el usuario marca 12 fotos, elige "Mover a la papelera" y confirma
- **THEN** las 12 quedan en `_trash/` y dejan de verse en el grid

#### Scenario: Una falla
- **WHEN** de 12 fotos marcadas, una no se puede mover porque ya existe un archivo con su nombre en `_trash/`
- **THEN** las otras 11 quedan en la papelera, la que falló sigue en el grid y se informa su nombre y el motivo

#### Scenario: Cancelar
- **WHEN** el usuario cierra el modal del lote con "Cancelar"
- **THEN** ninguna foto se mueve y siguen marcadas

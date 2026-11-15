# Spec Delta

## ADDED Requirements

### Requirement: Agregar las fotos marcadas
"Agregar a álbum" de la barra de acciones del proyecto SHALL abrir el mismo diálogo de elegir o crear álbum y SHALL agregar todas las fotos marcadas de una vez. Las que ya estén en el álbum no SHALL repetirse. Al terminar SHALL informarse cuántas se agregaron.

#### Scenario: Agregar un lote
- **WHEN** el usuario marca 20 fotos y las agrega a "Convocatoria México 2026", que ya tenía 3 de ellas
- **THEN** el álbum gana 17 fotos y se informa que se agregaron 17

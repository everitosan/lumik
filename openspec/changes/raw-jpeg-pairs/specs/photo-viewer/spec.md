# Spec Delta

## ADDED Requirements

### Requirement: Indicador de formato
Cada miniatura del grid y la foto abierta en el visor SHALL mostrar en la esquina superior derecha un indicador de su formato: `RAW+JPG` para un par, `RAW` para un RAW suelto y `JPG` para un JPEG suelto. El indicador SHALL leerse sobre fotos claras y oscuras.

#### Scenario: Grid con pares y sueltos
- **WHEN** el usuario abre un proyecto con pares, RAW sueltos y JPG sueltos
- **THEN** cada miniatura muestra `RAW+JPG`, `RAW` o `JPG` según corresponda

#### Scenario: Foto clara
- **WHEN** la foto tiene un cielo blanco en la esquina superior derecha
- **THEN** el indicador se sigue leyendo

### Requirement: Ver el RAW o el JPG de un par
En el visor, un par SHALL abrirse mostrando su JPG. El indicador `RAW+JPG` SHALL permitir cambiar entre el JPG y el RAW con el ratón y con el teclado, y SHALL señalar cuál se ve. Al pasar a otra foto, el visor SHALL volver a mostrar el JPG.

#### Scenario: Revisar el RAW
- **WHEN** el usuario abre un par y pulsa el indicador
- **THEN** el visor muestra la vista previa del RAW y el indicador señala que se ve el RAW

#### Scenario: Siguiente foto
- **WHEN** el usuario está viendo el RAW de un par y pasa a la siguiente foto
- **THEN** la siguiente foto se muestra con su JPG si es un par

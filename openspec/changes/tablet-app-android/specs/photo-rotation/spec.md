# Spec Delta

## MODIFIED Requirements

### Requirement: Orientación escrita en el archivo con espera
La orientación final SHALL escribirse en el archivo unos 600 ms después del último cambio, sin bloquear la interfaz. Si en ese intervalo llegan más rotaciones de la misma foto, SHALL hacerse una sola escritura con el valor final. En Linux y Windows se SHALL escribir dentro del archivo.

#### Scenario: Rotaciones seguidas
- **WHEN** el usuario pulsa `]` tres veces seguidas
- **THEN** el archivo recibe una sola escritura con orientación de 270°

#### Scenario: Android
- **WHEN** el fotógrafo abre una foto en el visor de la app de tablet Android
- **THEN** el visor no ofrece rotarla y la orientación del archivo no cambia

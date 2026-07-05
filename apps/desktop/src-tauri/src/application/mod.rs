//! Capa de aplicación: puertos (traits frontera) y, en fases posteriores, casos
//! de uso y estado de sesión. Depende solo de `domain`; la infraestructura
//! implementa sus puertos. Ver `docs/desktop-clean-architecture.md`.

pub mod ports;
pub mod use_cases;

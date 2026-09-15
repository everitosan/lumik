//! Rutas HTTP. El health check vive aquí; auth se compone en `lib::build_router`,
//! y catálogo/backup se añaden en fases posteriores (ver docs/backup-plan.md).

pub mod health;

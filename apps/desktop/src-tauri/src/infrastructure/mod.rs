//! Capa de infraestructura: implementaciones concretas de los puertos de
//! `application::ports` (dispositivos, exiftool/rawler, imágenes, …). Es la
//! única capa que puede contener `#[cfg(target_os)]` de plataforma; la
//! selección de qué implementación usar ocurre en `lib.rs`.

pub mod app_data;
pub mod backup_auth;
pub mod backup_client;
pub mod catalog;
pub mod content_model;
pub mod devices;
#[cfg(target_os = "macos")]
pub mod disk_arbitration;
pub mod finder_tags;
pub mod folder;
pub mod fs;
pub mod imaging;
pub mod lock;
pub mod import_pipeline;
pub mod import_signing;
pub mod metadata;
pub mod progress;

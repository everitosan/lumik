//! Capa de infraestructura: implementaciones concretas de los puertos de
//! `application::ports` (dispositivos, exiftool/rawler, imágenes, …). Es la
//! única capa que puede contener `#[cfg(target_os)]` de plataforma; la
//! selección de qué implementación usar ocurre en `lib.rs`.

pub mod devices;
pub mod finder_tags;
pub mod folder;
pub mod fs;
pub mod imaging;
pub mod metadata;
pub mod persistence;
pub mod progress;

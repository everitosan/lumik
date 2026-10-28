//! Histograma RGB de la vista previa, como el de la barra lateral del desktop
//! (`histogram.worker.ts`), calculado en un hilo de Rust.

use std::sync::Arc;

/// Cuentas por nivel (0–255) de cada canal.
#[derive(uniffi::Record)]
pub struct Histogram {
    pub r: Vec<u32>,
    pub g: Vec<u32>,
    pub b: Vec<u32>,
    pub elapsed_ms: u64,
}

#[uniffi::export(with_foreign)]
pub trait HistogramListener: Send + Sync {
    fn on_histogram(&self, histogram: Histogram);
    fn on_error(&self, message: String);
}

/// Decodifica `path` y cuenta cada píxel. Vuelve de inmediato; el resultado
/// llega a `listener` desde otro hilo.
#[uniffi::export]
pub fn start_histogram(path: String, listener: Arc<dyn HistogramListener>) {
    std::thread::spawn(move || match histogram_of(&path) {
        Ok(h) => listener.on_histogram(h),
        Err(e) => listener.on_error(e),
    });
}

pub(crate) fn histogram_of(path: &str) -> Result<Histogram, String> {
    let started = std::time::Instant::now();
    let img = image::open(path).map_err(|e| e.to_string())?.into_rgb8();
    let (mut r, mut g, mut b) = (vec![0u32; 256], vec![0u32; 256], vec![0u32; 256]);
    for px in img.pixels() {
        r[px[0] as usize] += 1;
        g[px[1] as usize] += 1;
        b[px[2] as usize] += 1;
    }
    Ok(Histogram { r, g, b, elapsed_ms: started.elapsed().as_millis() as u64 })
}

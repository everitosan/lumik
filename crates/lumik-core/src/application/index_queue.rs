//! Cola de proyectos por reindexar en la búsqueda (spec `photo-search`).
//!
//! Cada pedido lleva una hora a partir de la cual se puede atender: al escanear
//! discos se pide "ya", y tras una escritura con una espera corta, que se
//! reinicia con cada escritura nueva (debounce). Un hilo trabajador de la app
//! espera con `next_due` y reindexa lo que devuelve.

use std::collections::HashMap;
use std::sync::{Condvar, Mutex};
use std::time::{Duration, Instant};

#[derive(Default)]
pub struct IndexQueue {
    pending: Mutex<HashMap<String, Instant>>,
    wake: Condvar,
}

impl IndexQueue {
    pub fn new() -> Self {
        Self::default()
    }

    /// Pide reindexar `project_id` a partir de `delay` desde ahora. Un pedido
    /// nuevo reemplaza al anterior del mismo proyecto.
    pub fn request(&self, project_id: &str, delay: Duration) {
        self.request_at(project_id, Instant::now() + delay);
    }

    fn request_at(&self, project_id: &str, due: Instant) {
        self.pending.lock().unwrap().insert(project_id.to_string(), due);
        self.wake.notify_all();
    }

    /// Saca los proyectos cuya hora ya llegó en `now`.
    pub fn take_due(&self, now: Instant) -> Vec<String> {
        let mut pending = self.pending.lock().unwrap();
        let due: Vec<String> = pending.iter().filter(|(_, at)| **at <= now).map(|(id, _)| id.clone()).collect();
        for id in &due {
            pending.remove(id);
        }
        due
    }

    /// Bloquea hasta que haya proyectos listos y los devuelve.
    pub fn next_due(&self) -> Vec<String> {
        let mut pending = self.pending.lock().unwrap();
        loop {
            let now = Instant::now();
            if pending.values().any(|at| *at <= now) {
                drop(pending);
                return self.take_due(now);
            }
            pending = match pending.values().min().copied() {
                Some(soonest) => self.wake.wait_timeout(pending, soonest - now).unwrap().0,
                None => self.wake.wait(pending).unwrap(),
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_writes_in_a_row_reindex_once() {
        let q = IndexQueue::new();
        let start = Instant::now();
        q.request_at("p", start + Duration::from_secs(2));
        // Segunda escritura un segundo después: la espera se reinicia.
        q.request_at("p", start + Duration::from_secs(3));
        assert!(q.take_due(start + Duration::from_millis(2500)).is_empty());
        assert_eq!(q.take_due(start + Duration::from_secs(3)), ["p"]);
        assert!(q.take_due(start + Duration::from_secs(10)).is_empty(), "una sola vez");
    }

    #[test]
    fn immediate_requests_and_other_projects_are_independent() {
        let q = IndexQueue::new();
        let start = Instant::now();
        q.request_at("a", start);
        q.request_at("b", start + Duration::from_secs(5));
        assert_eq!(q.take_due(start), ["a"]);
        assert_eq!(q.take_due(start + Duration::from_secs(5)), ["b"]);
    }

    #[test]
    fn next_due_wakes_when_a_request_arrives() {
        let q = std::sync::Arc::new(IndexQueue::new());
        let worker = {
            let q = q.clone();
            std::thread::spawn(move || q.next_due())
        };
        std::thread::sleep(Duration::from_millis(20));
        q.request("p", Duration::from_millis(10));
        assert_eq!(worker.join().unwrap(), ["p"]);
    }
}

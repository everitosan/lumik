//! Cola de proyectos por analizar (spec `semantic-search`): un proyecto a la
//! vez, en el orden en que se pidieron. La alimentan "Analizar fotos", el final
//! de una importación y el aviso de los resultados; un hilo trabajador de la app
//! espera con `next` y llama a `finish` al terminar cada uno.
//!
//! Cada proyecto en curso tiene su bandera de cancelación: detenerlo, o expulsar
//! su disco, la levanta y `AnalyzeProject` se detiene antes de la foto siguiente.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

#[derive(Default)]
struct Inner {
    queued: VecDeque<String>,
    current: Option<(String, Arc<AtomicBool>)>,
}

#[derive(Default)]
pub struct AnalysisQueue {
    inner: Mutex<Inner>,
    changed: Condvar,
}

/// Qué hay en la cola, para la vista.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize)]
pub struct AnalysisQueueState {
    pub current: Option<String>,
    pub queued: Vec<String>,
}

impl AnalysisQueue {
    pub fn new() -> Self {
        Self::default()
    }

    /// Pide analizar `project_id`. No hace nada si ya está en curso o en espera.
    pub fn request(&self, project_id: &str) {
        let mut inner = self.inner.lock().unwrap();
        let running = matches!(&inner.current, Some((id, _)) if id == project_id);
        if !running && !inner.queued.iter().any(|id| id == project_id) {
            inner.queued.push_back(project_id.to_string());
            self.changed.notify_all();
        }
    }

    /// Saca `project_id` de la espera y, si está en curso, lo detiene.
    pub fn cancel(&self, project_id: &str) {
        let mut inner = self.inner.lock().unwrap();
        inner.queued.retain(|id| id != project_id);
        if let Some((id, flag)) = &inner.current {
            if id == project_id {
                flag.store(true, Ordering::Relaxed);
            }
        }
    }

    /// Cancela `project_ids` y espera, hasta `timeout`, a que ninguno siga en
    /// curso (para expulsar su disco sin la base abierta). `false` si no paró a tiempo.
    pub fn cancel_and_wait(&self, project_ids: &[String], timeout: Duration) -> bool {
        for id in project_ids {
            self.cancel(id);
        }
        let deadline = Instant::now() + timeout;
        let mut inner = self.inner.lock().unwrap();
        loop {
            let busy = matches!(&inner.current, Some((id, _)) if project_ids.contains(id));
            if !busy {
                return true;
            }
            let now = Instant::now();
            if now >= deadline {
                return false;
            }
            inner = self.changed.wait_timeout(inner, deadline - now).unwrap().0;
        }
    }

    /// Bloquea hasta que haya un proyecto en espera y lo pasa a "en curso",
    /// con su bandera de cancelación.
    pub fn next(&self) -> (String, Arc<AtomicBool>) {
        let mut inner = self.inner.lock().unwrap();
        loop {
            if inner.current.is_none() {
                if let Some(id) = inner.queued.pop_front() {
                    let flag = Arc::new(AtomicBool::new(false));
                    inner.current = Some((id.clone(), flag.clone()));
                    self.changed.notify_all();
                    return (id, flag);
                }
            }
            inner = self.changed.wait(inner).unwrap();
        }
    }

    /// El proyecto en curso terminó (o se detuvo).
    pub fn finish(&self, project_id: &str) {
        let mut inner = self.inner.lock().unwrap();
        if matches!(&inner.current, Some((id, _)) if id == project_id) {
            inner.current = None;
            self.changed.notify_all();
        }
    }

    pub fn state(&self) -> AnalysisQueueState {
        let inner = self.inner.lock().unwrap();
        AnalysisQueueState {
            current: inner.current.as_ref().map(|(id, _)| id.clone()),
            queued: inner.queued.iter().cloned().collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_project_at_a_time_in_order_without_duplicates() {
        let q = AnalysisQueue::new();
        q.request("a");
        q.request("b");
        q.request("a");
        let (first, _) = q.next();
        assert_eq!(first, "a");
        q.request("a");
        assert_eq!(q.state(), AnalysisQueueState { current: Some("a".into()), queued: vec!["b".into()] }, "en curso no se repite");
        q.finish("a");
        assert_eq!(q.next().0, "b");
    }

    #[test]
    fn cancelling_stops_the_current_one_and_drops_the_waiting_one() {
        let q = AnalysisQueue::new();
        q.request("a");
        q.request("b");
        let (_, flag) = q.next();
        q.cancel("a");
        q.cancel("b");
        assert!(flag.load(Ordering::Relaxed));
        assert_eq!(q.state().queued, Vec::<String>::new());
    }

    #[test]
    fn cancel_and_wait_returns_when_the_worker_finishes() {
        let q = Arc::new(AnalysisQueue::new());
        q.request("a");
        let worker = {
            let q = q.clone();
            std::thread::spawn(move || {
                let (id, flag) = q.next();
                while !flag.load(Ordering::Relaxed) {
                    std::thread::sleep(Duration::from_millis(5));
                }
                q.finish(&id);
            })
        };
        while q.state().current.is_none() {
            std::thread::sleep(Duration::from_millis(1));
        }
        assert!(q.cancel_and_wait(&["a".to_string()], Duration::from_secs(5)));
        worker.join().unwrap();
        assert!(q.cancel_and_wait(&["otro".to_string()], Duration::from_millis(1)), "sin nada en curso no espera");
    }
}

//! Sesiones de trabajo por proyecto y su candado (spec `project-lock`).
//!
//! Una sesión es "estoy trabajando en este proyecto": la vista del proyecto
//! abierta, o una operación que escribe en él (importar, aprobar, respaldar…).
//! Se cuentan por proyecto: la primera escribe `.lumik-lock` y la última lo
//! quita. Antes de cada escritura se relee el candado, así que si otra
//! instalación tomó el control, la escritura falla en lugar de pisar el disco.

use crate::application::ports::ProjectLockStore;
use crate::db::models::InstallIdentity;
use crate::domain::lock::{LockState, ProjectLock};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// Por qué no se pudo abrir una sesión.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionError {
    /// El proyecto lo tiene otra instalación.
    Locked(ProjectLock),
    /// No se pudo escribir el candado (disco lleno, sin permiso…).
    Io(String),
}

/// Cómo ve el candado de un proyecto la vista que lo abre.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViewLock {
    /// Se abrió con normalidad (libre o propio).
    Open,
    /// Lo tiene otra instalación: la vista queda en solo lectura.
    ReadOnly(ProjectLock),
}

struct Entry {
    dir: PathBuf,
    count: u32,
}

pub struct ProjectSessions {
    store: Arc<dyn ProjectLockStore>,
    install: Mutex<InstallIdentity>,
    sessions: Mutex<HashMap<String, Entry>>,
    /// Proyectos con la vista abierta (para cerrar solo lo que se abrió).
    views: Mutex<HashSet<String>>,
}

impl ProjectSessions {
    pub fn new(store: Arc<dyn ProjectLockStore>, install: InstallIdentity) -> Self {
        Self {
            store,
            install: Mutex::new(install),
            sessions: Mutex::new(HashMap::new()),
            views: Mutex::new(HashSet::new()),
        }
    }

    pub fn install(&self) -> InstallIdentity {
        self.install.lock().unwrap().clone()
    }

    pub fn set_install_name(&self, name: &str) {
        self.install.lock().unwrap().name = name.to_string();
    }

    /// Estado del candado de un proyecto visto desde esta instalación.
    pub fn state(&self, dir: &Path) -> LockState {
        LockState::of(self.store.read(dir), &self.install().id)
    }

    fn own_lock(&self, generation: i64) -> ProjectLock {
        let install = self.install();
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
        ProjectLock::new(&install.id, &install.name, &now, generation)
    }

    /// Abre una sesión. Falla si otra instalación tiene el proyecto.
    pub fn begin(&self, project_id: &str, dir: &Path, generation: i64) -> Result<(), SessionError> {
        let mut sessions = self.sessions.lock().unwrap();
        let state = self.state(dir);
        if let LockState::Foreign(lock) = state {
            return Err(SessionError::Locked(lock));
        }
        let entry = sessions
            .entry(project_id.to_string())
            .or_insert_with(|| Entry { dir: dir.to_path_buf(), count: 0 });
        // Primera sesión, o el candado desapareció (alguien lo borró a mano):
        // se escribe uno nuevo. Un candado propio de un cierre inesperado se
        // retoma igual, con hora nueva.
        if entry.count == 0 || state == LockState::Free {
            if let Err(e) = self.store.write(dir, &self.own_lock(generation)) {
                if entry.count == 0 {
                    sessions.remove(project_id);
                }
                return Err(SessionError::Io(e));
            }
        }
        entry.count += 1;
        Ok(())
    }

    /// Cierra una sesión; la última quita el candado si sigue siendo propio.
    pub fn end(&self, project_id: &str) {
        let mut sessions = self.sessions.lock().unwrap();
        let Some(entry) = sessions.get_mut(project_id) else {
            return;
        };
        entry.count = entry.count.saturating_sub(1);
        if entry.count == 0 {
            let dir = entry.dir.clone();
            sessions.remove(project_id);
            if self.state(&dir) == LockState::Own {
                let _ = self.store.remove(&dir);
            }
        }
    }

    /// Abre la vista del proyecto: sesión normal, o solo lectura si es ajeno.
    pub fn open_view(&self, project_id: &str, dir: &Path, generation: i64) -> Result<ViewLock, SessionError> {
        if self.views.lock().unwrap().contains(project_id) {
            // Ya abierta (p. ej. React montó la vista dos veces): no se cuenta de nuevo.
            return Ok(match self.state(dir) {
                LockState::Foreign(lock) => ViewLock::ReadOnly(lock),
                _ => ViewLock::Open,
            });
        }
        match self.begin(project_id, dir, generation) {
            Ok(()) => {
                self.views.lock().unwrap().insert(project_id.to_string());
                Ok(ViewLock::Open)
            }
            Err(SessionError::Locked(lock)) => Ok(ViewLock::ReadOnly(lock)),
            Err(e) => Err(e),
        }
    }

    /// Cierra la vista del proyecto (no hace nada si estaba en solo lectura).
    pub fn close_view(&self, project_id: &str) {
        if self.views.lock().unwrap().remove(project_id) {
            self.end(project_id);
        }
    }

    /// Toma el control: reemplaza el candado por uno propio y abre la vista.
    pub fn take_control(&self, project_id: &str, dir: &Path, generation: i64) -> Result<(), SessionError> {
        self.store
            .write(dir, &self.own_lock(generation))
            .map_err(SessionError::Io)?;
        if !self.views.lock().unwrap().contains(project_id) {
            self.begin(project_id, dir, generation)?;
            self.views.lock().unwrap().insert(project_id.to_string());
        }
        Ok(())
    }

    /// La carpeta del proyecto se movió (renombrar al editar): el candado viaja
    /// dentro de ella, así que solo cambia dónde buscarlo.
    pub fn relocate(&self, project_id: &str, new_dir: &Path) {
        if let Some(entry) = self.sessions.lock().unwrap().get_mut(project_id) {
            entry.dir = new_dir.to_path_buf();
        }
    }

    /// Suelta todas las sesiones (al cerrar la app).
    pub fn release_all(&self) {
        let ids: Vec<String> = self.sessions.lock().unwrap().keys().cloned().collect();
        self.release(&ids);
    }

    /// Suelta de golpe las sesiones de estos proyectos (al expulsar su disco).
    pub fn release(&self, project_ids: &[String]) {
        let mut sessions = self.sessions.lock().unwrap();
        let mut views = self.views.lock().unwrap();
        for id in project_ids {
            views.remove(id);
            if let Some(entry) = sessions.remove(id) {
                if self.state(&entry.dir) == LockState::Own {
                    let _ = self.store.remove(&entry.dir);
                }
            }
        }
    }
}

/// Sesión abierta mientras vive; la cierra al soltarse.
pub struct SessionGuard {
    sessions: Arc<ProjectSessions>,
    project_id: String,
}

impl SessionGuard {
    pub fn begin(
        sessions: &Arc<ProjectSessions>,
        project_id: &str,
        dir: &Path,
        generation: i64,
    ) -> Result<Self, SessionError> {
        sessions.begin(project_id, dir, generation)?;
        Ok(Self { sessions: sessions.clone(), project_id: project_id.to_string() })
    }
}

impl Drop for SessionGuard {
    fn drop(&mut self) {
        self.sessions.end(&self.project_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Candados en memoria, por carpeta.
    #[derive(Default)]
    struct MemLocks(Mutex<HashMap<PathBuf, ProjectLock>>);

    impl ProjectLockStore for MemLocks {
        fn read(&self, dir: &Path) -> Option<ProjectLock> {
            self.0.lock().unwrap().get(dir).cloned()
        }
        fn write(&self, dir: &Path, lock: &ProjectLock) -> Result<(), String> {
            self.0.lock().unwrap().insert(dir.to_path_buf(), lock.clone());
            Ok(())
        }
        fn remove(&self, dir: &Path) -> Result<(), String> {
            self.0.lock().unwrap().remove(dir);
            Ok(())
        }
    }

    fn install(id: &str, name: &str) -> InstallIdentity {
        InstallIdentity { id: id.into(), name: name.into() }
    }

    /// Dos instalaciones que comparten el mismo disco.
    fn two_devices() -> (Arc<MemLocks>, Arc<ProjectSessions>, Arc<ProjectSessions>) {
        let disk = Arc::new(MemLocks::default());
        let desktop = Arc::new(ProjectSessions::new(disk.clone(), install("pc", "estudio-pc")));
        let tablet = Arc::new(ProjectSessions::new(disk.clone(), install("tab", "Tablet de Ana")));
        (disk, desktop, tablet)
    }

    fn dir() -> PathBuf {
        PathBuf::from("/disco/lumik/2026/10/24_Boda")
    }

    #[test]
    fn view_takes_and_releases_the_lock() {
        let (disk, desktop, _) = two_devices();
        assert_eq!(desktop.open_view("p", &dir(), 7).unwrap(), ViewLock::Open);
        let lock = disk.read(&dir()).unwrap();
        assert_eq!((lock.install_id.as_str(), lock.generation, lock.pending), ("pc", 7, false));
        desktop.close_view("p");
        assert_eq!(disk.read(&dir()), None);
    }

    #[test]
    fn running_operation_keeps_the_lock_after_closing_the_view() {
        let (disk, desktop, _) = two_devices();
        desktop.open_view("p", &dir(), 0).unwrap();
        let import = SessionGuard::begin(&desktop, "p", &dir(), 0).unwrap();
        desktop.close_view("p");
        assert!(disk.read(&dir()).is_some(), "la importación sigue");
        drop(import);
        assert_eq!(disk.read(&dir()), None);
    }

    #[test]
    fn foreign_lock_gives_read_only_and_blocks_writes() {
        let (_, desktop, tablet) = two_devices();
        tablet.open_view("p", &dir(), 0).unwrap();

        match desktop.open_view("p", &dir(), 0).unwrap() {
            ViewLock::ReadOnly(lock) => assert_eq!(lock.install_name, "Tablet de Ana"),
            other => panic!("esperaba solo lectura, fue {other:?}"),
        }
        assert!(matches!(
            SessionGuard::begin(&desktop, "p", &dir(), 0),
            Err(SessionError::Locked(_))
        ));
        // Cerrar una vista en solo lectura no toca el candado ajeno.
        desktop.close_view("p");
        assert_eq!(desktop.state(&dir()), LockState::Foreign(tablet_lock(&desktop)));
    }

    fn tablet_lock(sessions: &ProjectSessions) -> ProjectLock {
        match sessions.state(&dir()) {
            LockState::Foreign(l) => l,
            other => panic!("esperaba candado ajeno, fue {other:?}"),
        }
    }

    #[test]
    fn take_control_and_the_other_device_cannot_write() {
        let (disk, desktop, tablet) = two_devices();
        tablet.open_view("p", &dir(), 0).unwrap();
        desktop.open_view("p", &dir(), 0).unwrap(); // solo lectura

        desktop.take_control("p", &dir(), 3).unwrap();
        assert_eq!(disk.read(&dir()).unwrap().install_id, "pc");
        // El desktop ya escribe con normalidad.
        let write = SessionGuard::begin(&desktop, "p", &dir(), 3);
        assert!(write.is_ok());
        drop(write);

        // La tablet, que creía tener la sesión, ya no puede escribir…
        assert!(matches!(
            SessionGuard::begin(&tablet, "p", &dir(), 0),
            Err(SessionError::Locked(_))
        ));
        // …y al cerrar su vista no quita el candado del desktop.
        tablet.close_view("p");
        assert_eq!(disk.read(&dir()).unwrap().install_id, "pc");
    }

    #[test]
    fn own_lock_after_a_crash_is_taken_back_silently() {
        let (disk, desktop, _) = two_devices();
        // Candado propio que quedó de un cierre inesperado.
        disk.write(&dir(), &ProjectLock::new("pc", "estudio-pc", "2026-10-01T00:00:00Z", 1))
            .unwrap();
        assert_eq!(desktop.open_view("p", &dir(), 5).unwrap(), ViewLock::Open);
        assert_eq!(disk.read(&dir()).unwrap().generation, 5, "se reescribe al retomarlo");
    }

    #[test]
    fn release_drops_own_locks_only() {
        let (disk, desktop, _) = two_devices();
        let other = PathBuf::from("/disco/lumik/2026/10/25_XV");
        desktop.open_view("p", &dir(), 0).unwrap();
        disk.write(&other, &ProjectLock::new("tab", "Tablet de Ana", "x", 0)).unwrap();

        desktop.release(&["p".into(), "q".into()]);
        assert_eq!(disk.read(&dir()), None);
        assert!(disk.read(&other).is_some(), "el ajeno no se toca");
        // La vista quedó cerrada: cerrarla otra vez no hace nada.
        desktop.close_view("p");
    }

    #[test]
    fn release_all_on_exit() {
        let (disk, desktop, _) = two_devices();
        desktop.open_view("p", &dir(), 0).unwrap();
        let _import = SessionGuard::begin(&desktop, "q", &PathBuf::from("/disco/q"), 0).unwrap();
        desktop.release_all();
        assert_eq!(disk.read(&dir()), None);
        assert_eq!(disk.read(&PathBuf::from("/disco/q")), None);
    }
}

use std::ffi::{OsStr, OsString};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::{Mutex, OnceLock};

use crate::util::silent_command;

/// Cómo lanzar exiftool: el programa y los argumentos que van antes de los de
/// cada llamada (en macOS, el script del bundle para `/usr/bin/perl`).
#[derive(Debug, Clone, PartialEq, Eq)]
struct Launcher {
    program: PathBuf,
    prefix: Vec<OsString>,
}

impl Launcher {
    fn plain(program: impl Into<PathBuf>) -> Self {
        Self { program: program.into(), prefix: Vec::new() }
    }
}

static LAUNCHER: OnceLock<Launcher> = OnceLock::new();

/// `Command` listo para exiftool; quien llama agrega sus argumentos.
///
/// En Windows el `exiftool.exe` del bundle está junto al ejecutable y el
/// sistema lo encuentra por nombre; en Linux se usa el del PATH. En macOS una
/// app abierta desde Finder recibe un PATH mínimo, así que se usa el script
/// empaquetado en `Contents/Resources/exiftool/` (ver `pick_macos_launcher`).
pub fn command() -> Command {
    let launcher = LAUNCHER.get_or_init(|| {
        let launcher = resolve_launcher();
        log::info!("exiftool: {} {:?}", launcher.program.display(), launcher.prefix);
        launcher
    });
    let mut cmd = silent_command(&launcher.program);
    cmd.args(&launcher.prefix);
    cmd
}

#[cfg(target_os = "macos")]
fn resolve_launcher() -> Launcher {
    let exe = std::env::current_exe().ok();
    let exe_dir = exe.as_deref().and_then(Path::parent);
    let path_var = std::env::var_os("PATH");
    pick_macos_launcher(exe_dir, path_var.as_deref(), |p| p.is_file())
}

#[cfg(not(target_os = "macos"))]
fn resolve_launcher() -> Launcher {
    Launcher::plain("exiftool")
}

const MACOS_PERL: &str = "/usr/bin/perl";
const MACOS_FALLBACK_DIRS: [&str; 2] = ["/opt/homebrew/bin", "/usr/local/bin"];

/// Elige cómo lanzar exiftool en macOS. `exe_dir` es la carpeta del ejecutable
/// (`Lumik.app/Contents/MacOS` dentro del bundle) y `exists` dice si un archivo
/// existe. Orden: script del bundle con el Perl del sistema, `exiftool` en el
/// PATH, en Homebrew (`/opt/homebrew/bin`, `/usr/local/bin`), y por último el
/// nombre a secas, que falla al lanzarse con un error claro.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn pick_macos_launcher(
    exe_dir: Option<&Path>,
    path_var: Option<&OsStr>,
    exists: impl Fn(&Path) -> bool,
) -> Launcher {
    let perl = Path::new(MACOS_PERL);
    if let Some(contents) = exe_dir.and_then(Path::parent) {
        let script = contents.join("Resources").join("exiftool").join("exiftool");
        if exists(&script) && exists(perl) {
            return Launcher { program: perl.to_path_buf(), prefix: vec![script.into_os_string()] };
        }
    }
    let path_dirs = path_var.map(|p| std::env::split_paths(p).collect::<Vec<_>>()).unwrap_or_default();
    path_dirs
        .into_iter()
        .chain(MACOS_FALLBACK_DIRS.iter().map(PathBuf::from))
        .map(|dir| dir.join("exiftool"))
        .find(|candidate| exists(candidate))
        .map(Launcher::plain)
        .unwrap_or_else(|| Launcher::plain("exiftool"))
}

/// Persistent exiftool process driven via `-stay_open True -@ -`.
/// Reuses one Perl runtime across calls, avoiding ~800ms cold-start on Windows
/// and ~80ms on Linux/macOS per invocation.
///
/// Text-only. Binary extraction (`-b`) still uses one-shot `Command::new`
/// because the binary stream would need careful framing against the sentinel.
struct ExifToolSession {
    _child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl ExifToolSession {
    fn spawn() -> Result<Self, String> {
        let mut child = command()
            .args(["-stay_open", "True", "-@", "-"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("Failed to spawn exiftool: {}", e))?;
        let stdin = child.stdin.take().ok_or("no stdin")?;
        let stdout = BufReader::new(child.stdout.take().ok_or("no stdout")?);
        Ok(Self { _child: child, stdin, stdout })
    }

    fn run(&mut self, args: &[String]) -> Result<String, String> {
        for arg in args {
            writeln!(self.stdin, "{}", arg).map_err(|e| format!("write: {}", e))?;
        }
        writeln!(self.stdin, "-execute").map_err(|e| format!("write: {}", e))?;
        self.stdin.flush().map_err(|e| format!("flush: {}", e))?;

        let mut out = String::new();
        let mut line = String::new();
        loop {
            line.clear();
            let n = self.stdout.read_line(&mut line).map_err(|e| format!("read: {}", e))?;
            if n == 0 {
                return Err("exiftool process ended".to_string());
            }
            if line.trim_end_matches(['\r', '\n']) == "{ready}" {
                break;
            }
            out.push_str(&line);
        }
        Ok(out)
    }
}

static SESSION: OnceLock<Mutex<Option<ExifToolSession>>> = OnceLock::new();

fn session() -> &'static Mutex<Option<ExifToolSession>> {
    SESSION.get_or_init(|| Mutex::new(None))
}

/// Run a sequence of exiftool args against the persistent session.
/// Spawns the session lazily and respawns it if it died.
pub fn run_text(args: &[String]) -> Result<String, String> {
    let mut guard = session().lock().unwrap();
    if guard.is_none() {
        *guard = Some(ExifToolSession::spawn()?);
    }
    let result = guard.as_mut().unwrap().run(args);
    if result.is_err() {
        *guard = None;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn existing(paths: &[&str]) -> impl Fn(&Path) -> bool {
        let set: HashSet<PathBuf> = paths.iter().map(PathBuf::from).collect();
        move |p: &Path| set.contains(p)
    }

    const EXE_DIR: &str = "/Applications/Lumik.app/Contents/MacOS";
    const SCRIPT: &str = "/Applications/Lumik.app/Contents/Resources/exiftool/exiftool";

    #[test]
    fn bundle_script_with_system_perl() {
        let l = pick_macos_launcher(
            Some(Path::new(EXE_DIR)),
            Some(OsStr::new("/usr/bin:/bin")),
            existing(&[SCRIPT, MACOS_PERL, "/opt/homebrew/bin/exiftool"]),
        );
        assert_eq!(l, Launcher { program: MACOS_PERL.into(), prefix: vec![SCRIPT.into()] });
    }

    #[test]
    fn without_perl_falls_back_to_path() {
        let l = pick_macos_launcher(
            Some(Path::new(EXE_DIR)),
            Some(OsStr::new("/usr/bin:/Users/x/.local/bin")),
            existing(&[SCRIPT, "/Users/x/.local/bin/exiftool"]),
        );
        assert_eq!(l, Launcher::plain("/Users/x/.local/bin/exiftool"));
    }

    #[test]
    fn dev_build_uses_path_before_homebrew() {
        let l = pick_macos_launcher(
            Some(Path::new("/repo/target/debug")),
            Some(OsStr::new("/usr/bin:/Users/x/.local/bin")),
            existing(&[MACOS_PERL, "/Users/x/.local/bin/exiftool", "/usr/local/bin/exiftool"]),
        );
        assert_eq!(l, Launcher::plain("/Users/x/.local/bin/exiftool"));
    }

    #[test]
    fn finder_path_falls_back_to_homebrew() {
        let l = pick_macos_launcher(
            None,
            Some(OsStr::new("/usr/bin:/bin:/usr/sbin:/sbin")),
            existing(&["/usr/local/bin/exiftool", "/opt/homebrew/bin/exiftool"]),
        );
        assert_eq!(l, Launcher::plain("/opt/homebrew/bin/exiftool"));
    }

    #[test]
    fn nothing_found_uses_bare_name() {
        let l = pick_macos_launcher(Some(Path::new(EXE_DIR)), None, existing(&[]));
        assert_eq!(l, Launcher::plain("exiftool"));
    }
}

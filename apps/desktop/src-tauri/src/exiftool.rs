use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Stdio};
use std::sync::{Mutex, OnceLock};

use crate::util::silent_command;

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
        let mut child = silent_command("exiftool")
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

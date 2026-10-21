//! FFI mínimo de DiskArbitration (macOS) para el watcher de volúmenes y la
//! expulsión. Solo se declara lo que usa Lumik; los tipos de CoreFoundation
//! (run loop, CFString, CFURL, CFArray) vienen de `core-foundation`.
//!
//! DiskArbitration es asíncrono: las operaciones reportan por callbacks que se
//! ejecutan en el run loop donde está programada la sesión. Por eso cada
//! `Session` se programa en el run loop del hilo que la usa.

use std::ffi::{c_char, c_void, CStr};
use std::path::Path;

use core_foundation::base::TCFType;
use core_foundation::string::CFString;
use core_foundation::url::CFURL;
use core_foundation_sys::array::CFArrayRef;
use core_foundation_sys::base::{CFAllocatorRef, CFRelease};
use core_foundation_sys::dictionary::CFDictionaryRef;
use core_foundation_sys::runloop::{kCFRunLoopDefaultMode, CFRunLoopGetCurrent, CFRunLoopRef};
use core_foundation_sys::string::CFStringRef;
use core_foundation_sys::url::CFURLRef;

pub type DASessionRef = *const c_void;
pub type DADiskRef = *const c_void;
pub type DADissenterRef = *const c_void;
/// `mach_error_t`.
pub type DAReturn = i32;

pub type DADiskDescriptionChangedCallback =
    extern "C" fn(disk: DADiskRef, keys: CFArrayRef, context: *mut c_void);
pub type DADiskDisappearedCallback = extern "C" fn(disk: DADiskRef, context: *mut c_void);
pub type DADiskUnmountCallback =
    extern "C" fn(disk: DADiskRef, dissenter: DADissenterRef, context: *mut c_void);
pub type DADiskEjectCallback =
    extern "C" fn(disk: DADiskRef, dissenter: DADissenterRef, context: *mut c_void);

/// Desmonta todos los volúmenes del disco completo.
pub const K_DA_DISK_UNMOUNT_OPTION_WHOLE: u32 = 0x0000_0001;
pub const K_DA_DISK_EJECT_OPTION_DEFAULT: u32 = 0x0000_0000;

// err_local | err_local_diskarbitration | código (DADissenter.h).
pub const K_DA_RETURN_ERROR: DAReturn = 0xF8DA_0001_u32 as i32;
pub const K_DA_RETURN_BUSY: DAReturn = 0xF8DA_0002_u32 as i32;
pub const K_DA_RETURN_EXCLUSIVE_ACCESS: DAReturn = 0xF8DA_0004_u32 as i32;
pub const K_DA_RETURN_NOT_FOUND: DAReturn = 0xF8DA_0006_u32 as i32;
pub const K_DA_RETURN_NOT_MOUNTED: DAReturn = 0xF8DA_0007_u32 as i32;
pub const K_DA_RETURN_NOT_PERMITTED: DAReturn = 0xF8DA_0008_u32 as i32;
pub const K_DA_RETURN_NOT_PRIVILEGED: DAReturn = 0xF8DA_0009_u32 as i32;

#[link(name = "DiskArbitration", kind = "framework")]
extern "C" {
    pub static kDADiskDescriptionVolumePathKey: CFStringRef;

    fn DASessionCreate(allocator: CFAllocatorRef) -> DASessionRef;
    fn DASessionScheduleWithRunLoop(session: DASessionRef, run_loop: CFRunLoopRef, mode: CFStringRef);
    fn DASessionUnscheduleFromRunLoop(session: DASessionRef, run_loop: CFRunLoopRef, mode: CFStringRef);

    fn DADiskCreateFromVolumePath(allocator: CFAllocatorRef, session: DASessionRef, path: CFURLRef) -> DADiskRef;
    fn DADiskCopyWholeDisk(disk: DADiskRef) -> DADiskRef;
    fn DADiskGetBSDName(disk: DADiskRef) -> *const c_char;

    pub fn DARegisterDiskDescriptionChangedCallback(
        session: DASessionRef,
        match_: CFDictionaryRef,
        watch: CFArrayRef,
        callback: DADiskDescriptionChangedCallback,
        context: *mut c_void,
    );
    pub fn DARegisterDiskDisappearedCallback(
        session: DASessionRef,
        match_: CFDictionaryRef,
        callback: DADiskDisappearedCallback,
        context: *mut c_void,
    );

    pub fn DADiskUnmount(disk: DADiskRef, options: u32, callback: DADiskUnmountCallback, context: *mut c_void);
    pub fn DADiskEject(disk: DADiskRef, options: u32, callback: DADiskEjectCallback, context: *mut c_void);

    pub fn DADissenterGetStatus(dissenter: DADissenterRef) -> DAReturn;
    pub fn DADissenterGetStatusString(dissenter: DADissenterRef) -> CFStringRef;
}

/// Sesión de DiskArbitration programada en el run loop del hilo que la crea.
/// Al soltarla se desprograma y se libera.
pub struct Session {
    raw: DASessionRef,
    run_loop: CFRunLoopRef,
}

impl Session {
    /// Crea una sesión y la programa en el run loop del hilo actual (modo default).
    pub fn new_on_current_run_loop() -> Option<Self> {
        let raw = unsafe { DASessionCreate(std::ptr::null()) };
        if raw.is_null() {
            return None;
        }
        let run_loop = unsafe { CFRunLoopGetCurrent() };
        unsafe { DASessionScheduleWithRunLoop(raw, run_loop, kCFRunLoopDefaultMode) };
        Some(Self { raw, run_loop })
    }

    pub fn as_raw(&self) -> DASessionRef {
        self.raw
    }

    /// Disco del volumen montado en `path`. `None` si no hay volumen ahí.
    pub fn disk_for_volume(&self, path: &Path) -> Option<Disk> {
        let url = CFURL::from_path(path, true)?;
        let raw = unsafe {
            DADiskCreateFromVolumePath(std::ptr::null(), self.raw, url.as_concrete_TypeRef())
        };
        Disk::from_create_rule(raw)
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        unsafe {
            DASessionUnscheduleFromRunLoop(self.raw, self.run_loop, kCFRunLoopDefaultMode);
            CFRelease(self.raw);
        }
    }
}

/// `DADiskRef` con dueño (se libera al soltarlo).
pub struct Disk {
    raw: DADiskRef,
}

impl Disk {
    fn from_create_rule(raw: DADiskRef) -> Option<Self> {
        (!raw.is_null()).then_some(Self { raw })
    }

    pub fn as_raw(&self) -> DADiskRef {
        self.raw
    }

    /// Disco físico completo que contiene este volumen.
    pub fn whole_disk(&self) -> Option<Disk> {
        Disk::from_create_rule(unsafe { DADiskCopyWholeDisk(self.raw) })
    }

    /// Nombre BSD (`disk4`, `disk4s1`).
    pub fn bsd_name(&self) -> Option<String> {
        let ptr = unsafe { DADiskGetBSDName(self.raw) };
        if ptr.is_null() {
            return None;
        }
        Some(unsafe { CStr::from_ptr(ptr) }.to_string_lossy().into_owned())
    }
}

impl Drop for Disk {
    fn drop(&mut self) {
        unsafe { CFRelease(self.raw) };
    }
}

/// Texto del dissenter, si el sistema lo provee (puede incluir el proceso que bloquea).
pub fn dissenter_status_string(dissenter: DADissenterRef) -> Option<String> {
    let raw = unsafe { DADissenterGetStatusString(dissenter) };
    if raw.is_null() {
        return None;
    }
    let s = unsafe { CFString::wrap_under_get_rule(raw) }.to_string();
    (!s.trim().is_empty()).then_some(s)
}

/// Resultado de una operación asíncrona de DiskArbitration (unmount/eject).
#[derive(Debug, PartialEq)]
pub enum DiskOpResult {
    Done,
    /// El sistema la rechazó: código y texto del dissenter, si lo hay.
    Dissented { status: DAReturn, message: Option<String> },
    TimedOut,
}

/// Estado compartido con el callback. Vive en el heap: si la espera vence, se
/// filtra a propósito para que un callback tardío no escriba en memoria liberada.
struct PendingOp {
    run_loop: CFRunLoopRef,
    result: Option<DiskOpResult>,
}

extern "C" fn on_disk_op_done(_disk: DADiskRef, dissenter: DADissenterRef, context: *mut c_void) {
    let pending = unsafe { &mut *(context as *mut PendingOp) };
    pending.result = Some(if dissenter.is_null() {
        DiskOpResult::Done
    } else {
        DiskOpResult::Dissented {
            status: unsafe { DADissenterGetStatus(dissenter) },
            message: dissenter_status_string(dissenter),
        }
    });
    unsafe { core_foundation_sys::runloop::CFRunLoopStop(pending.run_loop) };
}

/// Ejecuta `start` (que registra `on_disk_op_done` con el contexto dado) y corre
/// el run loop del hilo actual hasta que llegue el callback o venza `timeout`.
/// La sesión del disco debe estar programada en este mismo run loop.
fn run_disk_op(timeout: std::time::Duration, start: impl FnOnce(*mut c_void)) -> DiskOpResult {
    use core_foundation_sys::runloop::CFRunLoopRunInMode;
    use std::time::Instant;

    let pending = Box::into_raw(Box::new(PendingOp {
        run_loop: unsafe { CFRunLoopGetCurrent() },
        result: None,
    }));
    start(pending as *mut c_void);

    let deadline = Instant::now() + timeout;
    loop {
        if let Some(result) = unsafe { (*pending).result.take() } {
            drop(unsafe { Box::from_raw(pending) });
            return result;
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return DiskOpResult::TimedOut; // `pending` se filtra (ver PendingOp)
        }
        unsafe { CFRunLoopRunInMode(kCFRunLoopDefaultMode, remaining.as_secs_f64(), 0) };
    }
}

/// Desmonta todos los volúmenes del disco completo `whole`.
pub fn unmount_whole_disk(whole: &Disk, timeout: std::time::Duration) -> DiskOpResult {
    run_disk_op(timeout, |ctx| unsafe {
        DADiskUnmount(whole.as_raw(), K_DA_DISK_UNMOUNT_OPTION_WHOLE, on_disk_op_done, ctx)
    })
}

/// Expulsa el disco completo `whole` (ya desmontado).
pub fn eject_disk(whole: &Disk, timeout: std::time::Duration) -> DiskOpResult {
    run_disk_op(timeout, |ctx| unsafe {
        DADiskEject(whole.as_raw(), K_DA_DISK_EJECT_OPTION_DEFAULT, on_disk_op_done, ctx)
    })
}

/// Mensaje legible para un código `DAReturn`.
pub fn status_message(status: DAReturn) -> String {
    match status {
        K_DA_RETURN_BUSY => "the disk is in use by another process".to_string(),
        K_DA_RETURN_EXCLUSIVE_ACCESS => "another process has exclusive access to the disk".to_string(),
        K_DA_RETURN_NOT_PERMITTED => "the operation is not permitted".to_string(),
        K_DA_RETURN_NOT_PRIVILEGED => "the operation requires more privileges".to_string(),
        K_DA_RETURN_NOT_FOUND => "the disk was not found".to_string(),
        K_DA_RETURN_NOT_MOUNTED => "the disk is not mounted".to_string(),
        K_DA_RETURN_ERROR => "the system reported an error".to_string(),
        other => format!("the system reported error 0x{:08X}", other as u32),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_message_busy() {
        assert_eq!(status_message(K_DA_RETURN_BUSY), "the disk is in use by another process");
    }

    #[test]
    fn status_message_unknown_code() {
        assert_eq!(status_message(0x1234), "the system reported error 0x00001234");
    }

    #[test]
    fn session_resolves_root_volume() {
        let session = Session::new_on_current_run_loop().expect("DASessionCreate");
        let disk = session.disk_for_volume(Path::new("/")).expect("disk for /");
        let bsd = disk.bsd_name().expect("bsd name");
        assert!(bsd.starts_with("disk"), "unexpected bsd name {bsd}");
        let whole = disk.whole_disk().expect("whole disk");
        assert!(whole.bsd_name().unwrap().starts_with("disk"));
    }
}

//! Native power policy and conservative NTFS journal checkpoints.
//! Journal access may require extra Windows privileges; callers must reconcile
//! metadata when access is denied, the journal wraps, or its identity changes.
/// Windows closes this job handle even if the parent is force-terminated by an
/// installer. Kill the worker (and its children) instead of leaving DLLs locked.
#[cfg(windows)]
pub fn contain_child(
    child: &std::process::Child,
) -> anyhow::Result<std::os::windows::io::OwnedHandle> {
    use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
    use windows::Win32::{
        Foundation::HANDLE,
        System::JobObjects::{
            AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
            JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
            SetInformationJobObject,
        },
    };
    unsafe {
        let job = OwnedHandle::from_raw_handle(CreateJobObjectW(None, None)?.0);
        let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        SetInformationJobObject(
            HANDLE(job.as_raw_handle()),
            JobObjectExtendedLimitInformation,
            (&limits as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
            std::mem::size_of_val(&limits) as u32,
        )?;
        AssignProcessToJobObject(HANDLE(job.as_raw_handle()), HANDLE(child.as_raw_handle()))?;
        Ok(job)
    }
}

#[cfg(all(test, windows))]
mod process_tests {
    #[test]
    fn closing_parent_job_terminates_its_worker() {
        use std::{os::windows::process::CommandExt, process::Command, time::Instant};
        let mut child = Command::new("cmd.exe")
            .args(["/D", "/C", "ping -n 60 127.0.0.1 > nul"])
            .creation_flags(0x08000000)
            .spawn()
            .unwrap();
        let job = super::contain_child(&child).unwrap();
        assert!(child.try_wait().unwrap().is_none());
        drop(job);
        let started = Instant::now();
        while child.try_wait().unwrap().is_none() {
            assert!(started.elapsed().as_secs() < 5);
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
    }
}
#[cfg(windows)]
pub fn power_constrained() -> bool {
    #[repr(C)]
    struct Status {
        ac: u8,
        battery: u8,
        percent: u8,
        saver: u8,
        life: u32,
        full: u32,
    }
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetSystemPowerStatus(status: *mut Status) -> i32;
    }
    let mut status = Status {
        ac: 255,
        battery: 255,
        percent: 255,
        saver: 0,
        life: 0,
        full: 0,
    };
    unsafe { GetSystemPowerStatus(&mut status) != 0 && (status.ac == 0 || status.saver == 1) }
}
#[cfg(not(windows))]
pub fn power_constrained() -> bool {
    false
}

pub fn deadline(pid: u32, seconds: u64) -> std::sync::mpsc::Sender<()> {
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        if receiver.recv_timeout(std::time::Duration::from_secs(seconds))
            == Err(std::sync::mpsc::RecvTimeoutError::Timeout)
        {
            #[cfg(windows)]
            unsafe {
                #[link(name = "kernel32")]
                unsafe extern "system" {
                    fn OpenProcess(access: u32, inherit: i32, pid: u32) -> *mut std::ffi::c_void;
                    fn TerminateProcess(handle: *mut std::ffi::c_void, code: u32) -> i32;
                    fn CloseHandle(handle: *mut std::ffi::c_void) -> i32;
                }
                let handle = OpenProcess(1, 0, pid);
                if !handle.is_null() {
                    TerminateProcess(handle, 124);
                    CloseHandle(handle);
                }
            }
            #[cfg(not(windows))]
            {
                let _ = std::process::Command::new("kill")
                    .args(["-9", &pid.to_string()])
                    .status();
            }
        }
    });
    sender
}

#[cfg(windows)]
pub fn journal_checkpoint(path: &std::path::Path) -> Option<String> {
    use std::{
        fs::OpenOptions,
        os::windows::{fs::OpenOptionsExt, io::AsRawHandle},
    };
    let display = path.to_string_lossy();
    let drive = display.get(..2)?;
    if !drive.as_bytes()[0].is_ascii_alphabetic() || !drive.ends_with(':') {
        return None;
    }
    let file = OpenOptions::new()
        .read(true)
        .share_mode(7)
        .open(format!("\\\\.\\{drive}"))
        .ok()?;
    #[repr(C)]
    #[derive(Default)]
    struct Journal {
        id: u64,
        first: i64,
        next: i64,
        lowest: i64,
        max: i64,
        size: u64,
        delta: u64,
    }
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn DeviceIoControl(
            handle: *mut std::ffi::c_void,
            code: u32,
            input: *const std::ffi::c_void,
            input_size: u32,
            output: *mut std::ffi::c_void,
            output_size: u32,
            returned: *mut u32,
            overlapped: *mut std::ffi::c_void,
        ) -> i32;
    }
    let mut journal = Journal::default();
    let mut returned = 0;
    let ok = unsafe {
        DeviceIoControl(
            file.as_raw_handle(),
            0x000900f4,
            std::ptr::null(),
            0,
            (&mut journal as *mut Journal).cast(),
            std::mem::size_of::<Journal>() as u32,
            &mut returned,
            std::ptr::null_mut(),
        )
    };
    (ok != 0 && returned as usize >= std::mem::size_of::<Journal>())
        .then(|| format!("{}:{}", journal.id, journal.next))
}
#[cfg(not(windows))]
pub fn journal_checkpoint(_path: &std::path::Path) -> Option<String> {
    None
}

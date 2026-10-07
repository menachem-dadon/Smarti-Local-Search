use anyhow::{Result, ensure};
use std::path::Path;
#[cfg(windows)]
pub fn open(path: &Path) -> Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows::{
        Win32::UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOWNORMAL},
        core::PCWSTR,
    };
    let path = std::fs::canonicalize(path)?;
    ensure!(path.is_file(), "File is no longer available");
    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    let result = unsafe {
        ShellExecuteW(
            None,
            PCWSTR::null(),
            PCWSTR(wide.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };
    ensure!(result.0 as isize > 32, "Windows could not open this file");
    Ok(())
}
#[cfg(not(windows))]
pub fn open(_path: &Path) -> Result<()> {
    anyhow::bail!("Windows integration is unavailable on this platform")
}
pub fn reveal(path: &Path) -> Result<()> {
    let path = std::fs::canonicalize(path)?;
    std::process::Command::new("explorer.exe")
        .arg(format!(
            "/select,{}",
            smarti_search_core::engine::display_path(&path)
        ))
        .spawn()?;
    Ok(())
}
#[cfg(windows)]
pub fn configure(autostart: bool, explorer: bool, language: &str) -> Result<()> {
    use winreg::{RegKey, enums::HKEY_CURRENT_USER};
    let root = RegKey::predef(HKEY_CURRENT_USER);
    let exe = std::env::current_exe()?.to_string_lossy().into_owned();
    let (run, _) = root.create_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Run")?;
    if autostart {
        run.set_value("SmartiLocalSearch", &format!("\"{exe}\" --tray"))?
    } else {
        let _ = run.delete_value("SmartiLocalSearch");
    }
    for (base, label, arg) in [
        (
            "Software\\Classes\\*\\shell\\SmartiLocalSearch",
            if language == "he" {
                "חיפוש דומה עם Smarti Local Search"
            } else {
                "Search similar with Smarti Local Search"
            },
            "--similar",
        ),
        (
            "Software\\Classes\\Directory\\shell\\SmartiLocalSearch",
            if language == "he" {
                "חיפוש עם Smarti Local Search"
            } else {
                "Search with Smarti Local Search"
            },
            "--folder",
        ),
    ] {
        if explorer {
            let (key, _) = root.create_subkey(base)?;
            key.set_value("", &label)?;
            key.set_value("Icon", &exe)?;
            let (command, _) = key.create_subkey("command")?;
            command.set_value("", &format!("\"{exe}\" {arg} \"%1\""))?;
        } else {
            let _ = root.delete_subkey_all(base);
        }
    }
    Ok(())
}
#[cfg(not(windows))]
pub fn configure(_autostart: bool, _explorer: bool, _language: &str) -> Result<()> {
    Ok(())
}
#[cfg(windows)]
pub fn power() -> serde_json::Value {
    use windows::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
    let mut status = SYSTEM_POWER_STATUS::default();
    if unsafe { GetSystemPowerStatus(&mut status) }.is_ok() {
        serde_json::json!({"on_battery":status.ACLineStatus==0,"battery_percent":if status.BatteryLifePercent==255{None}else{Some(status.BatteryLifePercent)},"battery_saver":status.SystemStatusFlag==1})
    } else {
        serde_json::json!({"available":false})
    }
}
#[cfg(not(windows))]
pub fn power() -> serde_json::Value {
    serde_json::json!({"available":false})
}

use std::{
    io::ErrorKind,
    path::PathBuf,
    process::{Command, ExitStatus, Stdio},
    sync::LazyLock,
};

use tokio::task::JoinHandle;
use windows::Win32::{
    Foundation::{CloseHandle, HANDLE},
    Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY},
    System::Threading::{GetCurrentProcess, OpenProcessToken},
};

use crate::vrchat::VRCHAT_LOW_PATH;

pub const PROCMON_BACKING_NAME: &str = "VRC-LOG.PML";
pub const PROCMON_CONFIG_NAME: &str = "VRC-LOG.PMC";
pub const PROCMON_CONFIG_BYTES: &[u8] = include_bytes!("../VRC-LOG.PMC");

pub static TEMP_DIR_PATH: LazyLock<PathBuf> = LazyLock::new(std::env::temp_dir);

/// # Checks if the process has elevated privileges.
/// # Errors
/// Will return `Err` if `OpenProcessToken`, `GetTokenInformation`, or `CloseHandle` fail.
pub fn is_elevated() -> windows::core::Result<bool> {
    unsafe {
        let mut h_token = HANDLE(0 as _);
        if let Err(error) = OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &raw mut h_token) {
            CloseHandle(h_token)?;
            return Err(error);
        }

        let mut token_elevation = core::mem::zeroed::<TOKEN_ELEVATION>();
        if let Err(error) = GetTokenInformation(
            h_token,
            TokenElevation,
            Some(core::ptr::addr_of_mut!(token_elevation).cast()),
            u32::try_from(size_of::<TOKEN_ELEVATION>())?,
            &mut 0,
        ) {
            CloseHandle(h_token)?;
            return Err(error);
        }

        CloseHandle(h_token)?;
        Ok(token_elevation.TokenIsElevated != 0)
    }
}

/// # Installs Process Monitor using Winget
/// # Errors
/// Will return `Err` if `Command::spawn` or `Command::wait` fails.
pub fn install_procmon() -> std::io::Result<ExitStatus> {
    Command::new("winget")
        .arg("install")
        .arg("Microsoft.SysInternals.ProcessMonitor")
        .arg("--accept-package-agreements")
        .arg("--accept-source-agreements")
        .arg("--silent")
        .stdout(Stdio::null())
        .spawn()?
        .wait()
}

/// # Starts Process Monitor using the provided program.
/// # Errors
/// Will return `Err` if `Command::spawn` or `Command::wait` fails.
pub fn start_procmon() -> std::io::Result<ExitStatus> {
    std::fs::remove_file(TEMP_DIR_PATH.join(PROCMON_BACKING_NAME))?;

    Command::new("Procmon.exe")
        .arg("/AcceptEula")
        .arg("/BackingFile")
        .arg(TEMP_DIR_PATH.join(PROCMON_BACKING_NAME))
        .arg("/LoadConfig")
        .arg(TEMP_DIR_PATH.join(PROCMON_CONFIG_NAME))
        .arg("/Minimized")
        .arg("/Quiet")
        .spawn()?
        .wait()
}

/// # Processes the Process Monitor capture into CSV.
/// # Errors
/// Will return `Err` if `Command::spawn` or `Command::wait` fails.
pub fn process_procmon() -> std::io::Result<ExitStatus> {
    Command::new("Procmon.exe")
        .arg("/AcceptEula")
        .arg("/OpenLog")
        .arg(TEMP_DIR_PATH.join(PROCMON_BACKING_NAME))
        .arg("/SaveAs")
        .arg(VRCHAT_LOW_PATH.join("Procmon.csv"))
        .arg("/Minimized")
        .arg("/Quiet")
        .spawn()?
        .wait()
}

/// # Terminate Process Monitor.
/// # Errors
/// Will return `Err` if `Command::spawn` or `Command::wait` fails.
pub fn terminate_procmon() -> std::io::Result<ExitStatus> {
    Command::new("Procmon.exe")
        .arg("/AcceptEula")
        .arg("/Minimized")
        .arg("/Quiet")
        .arg("/Terminate")
        .spawn()?
        .wait()
}

/// # Spawns the Process Monitor watcher in a background thread.
/// # Errors
/// Will return `Err` if `tokio::task::spawn_blocking` fails.
pub fn spawn_procmon_watcher() -> JoinHandle<anyhow::Result<()>> {
    tokio::task::spawn_blocking(start_procmon_watcher)
}

/// # Starts the Process Monitor watcher.
/// # Errors
/// Will return `Err` if anything fails.
pub fn start_procmon_watcher() -> anyhow::Result<()> {
    terminate_procmon()?;

    std::fs::write(
        TEMP_DIR_PATH.join(PROCMON_CONFIG_NAME),
        PROCMON_CONFIG_BYTES,
    )?;

    loop {
        if TEMP_DIR_PATH.join(PROCMON_BACKING_NAME).exists() {
            process_procmon()?;
        }

        /* Block until Procmon is closed by the user */
        if let Err(error) = start_procmon() {
            if error.kind() == ErrorKind::NotFound {
                install_procmon()?;
                start_procmon()?;
            } else {
                anyhow::bail!(error)
            }
        }
    }
}

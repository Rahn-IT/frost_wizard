use std::{env::current_exe, ffi::CString};

use windows::Win32::{
    Foundation::HANDLE,
    Security::{
        GetTokenInformation, TOKEN_ELEVATION, TOKEN_QUERY, TokenElevation,
    },
    System::Console::{ATTACH_PARENT_PROCESS, AttachConsole},
    System::Threading::{GetCurrentProcess, OpenProcessToken},
    UI::Shell::ShellExecuteA,
    UI::WindowsAndMessaging::SW_NORMAL,
};

pub fn attach_and_ensure_admin() {
    let attach_result = attach();

    match elevated() {
        Err(err) => {
            eprintln!("Error checking admin priveliges: {}", err);
        }
        Ok(false) => match attach_result {
            Ok(()) => {
                eprintln!("Installation requires admin access!");
                std::process::exit(1);
            }
            Err(_) => {
                restart_with_admin_prompt();
                std::process::exit(0);
            }
        },
        Ok(true) => (),
    }
}

pub fn attach() -> Result<(), windows_result::Error> {
unsafe { AttachConsole(ATTACH_PARENT_PROCESS) }
}

pub fn elevated() -> Result<bool, windows_result::Error> {
    Ok(get_elevated_info()?.TokenIsElevated != 0)
}

fn get_elevated_info() -> Result<TOKEN_ELEVATION, windows_result::Error> {
    let token = get_process_tokem()?;
    let mut info: TOKEN_ELEVATION = TOKEN_ELEVATION::default();
    let mut n = 0;
    unsafe {
        GetTokenInformation(
            token,
            TokenElevation,
            Some(&mut info as *mut _ as *mut std::ffi::c_void),
            size_of::<TOKEN_ELEVATION>() as u32,
            &mut n,
        )
    }?;
    Ok(info)
}

fn get_process_tokem() -> Result<HANDLE, windows_result::Error> {
    let current_process = unsafe { GetCurrentProcess() };
    let mut token = HANDLE::default();
    unsafe { OpenProcessToken(current_process, TOKEN_QUERY, &mut token) }?;
    Ok(token)
}

pub fn restart_with_admin_prompt() {
    let runas = CString::from(c"runas");
    let runas_ptr = windows::core::PCSTR::from_raw(runas.as_ptr() as *const u8);
    let exe = CString::new(
        current_exe()
            .expect("requesting current exe name should work")
            .into_os_string()
            .into_encoded_bytes(),
    )
    .unwrap();
    let exe_ptr = windows::core::PCSTR::from_raw(exe.as_ptr() as *const u8);

    let _instance = unsafe { ShellExecuteA(None, runas_ptr, exe_ptr, None, None, SW_NORMAL) };
}

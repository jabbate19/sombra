use std::env::current_exe;
use std::fs::{read_to_string, File, read_link};
use std::process::id;
use crate::util::sha1sum;
use std::path::PathBuf;
#[cfg(target_os = "windows")]
use windows::Win32::System::ProcessStatus::K32GetProcessImageFileNameA;
pub fn check_persistence() -> bool {
    let path = if cfg!(target_os = "windows") {
        "C:\\Users\\Public\\.runtime".to_owned()
    } else {
        "/tmp/.runtime".to_owned()
    };

    match read_to_string(&path) {
        Ok(pid) => {
            let exe = match get_exe(pid) {
                Some(x) => x,
                None => return false
            };
            return sha1sum(&exe).unwrap() == sha1sum(&current_exe().unwrap()).unwrap();
        },
        Err(_) => {
            return false;
        }
    };
}

#[cfg(target_os = "linux")]
fn get_exe(pid: String) -> Option<PathBuf> {
    read_link(format!("/proc/{}/exe", pid)).ok()
}

#[cfg(target_os = "freebsd")]
fn get_exe(pid: String) -> String {
    let cmd = Command::new("procstat").arg("-b").arg(&pid).output().unwrap();
    let out_str = str::from_utf8(&cmd.stdout).unwrap();
    Some(PathBuf::from(out_str.split_whitespace().last().unwrap()))
}

#[cfg(target_os = "windows")]
fn get_exe(pid: String) {
    let p: u32 = pid.parse().unwrap();
    let mut file_raw = [10 as u16; 1000];
    unsafe {
        let process_handle = match OpenProcess(PROCESS_ALL_ACCESS, false, p) {
            Ok(x) => x,
            Err(_) => return None
        };
        let x: usize = K32GetModuleFileNameExW(process_handle, None, &mut file_raw);
        let data = Vec::from(file_raw);
        return PathBuf::from(String::from_utf16(&data[0..x]).unwrap());
    }
}

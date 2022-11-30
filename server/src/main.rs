mod icmp;

use cursive::{
    align::HAlign,
    event::Key,
    menu,
    theme::{BaseColor, BorderStyle, Color},
    traits::*,
    view::ScrollStrategy,
    views::{Button, Dialog, EditView, LinearLayout, PaddedView, ScrollView, SelectView, TextView},
    Cursive,
};
use default_net::get_default_interface;
use icmp::IcmpListener;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::from_str;
use std::{
    collections::HashMap,
    fs::read_to_string,
    io::{Read, Write},
    sync::{Arc, Mutex},
};

static ending: &str = "DONE";

//#[derive(Debug)]
struct Host {
    //name: String,
    os: String,
    stream: LigmaListener,
    //connected: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Config {
    teams: u8,
    setup: Vec<Preset>,
    breaks: HashMap<String, Vec<Break>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Preset {
    ip: String,
    hostname: String,
    os: String,
    groups: Vec<String>,
    connected: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Break {
    name: String,
    cmd: Vec<Vec<String>>,
}

fn main() {
    let config_file = read_to_string("config.json").unwrap();
    let config: Config = from_str(&config_file).unwrap();
    let config2 = config.clone();
    let config3 = config.clone();
    let mut siv = cursive::default();
    let streams = Arc::new(Mutex::new(HashMap::<String, Host>::new()));

    let mut team_ip = vec![];
    team_ip.push(vec![]);
    for team in 1..=config.teams {
        let mut ip_list = vec![];
        for preset in &config.setup {
            let ip = preset.ip.replace("x", &format!("{}", team));
            ip_list.push(ip.clone());
            let mut stream_lock = streams.lock().unwrap();
            stream_lock.insert(
                ip.clone(),
                Host {
                    //name: preset.hostname.clone(),
                    os: preset.os.clone(),
                    //stream: LigmaListener::new("br-c62428f26c3d".to_string(), ip.parse().unwrap()),
                    stream: LigmaListener::new(
                        get_default_interface().unwrap().name,
                        ip.parse().unwrap(),
                    ),
                    //connected: true
                },
            );
        }
        team_ip.push(ip_list);
    }
    let team_ip = Arc::new(Mutex::new(team_ip));
    let thread_arc = streams.clone();
    let thread_arc2 = streams.clone();
    let thread_arc3 = streams.clone();
    let thread_arc4 = streams.clone();
    let thread_arc5 = streams.clone();
    let thread_arc6 = streams.clone();

    let mut theme = siv.current_theme().clone();

    //theme.shadow = !theme.shadow;
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
            SelectView::<String>::new()
                .h_align(HAlign::Center)
                .on_submit(move |siv, socket: &str| {
                    let thread_arc_in = thread_arc.clone();
                    pick_object(siv, socket, thread_arc_in);
                })
                .with_name("table"),
        ))
        .child(PaddedView::lrtb(
            2,
            2,
            0,
            0,
            Button::new("Select All", move |s| {
                let thread_arc_in = thread_arc2.clone();
                select_all(s, thread_arc_in);
            }),
        ));
    siv.menubar()
        .add_leaf("All Systems", move |s| {
            if let Ok(t) = thread_arc6.lock() {
                s.call_on_name("table", |view: &mut SelectView| {
                    let mut keys: Vec<&String> = t.keys().collect();
                    keys.sort();
                    view.clear();
                    view.add_all_str(keys);
                });
            }
        })
        .add_subtree(
            "OS",
            menu::Tree::new().with(move |tree| {
                let thread_arc_in = thread_arc3.clone();
                tree_os(tree, thread_arc_in);
            }),
        )
        .add_subtree(
            "Teams",
            menu::Tree::new().with(move |tree| {
                let thread_arc_in = thread_arc4.clone();
                tree_teams(tree, thread_arc_in, config2, team_ip);
            }),
        )
        .add_subtree(
            "Boxes",
            menu::Tree::new().with(move |tree| {
                let thread_arc_in = thread_arc5.clone();
                tree_boxes(tree, thread_arc_in, config3);
            }),
        )
        .add_delimiter()
        .add_leaf("Quit", |s| s.quit());

    // When `autohide` is on (default), the menu only appears when active.
    // Turning it off will leave the menu always visible.
    // Try uncommenting this line!

    // siv.set_autohide_menu(false);

    siv.add_global_callback(Key::Esc, |s| s.select_menubar());

    siv.add_layer(Dialog::around(table).title("Available Clients"));

    siv.run();
}

fn pick_object(siv: &mut Cursive, socket: &str, thread_arc: Arc<Mutex<HashMap<String, Host>>>) {
    let thread_arc2 = thread_arc.clone();
    siv.add_layer(
        Dialog::new()
            .title(format!("{}", socket))
            .padding_lrtb(1, 1, 1, 0)
            .content(
                LinearLayout::vertical()
                    .child(ScrollView::new(TextView::empty().with_name("shellout")).scroll_strategy(ScrollStrategy::StickToBottom))
                    .child(
                        EditView::new()
                            .on_submit(move |s, cmd| {
                                shell(s, cmd, thread_arc2.clone());
                            })
                            .with_name("shell"),
                    ),
            )
            .button("Exit", |s| {
                s.pop_layer();
            }),
    );
}

fn shell(s: &mut Cursive, cmd: &str, thread_arc: Arc<Mutex<HashMap<String, Host>>>) {
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
        view.get_shared_content().append(format!("> {}\n", cmd));
    });
    for (_, sock) in s.find_name::<SelectView>("table").unwrap().iter() {
        let mut lock = thread_arc.lock().unwrap();
        let shell_stream = &mut lock.get_mut(sock).unwrap().stream;
        shell_stream.write(cmd.as_bytes()).unwrap();
        let mut first = true;
        loop {
            let mut buffer = [0; 250000];
            let output = shell_stream.read(&mut buffer).unwrap();
            if output == 0 {
                s.call_on_name("shell", |view: &mut EditView| {
                    view.set_content("");
                    view.disable();
                })
                .unwrap();
                s.call_on_name("shellout", |view: &mut TextView| {
                    view.get_shared_content()
                        .append(format!("{} < CONNECTION LOST\n", sock));
                })
                .unwrap();
                // lock.remove(sock);
                // s.call_on_name(
                //     "table",
                //     |view: &mut SelectView| {
                //         let keys = lock.keys();
                //         view.clear();
                //         view.add_all_str(keys);
                //     },
                // );
                break;
            }
            let outstr = String::from_utf8(Vec::from(buffer)).unwrap();
            let outstr = outstr.trim_matches(char::from(0));
            let mut end_check = String::from(outstr);
            let outstr = match outstr.strip_suffix(ending) {
                Some(x) => x,
                None => outstr,
            };
            s.call_on_name("shell", |view: &mut EditView| {
                view.set_content("");
            })
            .unwrap();
            let printed = match first {
                true => {
                    first = false;
                    format!("{} < {}", sock, outstr,)
                }
                false => {
                    format!("{}", outstr,)
                }
            };
            s.call_on_name("shellout", |view: &mut TextView| {
                view.get_shared_content().append(printed);
            })
            .unwrap();

            let _ = end_check.drain(..end_check.len() - 5);
            if end_check == ending {
                break;
            }
        }
        s.call_on_name("shellout", |view: &mut TextView| {
            view.get_shared_content().append("\n".to_string());
        })
        .unwrap();
    }
}

fn select_all(s: &mut Cursive, thread_arc: Arc<Mutex<HashMap<String, Host>>>) {
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
                                let thread_arc2 = thread_arc.clone();
                                shell(s, cmd, thread_arc2);
                            })
                            .with_name("shell"),
                    ),
            )
            .button("Exit", |s| {
                s.pop_layer();
            }),
    );
}

fn tree_os(tree: &mut menu::Tree, thread_arc: Arc<Mutex<HashMap<String, Host>>>) {
    for os in ["WINDOWS", "LINUX", "BSD", "OTHER"] {
        let thread_arc2 = thread_arc.clone();
        tree.add_item(menu::Item::leaf(format!("{}", os), move |s| {
            if let Ok(t) = thread_arc2.lock() {
                s.call_on_name("table", |view: &mut SelectView| {
                    let mut keys: Vec<&String> = t
                        .keys()
                        .filter(|x| t.get(&format!("{}", x)).unwrap().os == os)
                        .collect();
                    keys.sort();
                    view.clear();
                    view.add_all_str(keys);
                });
            }
        }))
    }
}

fn tree_boxes(
    tree: &mut menu::Tree,
    thread_arc: Arc<Mutex<HashMap<String, Host>>>,
    config: Config,
) {
    for preset in config.setup {
        let thread_arc2 = thread_arc.clone();
        tree.add_item(menu::Item::leaf(
            format!("{} ({})", preset.ip, preset.hostname),
            move |s| {
                if let Ok(t) = thread_arc2.lock() {
                    s.call_on_name("table", |view: &mut SelectView| {
                        let mut keys: Vec<&String> = t
                            .keys()
                            .filter(|x| {
                                let ip = x.split(':').next().unwrap();
                                let re = Regex::new(&preset.ip.replace("x", r"\d{1,3}")).unwrap();
                                re.is_match(ip)
                            })
                            .collect();
                        keys.sort();
                        view.clear();
                        view.add_all_str(keys);
                    });
                }
            },
        ))
    }
}

fn tree_teams(
    tree: &mut menu::Tree,
    thread_arc: Arc<Mutex<HashMap<String, Host>>>,
    config: Config,
    team_ip: Arc<Mutex<Vec<Vec<String>>>>,
) {
    for i in 1..config.teams + 1 {
        let thread_arc2 = thread_arc.clone();
        let ips = team_ip.clone();
        tree.add_item(menu::Item::leaf(format!("Team {}", i), move |s| {
            if let Ok(t) = thread_arc2.lock() {
                s.call_on_name("table", |view: &mut SelectView| {
                    let mut keys: Vec<&String> = t
                        .keys()
                        .filter(|x| {
                            let ip = x.split(':').next().unwrap();
                            ips.lock()
                                .unwrap()
                                .to_vec()
                                .get(i as usize)
                                .unwrap()
                                .contains(&String::from(ip))
                        })
                        .collect();
                    keys.sort();
                    view.clear();
                    view.add_all_str(keys);
                });
            }
        }))
    }
}

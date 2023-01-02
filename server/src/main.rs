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

struct Host {
    os: String,
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
    let config: Config = read_to_string("config.json").map(|config_file| from_str(&config_file)).unwrap().unwrap();
    let config_teams = config.clone();
    let config_boxes = config.clone();
    
    let streams = Arc::new(Mutex::new(HashMap::<String, Host>::new()));
    let listener = Arc::new(Mutex::new(IcmpListener::new("tap0".to_owned())));
    let mut team_ip = vec![];
    team_ip.push(vec![]);
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
            let mut stream_lock = streams.lock().unwrap();
            stream_lock.insert(
                ip.clone(),
                Host {
                    os: preset.os.clone(),
                },
            );
        }
        team_ip.push(ip_list);
    }
    let team_ip = Arc::new(Mutex::new(team_ip));
    let table_streams_arc = streams.clone();
    let select_all_streams_arc = streams.clone();
    let all_streams_arc = streams.clone();
    let os_streams_arc = streams.clone();
    let teams_streams_arc = streams.clone();
    let boxes_streams_arc = streams.clone();
    
    let table_listener_arc = listener.clone();
    let select_all_listener_arc = listener.clone();

    let mut siv = cursive::default();

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
            SelectView::<String>::new()
                .h_align(HAlign::Center)
                .on_submit(move |siv, socket: &str| {
                    let pick_object_streams_arc = table_streams_arc.clone();
                    let pick_object_listener_arc = table_listener_arc.clone();
                    pick_object(siv, socket, pick_object_streams_arc, pick_object_listener_arc);
                })
                .with_name("table"),
        ))
        .child(PaddedView::lrtb(
            2,
            2,
            0,
            0,
            Button::new("Select All", move |s| {
                let select_all_streams_arc_internal = select_all_streams_arc.clone();
                let select_all_listener_arc_internal = select_all_listener_arc.clone();
                select_all(s, select_all_streams_arc_internal, select_all_listener_arc_internal);
            }),
        ));

    siv.menubar()
        .add_leaf("All Systems", move |s| {
            if let Ok(t) = all_streams_arc.lock() {
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
                let tree_os_streams_arc = os_streams_arc.clone();
                tree_os(tree, tree_os_streams_arc);
            }),
        )
        .add_subtree(
            "Teams",
            menu::Tree::new().with(move |tree| {
                let tree_teams_streams_arc = teams_streams_arc.clone();
                tree_teams(tree, tree_teams_streams_arc, config_teams, team_ip);
            }),
        )
        .add_subtree(
            "Boxes",
            menu::Tree::new().with(move |tree| {
                let tree_boxes_streams_arc = boxes_streams_arc.clone();
                tree_boxes(tree, tree_boxes_streams_arc, config_boxes);
            }),
        )
        .add_delimiter()
        .add_leaf("Quit", |s| s.quit());

    siv.add_global_callback(Key::Esc, |s| s.select_menubar());

    siv.add_layer(Dialog::around(table).title("Available Clients"));

    siv.run();
}

fn pick_object(
    siv: &mut Cursive,
    socket: &str,
    streams_arc: Arc<Mutex<HashMap<String, Host>>>,
    mut listener: Arc<Mutex<IcmpListener>>,
) {
    let shell_streams_arc = streams_arc.clone();
    let set = Arc::new(Some(String::from(socket)));
    siv.add_layer(
        Dialog::new()
            .title(format!("{}", socket))
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
                                let shell_listener = listener.clone();
                                let shell_set = set.clone();
                                shell(s, cmd, shell_streams_arc.clone(), shell_listener, shell_set);
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
    s: &mut Cursive,
    cmd: &str,
    streams_arc: Arc<Mutex<HashMap<String, Host>>>,
    mut listener_arc: Arc<Mutex<IcmpListener>>,
    specific_ip: Arc<Option<String>>,
) {
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
        let mut lock = streams_arc.lock().unwrap();
        let sock2 = sock.split_whitespace().next().unwrap();
        match *specific_ip {
            Some(ref specific) => {
                let specific2 = specific.split_whitespace().next().unwrap();
                if sock2 != specific2 {
                    continue;
                }
            }
            None => {}
        }
        let mut listener = listener_arc.lock().unwrap();
        listener.set_dest(sock2.parse().unwrap());
        listener.write(cmd.as_bytes()).unwrap();
        let mut first = true;
        loop {
            let mut buffer = [0; 250000];
            let output = listener.read(&mut buffer).unwrap();
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

            let _ = end_check.drain(..end_check.len() - 4);
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

fn select_all(
    s: &mut Cursive,
    streams_arc: Arc<Mutex<HashMap<String, Host>>>,
    listener: Arc<Mutex<IcmpListener>>,
) {
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
                                let l2 = listener.clone();
                                let shell_streams_arc = streams_arc.clone();
                                shell(s, cmd, shell_streams_arc, l2, Arc::new(None));
                            })
                            .with_name("shell"),
                    ),
            )
            .button("Exit", |s| {
                s.pop_layer();
            }),
    );
}

fn tree_os(tree: &mut menu::Tree, streams_arc: Arc<Mutex<HashMap<String, Host>>>) {
    for os in ["WINDOWS", "LINUX", "BSD", "OTHER"] {
        let leaf_streams_arc = streams_arc.clone();
        tree.add_item(menu::Item::leaf(format!("{}", os), move |s| {
            if let Ok(t) = leaf_streams_arc.lock() {
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
                        let keys: Vec<&String> = t
                            .keys()
                            .filter(|x| {
                                let ip = x.split(':').next().unwrap();
                                let re = Regex::new(&preset.ip.replace("x", r"\d{1,3}")).unwrap();
                                re.is_match(ip)
                            })
                            .collect();
                        let mut new_keys: Vec<String> = Vec::new();
                        for key in keys {
                            let mut parts = key.split(".");
                            let part: usize = if parts.next().unwrap() == "192" {
                                let x: usize = parts.last().unwrap().parse().unwrap();
                                (x + 6) / 8
                            } else {
                                parts.next().unwrap().parse().unwrap()
                            };
                            new_keys.push(format!("{} (Team {})", key, part));
                        }
                        new_keys.sort();
                        view.clear();
                        view.add_all_str(new_keys);
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

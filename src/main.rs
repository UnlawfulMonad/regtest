// regtest - Interactive frontend for the regex crate
// Copyright (C) 2016  Lucas Salibian
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See
// the GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

#![allow(dead_code)]
#![allow(non_upper_case_globals)]
#![allow(unused_must_use)]

use std::io;
use std::io::Write;
use std::path::PathBuf;
use std::time::Instant;

use regex::Regex;
use clap::{Arg, ArgAction, Command};
use rustyline::DefaultEditor;
use directories::ProjectDirs;

bitflags::bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    struct Config: u32 {
        const VERBOSE_ERRORS = 0b00000001;
        const CAPTURE_GROUPS = 0b00000010;
        const COMPILE_TIME   = 0b00000100;
    }
}

impl Default for Config {
    fn default() -> Config {
        Config::VERBOSE_ERRORS | Config::COMPILE_TIME
    }
}

const HELP: &str = "\
:t - Toggle compile time display
:g - Toggle capture groups display
:v - Toggle verbose errors
:h - Print this menu
:q - Quit";

const MENU_PRMT: &str = ":b - Go back to the regex prompt";

/// Define the possible things that may happen after a menu
/// ineration within any of the sub menus (regex input or
/// testing input).
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum Action {
    Continue,
    Loop,
    ToRegexPrompt,
    Exit,
}

/// Check if a given `line` corresponds to a menu command.
fn options_menu(line: &str, config: &mut Config) -> Action {
    let mut stderr = io::stderr();
    match line {
        ":q" => Action::Exit,

        ":v" => {
            config.toggle(Config::VERBOSE_ERRORS);
            if config.contains(Config::VERBOSE_ERRORS) {
                write!(stderr, "Verbose errors: on\n");
            } else {
                write!(stderr, "Verbose errors: off\n");
            }
            Action::Loop
        }

        ":t" => {
            config.toggle(Config::COMPILE_TIME);
            if config.contains(Config::COMPILE_TIME) {
                write!(stderr, "Show compile time: on\n");
            } else {
                write!(stderr, "Show compile time: off\n");
            }
            Action::Loop
        }

        ":b" => Action::ToRegexPrompt,

        ":g" => {
            config.toggle(Config::CAPTURE_GROUPS);
            if config.contains(Config::CAPTURE_GROUPS) {
                write!(stderr, "Show capture groups: on\n");
            } else {
                write!(stderr, "Show capture groups: off\n");
            }
            Action::Loop
        }

        ":h" | ":?" => {
            write!(stderr, "{}\n", HELP);
            Action::Loop
        }

        _ => Action::Continue,
    }
}

/// Show a prompt saying "n>" requesting that a regex be input.
/// If this function returns true, the user will be prompted
/// to input a regex and if false the program will exit.
fn regex_prompt(editor: &mut DefaultEditor, config: &mut Config) -> bool {
    let mut stderr = io::stderr();

    let line = editor.readline("Input> ").expect("Failed to read line!");
    editor.add_history_entry(line.as_str());

    match options_menu(&line, config) {
        Action::Continue => {}
        Action::ToRegexPrompt | Action::Loop => return true,
        Action::Exit => return false,
    }

    let t1 = Instant::now();
    let reg = match Regex::new(&line) {
        Ok(r) => r,
        Err(e) => {
            if config.contains(Config::VERBOSE_ERRORS) {
                write!(stderr, "Error compiling regex: {:?}\n", e);
            } else {
                stderr.write(b"Failed to compile regex\n");
                stderr.write(b"Turn on verbose errors with :v\n");
            }
            return true;
        }
    };

    if config.contains(Config::COMPILE_TIME) {
        write!(stderr, "Regex compiled in {}ns\n", t1.elapsed().as_nanos());
    }

    prompt(editor, &reg, config)
}

// If this returns false, the program with exit.
// If it returns true, the prompt for a new regex
// will be shown.
fn prompt(editor: &mut DefaultEditor, reg: &Regex, config: &mut Config) -> bool {
    let mut stderr = io::stderr();
    let prompt_str = format!("Regex({})> ", reg.as_str());

    loop {
        let line = editor.readline(&prompt_str).expect("Failed to read line");
        editor.add_history_entry(line.as_str());

        match options_menu(&line, config) {
            Action::Exit => return false,
            Action::Loop => continue,
            Action::ToRegexPrompt => return true,
            Action::Continue => {
                if config.contains(Config::CAPTURE_GROUPS) {
                    let caps = reg.captures_iter(&line).enumerate();
                    write!(stderr, "Captures:\n");
                    for (i, outer_cap) in caps {
                        for (j, cap) in outer_cap.iter().enumerate() {
                            write!(stderr,
                                   "{}:{}: {}\n",
                                   i,
                                   j,
                                   if let Some(c) = cap { c.as_str() } else { "None" });
                        }
                    }
                } else if reg.is_match(&line) {
                    write!(stderr, "Matched\n");
                } else {
                    write!(stderr, "Failed to match\n");
                }
            }
        }
    }
}

/// Determine the history file path, creating its directory if needed.
/// Failure is non-fatal — only a warning is shown.
fn with_history_file<F>(mut f: F)
where
    F: FnMut(&PathBuf),
{
    let dirs = match ProjectDirs::from("", "Lucas Salibian", "regtest") {
        Some(d) => d,
        None => {
            println!("Failed to determine history file location");
            return;
        }
    };
    let data_dir = dirs.data_dir();
    if let Err(e) = std::fs::create_dir_all(data_dir) {
        println!("Failed to create data directory: {:?}", e);
        return;
    }
    let mut path = data_dir.to_path_buf();
    path.push("history");
    f(&path);
}

fn main() {
    let mut config = Config::default();

    let matches = Command::new("regtest")
        .version(env!("CARGO_PKG_VERSION"))
        .author("Lucas Salibian <lucas.salibian@gmail.com>")
        .about("Test regexes from the command line")
        .arg(Arg::new("no-verbose-errors")
            .long("no-verbose-errors")
            .help("Disable verbose errors when the regex fails to compile")
            .action(ArgAction::SetTrue))
        .arg(Arg::new("capture")
            .short('c')
            .long("capture")
            .help("Enable capture group display after matching test")
            .action(ArgAction::SetTrue))
        .arg(Arg::new("no-compile-time")
            .long("no-compile-time")
            .help("Disable showing the amount of time it took to compile the regular expression.")
            .action(ArgAction::SetTrue))
        .get_matches();

    if matches.get_flag("no-verbose-errors") {
        config.remove(Config::VERBOSE_ERRORS);
    }

    if matches.get_flag("capture") {
        config.insert(Config::CAPTURE_GROUPS);
    }

    let mut editor = DefaultEditor::new().unwrap();

    with_history_file(|path| { editor.load_history(path); });

    loop {
        if !regex_prompt(&mut editor, &mut config) {
            break;
        }
    }

    with_history_file(|path| { editor.save_history(path).unwrap(); });
}

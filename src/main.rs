/*
 * regtest - Interactive frontend for the regex crate
 * Copyright (C) 2016  Lucas Salibian
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See
 * the GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

#![allow(dead_code)]
#![allow(non_upper_case_globals)]
#![allow(unused_must_use)]

extern crate regex;
extern crate time;
#[macro_use]
extern crate bitflags;
extern crate rustyline;
extern crate clap;

use std::io;
use std::io::Write;
use std::env;
use std::default::Default;
use std::path::PathBuf;

use regex::Regex;

use clap::{Arg, App};

use rustyline::Editor;

bitflags! {
    flags Config: u32 {
        const VERBOSE_ERRORS = 0b00000001,
        const CAPTURE_GROUPS = 0b00000010,
        const COMPILE_TIME   = 0b00000100,
    }
}

impl Default for Config {
    fn default() -> Config {
        VERBOSE_ERRORS | COMPILE_TIME
    }
}

const HELP: &'static str = "\
:t - Toggle compile time display
:g - Toggle capture groups display
:v - Toggle verbose errors
:h - Print this menu
:q - Quit";

const MENU_PRMT: &'static str = ":b - Go back to the regex prompt";

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
    // What can you do from here?
    match &line as &str {
        // Quit on :q
        ":q" => Action::Exit,

        // Toggle verbose errors
        ":v" => {
            config.toggle(VERBOSE_ERRORS);
            if config.contains(VERBOSE_ERRORS) {
                write!(stderr, "Verbose errors: on\n");
            } else {
                write!(stderr, "Verbose errors: off\n");
            }
            Action::Loop
        },

        // Toggle message reporting time to
        // compile regex
        ":t" => {
            config.toggle(COMPILE_TIME);
            if config.contains(COMPILE_TIME) {
                write!(stderr, "Show compile time: on\n");
            } else {
                write!(stderr, "Show compile time: off\n");
            }
            Action::Loop
        },

        // When in regex test menu, go back to regex
        // prompt. Otherwise, do nothing
        ":b" => Action::ToRegexPrompt,

        // Toggle displaying capture groups
        ":g" => {
            config.toggle(CAPTURE_GROUPS);
            if config.contains(CAPTURE_GROUPS) {
                write!(stderr, "Show capture groups: on\n");
            } else {
                write!(stderr, "Show capture groups: off\n");
            }
            Action::Loop
        },

        // Display help
        ":h" | ":?" => {
            write!(stderr, "{}\n", HELP);
            Action::Loop
        },

        // Continue
        _ => Action::Continue,
    }
}

/// Show a prompt saying "n>" requesting that a regex be input.
/// If this function returns true, the user will be prompted
/// to input a regex and if false the program will exit.
fn regex_prompt(editor: &mut Editor<()>, config: &mut Config) -> bool {
    // Get stderr up here just for convienience
    let mut stderr = io::stderr();

    // Read the line and add it to history
    let line = editor.readline("Input> ").expect("Failed to read line!");
    editor.add_history_entry(&line);

    // Process the line against the options menu
    match options_menu(&line, config) {
        Action::Continue => {},
        Action::ToRegexPrompt | Action::Loop => return true,
        Action::Exit => return false,
    }
 
    // Get the time for compiling regex
    let t1 = time::now();
    let reg = match Regex::new(&line) {
        Ok(r) => r,
        Err(e) => {
            if config.contains(VERBOSE_ERRORS) {
                write!(stderr, "Error compiling regex: {:?}\n", e);
            } else {
                stderr.write(b"Failed to compile regex\n");
                stderr.write(b"Turn on verbose errors with :v\n");
            }
            return true;
        },
    };

    let t2 = time::now();
    // Display the time if the appropriate flag is set
    if config.contains(COMPILE_TIME) {
        let dur = t2 - t1;
        write!(stderr, "Regex compiled in {}ns\n", match dur.num_nanoseconds() {
            Some(x) => x,
            None => dur.num_milliseconds(),
        });
    }

    // Display a prompt using the compiled regex
    prompt(editor, &reg, config)
}

// If this returns false, the program with exit.
// If it returns true, the prompt for a new regex
// will be shown.
fn prompt(editor: &mut Editor<()>, reg: &Regex, config: &mut Config) -> bool {
    let mut stderr = io::stderr();
    let prompt = &format!("Regex({})> ", reg.as_str());

    loop {
        let line = editor.readline(prompt).expect("Failed to read line");
        editor.add_history_entry(&line);

        // Enable menu
        match options_menu(&line, config) {
            Action::Exit => return false,
            Action::Loop => continue,
            Action::ToRegexPrompt => return true,
            // Not a command so test it against the regex
            Action::Continue => {
                // Are we dealing with capture groups?
                if config.contains(CAPTURE_GROUPS) {
                    let caps = match reg.captures(&line){
                        Some(v) => v,
                        None => {
                            write!(stderr, "Failed to match\n");
                            continue;
                        },
                    };
                    write!(stderr, "Captures:\n");
                    for (i, cap) in caps.iter().enumerate() {
                        write!(stderr, "{}: {}\n", i, if let Some(c) = cap {
                            c
                        } else {
                            "None"
                        });
                    }
                } else {
                    if reg.is_match(&line) {
                        write!(stderr, "Matched\n");
                    } else {
                        write!(stderr, "Failed to match\n");
                    }
                }
            },
        }
    }
}

/// Determine and load the history file erroring out
/// upon failure.
///
/// # Notes
/// Failure within this function is non-fatal. It will
/// not panic and only show a warning to the user.
fn with_history_file<F>(mut f: F)
where F: FnMut(&PathBuf),
{
    if cfg!(unix) {
        match env::var("HOME") {
            Ok(x) => {
                let mut path = PathBuf::from(x);
                path.push(".regtest_history");
                f(&path);
            },
            Err(_) => println!("Failed to find history file"),
        }
    } else if cfg!(windows) {
        match env::var("APPDATA") {
            Ok(x) => {
                let mut path = PathBuf::from(x);
                path.push("Roaming");
                path.push("regtest_history");
                f(&path);
            },
            Err(_) => println!("Failed to find history"),
        }
    } else {
        println!("Warning: unknown platform. Unable to \
                  determine location for history file.");
    }
}

fn main() {
    let mut config = Config::default();
    // Configure command line flags
    let matches = App::new("regtest")
        .version(env!("CARGO_PKG_VERSION"))
        .author("Lucas Salibian <lucas.salibian@gmail.com>")
        .about("Test regexes from the command line")
        .arg(Arg::with_name("no-verbose-errors")
             .long("no-verbose-errors")
             .help("Disable verbose errors when the regex fails to compile"))
        .arg(Arg::with_name("capture")
             .short("c")
             .long("capture")
             .help("Enable capture group display after matching test"))
        .arg(Arg::with_name("no-comple-time")
             .long("no-compile-time")
             .help("Disable showing the amount of time it took\
                    to compile the regular expression."))
        .get_matches();

    if matches.is_present("no-verbose-errors") {
        config.remove(VERBOSE_ERRORS);
    }

    if matches.is_present("capture") {
        config.insert(CAPTURE_GROUPS);
    }

    // Initialize the rustline (readline) editor
    let mut editor = Editor::<()>::new();

    with_history_file(|path| {
        editor.load_history(path);
    });

    // Enter the main loop
    loop {
        if !regex_prompt(&mut editor, &mut config) {
            break;
        }
    }

    with_history_file(|path| {
        editor.save_history(path).unwrap();
    });
}

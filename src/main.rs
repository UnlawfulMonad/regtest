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
use std::default::Default;

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

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum Action {
    Continue,
    Loop,
    ToRegexPrompt,
    Exit,
}

fn options_menu(line: &str, config: &mut Config) -> Action {
    let mut stderr = io::stderr();
    // What can you do from here?
    match &line as &str {
        ":q" => Action::Exit,
        ":v" => {
            config.toggle(VERBOSE_ERRORS);
            if config.contains(VERBOSE_ERRORS) {
                write!(stderr, "Verbose errors: on\n");
            } else {
                write!(stderr, "Verbose errors: off\n");
            }
            Action::Loop
        },
        ":t" => {
            config.toggle(COMPILE_TIME);
            if config.contains(COMPILE_TIME) {
                write!(stderr, "Show compile time: on\n");
            } else {
                write!(stderr, "Show compile time: off\n");
            }
            Action::Loop
        },
        ":b" => Action::ToRegexPrompt,
        ":g" => {
            config.toggle(CAPTURE_GROUPS);
            if config.contains(CAPTURE_GROUPS) {
                write!(stderr, "Show capture groups: on\n");
            } else {
                write!(stderr, "Show capture groups: off\n");
            }
            Action::Loop
        },
        ":h" | ":?" => {
            write!(stderr, "{}\n", HELP);
            Action::Loop
        },
        // Continue
        _ => Action::Continue,
    }
}

// Show a prompt saying "n>" requesting
// that a regex be input.
// If this function returns true, the user will
// be prompted to input a regex.
fn regex_prompt(editor: &mut Editor<()>, config: &mut Config) -> bool {
    let mut stderr = io::stderr();

    let line = editor.readline("Input> ").expect("Failed to read line!");
    editor.add_history_entry(&line);

    match options_menu(&line, config) {
        Action::Continue => {},
        Action::ToRegexPrompt | Action::Loop => return true,
        Action::Exit => return false,
    }
 
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
    if config.contains(COMPILE_TIME) {
        let dur = t2 - t1;
        write!(stderr, "Regex compiled in {}ms\n", dur.num_milliseconds());
    }

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

fn main() {
    let mut config = Config::default();
    let matches = App::new("regtest")
        .version("0.1.0")
        .author("Lucas Salibian <lucas.salibian@gmail.com>")
        .about("Test regex from the command line")
        .arg(Arg::with_name("no-verbose-errors")
             .short("v")
             .long("no-verbose-errors")
             .help("Disable verbose errors when the regex fails to compile"))
        .get_matches();

    if matches.is_present("no-verbose-errors") {
        config.remove(VERBOSE_ERRORS);
    }

    let mut editor = Editor::<()>::new();
    loop {
        if !regex_prompt(&mut editor, &mut config) {
            break;
        }
    }
}

#![allow(unused_imports)]
use core::fmt::Error;
use miette::{IntoDiagnostic, Result};
use nom::branch::alt;
use nom::bytes::complete::is_a;
use nom::bytes::complete::is_not;
use nom::bytes::complete::tag;
use nom::bytes::complete::take_until;
use nom::character::complete::multispace0;
use nom::character::complete::not_line_ending;
use nom::character::complete::space1;
use nom::combinator::eof;
use nom::combinator::opt;
use nom::multi::many_till;
use nom::sequence::pair;
use nom::sequence::preceded;
use nom::sequence::terminated;
use nom::IResult;
use nom::Parser;
use std::fs;
use std::fs::copy;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;
use watchexec::{
    action::{Action, Outcome},
    config::{InitConfig, RuntimeConfig},
    handler::PrintDebug,
    Watchexec,
};
use watchexec_signals::Signal;

#[derive(Clone)]
struct Config {
    input_root: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    try_to_set_tmux_title();
    let mut init = InitConfig::default();
    init.on_error(PrintDebug(std::io::stderr()));
    let mut runtime = RuntimeConfig::default();
    runtime.pathset(["/Users/alan/Grimoire"]);
    runtime.action_throttle(Duration::new(0, 100000000));
    let we = Watchexec::new(init, runtime.clone())?;
    runtime.on_action(move |action: Action| async move {
        let mut stop_running = false;
        let mut load_content = false;
        for event in action.events.iter() {
            event.signals().for_each(|sig| match sig {
                Signal::Interrupt => {
                    stop_running = true;
                }
                _ => {}
            });
            if event
                .paths()
                .any(|(p, _)| p.starts_with("/Users/alan/Grimoire"))
            {
                load_content = true;
            }
        }
        if stop_running {
            action.outcome(Outcome::Exit);
        }
        if load_content {
            load_dotfiles_from_grimoire();
        }
        Ok::<(), Error>(())
    });
    let _ = we.reconfigure(runtime);
    let _ = we.main().await.into_diagnostic()?;
    Ok(())
}

fn load_dotfiles_from_grimoire() {
    println!("Loading dotfile");
    let prod = Config {
        input_root: "/Users/alan/Grimoire".to_string(),
    };
    let config = prod.clone();
    let paths = filter_extensions(
        fs::read_dir(&config.input_root)
            .unwrap()
            .into_iter()
            .map(|p| p.expect("here").path())
            .collect::<Vec<PathBuf>>(),
    );
    paths.iter().for_each(|p| {
        let data = fs::read_to_string(p).unwrap();
        match do_the_thing(data.as_str()).unwrap().1 {
            None => {}
            Some((path, content)) => {
                dbg!(&path);
                let _ = fs::write(path, content);
                ()
            }
        }
    });
    println!("Process complete");
}

pub fn do_the_thing(source: &str) -> IResult<&str, Option<(String, String)>> {
    let (source, return_value) = opt(second_level)(source)?;
    Ok((source, return_value))
}

pub fn second_level(source: &str) -> IResult<&str, (String, String)> {
    let (source, _) = take_until("-- startexport:")(source)?;
    let (source, _) = tag("-- startexport:")(source)?;
    let (source, _) = space1(source)?;
    let (source, path) = not_line_ending(source)?;
    let (source, content) = take_until("-- endexport")(source)?;
    Ok((
        source,
        (path.trim().to_string(), content.trim().to_string()),
    ))
}

pub fn filter_extensions(list: Vec<PathBuf>) -> Vec<PathBuf> {
    list.into_iter()
        .filter(|p| match p.extension() {
            Some(ext) => {
                if ext == "org" {
                    true
                } else {
                    false
                }
            }
            None => false,
        })
        .collect()
}

pub fn try_to_set_tmux_title() {
    let args: Vec<&str> = vec!["select-pane", "-T", "dotfile_loader"];
    let _ = Command::new("tmux").args(args).output().unwrap();
}

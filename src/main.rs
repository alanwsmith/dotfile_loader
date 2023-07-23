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
    let mut init = InitConfig::default();
    init.on_error(PrintDebug(std::io::stderr()));
    let mut runtime = RuntimeConfig::default();
    runtime.pathset(["/Users/alan/Desktop/_dottest"]);
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
                .any(|(p, _)| p.starts_with("/Users/alan/Desktop/_dottest"))
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
        input_root: "/Users/alan/Desktop/_dottest".to_string(),
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
        dbg!(&p);
        let data = fs::read_to_string(p).unwrap();
        match do_the_thing(data.as_str()).unwrap().1 {
            None => {}
            Some((path, content)) => {
                let _ = fs::write(path, content);
                ()
            }
        }
    });
    println!("Process complete");
}

pub fn do_the_thing(source: &str) -> IResult<&str, Option<(String, String)>> {
    let (source, _) = opt(take_until("-- startexport:"))(source)?;
    let (source, _) = tag("-- startexport:")(source)?;
    let (source, _) = space1(source)?;
    let (source, path) = not_line_ending(source)?;
    let (source, content) = take_until("-- endexport")(source)?;
    Ok((
        source,
        Some((path.trim().to_string(), content.trim().to_string())),
    ))
}

// pub fn file_id(source: &str) -> IResult<&str, &str> {
//     let (a, _b) = take_until("\n-- attributes")(source)?;
//     let (a, _b) = tag("\n-- attributes")(a)?;
//     let (a, _b) = take_until("-- id: ")(a)?;
//     let (a, _b) = tag("-- id: ")(a)?;
//     let (a, _b) = multispace0(a)?;
//     let (_a, b) = not_line_ending(a)?;
//     Ok(("", b.trim()))
// }

// pub fn output_dir_name<'a>(source: &'a str, id: &'a str) -> IResult<&'a str, String> {
//     let (source, _) = multispace0(source.trim())?;
//     let (source, parts) =
//         many_till(terminated(is_not(" -.'"), alt((is_a(" -.'"), eof))), eof)(source)?;
//     let response = format!(
//         "{}--{}",
//         parts
//             .0
//             .iter()
//             .map(|p| p.to_lowercase())
//             .collect::<Vec<String>>()
//             .join("-"),
//         id
//     );
//     Ok((source, response))
// }

// pub fn filter_status(source: &str) -> IResult<&str, bool> {
//     let (source, check_status_1) = opt(take_until("\n-- attributes"))(source)?;
//     match check_status_1 {
//         Some(_) => {
//             let (source, _) = tag("\n-- attributes")(source)?;
//             let (source, check_status_2) = opt(take_until("-- status: "))(source)?;
//             match check_status_2 {
//                 Some(_) => {
//                     let (source, _) = tag("-- status: ")(source)?;
//                     let (source, b) = not_line_ending(source)?;
//                     match b.trim() {
//                         "published" => Ok((source, true)),
//                         "draft" => Ok((source, true)),
//                         "scratch" => Ok((source, true)),
//                         _ => Ok((source, false)),
//                     }
//                 }
//                 None => Ok((source, false)),
//             }
//         }
//         None => Ok((source, false)),
//     }
// }

// pub fn filter_site<'a>(source: &'a str, site_id: &'a str) -> IResult<&'a str, bool> {
//     let (source, check_site_1) = opt(take_until("\n-- attributes"))(source)?;
//     match check_site_1 {
//         Some(_) => {
//             let (source, _) = tag("\n-- attributes")(source)?;
//             let (source, check_site_2) = opt(take_until("-- site: "))(source)?;
//             match check_site_2 {
//                 Some(_) => {
//                     let (source, _) = tag("-- site: ")(source)?;
//                     let (source, the_id) = not_line_ending(source)?;
//                     if the_id.trim() == site_id {
//                         Ok((source, true))
//                     } else {
//                         Ok((source, false))
//                     }
//                 }
//                 None => Ok((source, false)),
//             }
//         }
//         None => Ok((source, false)),
//     }
// }

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

// pub fn override_path(source: &str) -> IResult<&str, Option<String>> {
//     let (source, _) = pair(take_until("\n-- attributes"), tag("\n-- attributes"))(source)?;
//     let (source, the_path) = opt(preceded(
//         pair(take_until("-- path: "), tag("-- path: ")),
//         not_line_ending.map(|s: &str| s.to_string()),
//     ))(source)?;
//     Ok((source, the_path))
// }

//pub fn valid_nonce(p: PathBuf) -> bool {
//    // TODO: Remove `aws-` to see if there's anytying there that needs
//    // to be removed
//    // Don't do `cloudinary- ` until you've scrubbed it
//    let nonces = vec![
//        "alt- ",
//        "ansible- ",
//        "apis- ",
//        "app- ",
//        "applescript- ",
//        "apps- ",
//        "ascii- ",
//        "audition- ",
//        "automator- ",
//        "awk- ",
//        "bash- ",
//        "bbedit- ",
//        "blender- ",
//        "bookmarks- ",
//        "books- ",
//        "chrome- ",
//        "classnotes- ",
//        "cli- ",
//        "colors- ",
//        "confnotes- ",
//        "css- ",
//        "cuc- ",
//        "d3- ",
//        "daily-links- ",
//        "data- ",
//        "davinci- ",
//        "design- ",
//        "dev- ",
//        "django- ",
//        "docker- ",
//        "drupal- ",
//        "eclipse- ",
//        "emacs- ",
//        "electron- ",
//        "examples- ",
//        "exiftool- ",
//        "ffmpeg- ",
//        "freenas- ",
//        "gatsby- ",
//        "gif- ",
//        "github- ",
//        "grim- ",
//        "grub- ",
//        "hammerspoon-",
//        "heroku- ",
//        "html- ",
//        "htpc- ",
//        "httrack- ",
//        "hugo- ",
//        "iterm2- ",
//        "jekyll- ",
//        "jq- ",
//        "jquery- ",
//        "js- ",
//        "json- ",
//        "keyboard-maestro- ",
//        "keyboards- ",
//        "kindle- ",
//        "launchd- ",
//        "ligthroom- ",
//        "lists- ",
//        "lua- ",
//        "image-magick- ",
//        "minecraft- ",
//        "misc- ",
//        "music- ",
//        "musicbrainz- ",
//        "neo- ",
//        "neoe- ",
//        "neop- ",
//        "netlify- ",
//        "nextjs- ",
//        "nginx- ",
//        "ngrok- ",
//        "node- ",
//        "nokogiri- ",
//        "notes- ",
//        "nvalt- ",
//        "nvim- ",
//        //
//        //
//        "post- ",
//        "site- ",
//        "stream- ",
//        "tools- ",
//    ];
//    match nonces.iter().find(|&&n| {
//        p.file_name()
//            .unwrap()
//            .to_os_string()
//            .into_string()
//            .unwrap()
//            .starts_with(n)
//    }) {
//        Some(_) => true,
//        None => false,
//    }
//}

// #[cfg(test)]
// mod test {
//     use super::*;
//     use std::path::PathBuf;
//     #[test]
//     pub fn filter_extensions_test() {
//         let files = vec![
//             PathBuf::from("/a/b/alfa.org"),
//             PathBuf::from("/a/b/bravo.txt"),
//         ];
//         assert_eq!(
//             vec![PathBuf::from("/a/b/alfa.org")],
//             filter_extensions(files)
//         );
//     }
//     #[test]
//     pub fn filter_status_false() {
//         // allowed statuses are hard coded above
//         let lines = [
//             "",
//             "-- attributes",
//             "-- id: 12341234",
//             "-- status: unpublished",
//         ];
//         assert_eq!(filter_status(lines.join("\n").as_str()).unwrap().1, false);
//     }
//     #[test]
//     pub fn filter_status_true() {
//         let lines = [
//             "",
//             "-- attributes",
//             "-- id: 12341234",
//             "-- status: published",
//         ];
//         assert_eq!(filter_status(lines.join("\n").as_str()).unwrap().1, true);
//     }
//     #[test]
//     pub fn filter_status_with_trailing_space() {
//         let lines = [
//             "",
//             "-- attributes",
//             "-- id: 12341234",
//             "-- status: published ",
//         ];
//         assert_eq!(filter_status(lines.join("\n").as_str()).unwrap().1, true);
//     }
//     #[test]
//     pub fn filter_status_with_no_content() {
//         let lines = ["", "-- attributes", "-- date: 2023-02-03 13:14:15"];
//         assert_eq!(filter_status(lines.join("\n").as_str()).unwrap().1, false);
//     }
//     #[test]
//     pub fn filter_site_test() {
//         let lines = [
//             "",
//             "-- attributes",
//             "-- id: 12341234",
//             "-- site: neoengine",
//             "-- status: published ",
//         ];
//         assert_eq!(
//             filter_site(lines.join("\n").as_str(), "neoengine")
//                 .unwrap()
//                 .1,
//             true
//         );
//     }
//     #[test]
//     pub fn filter_site_test_with_no_attibutes() {
//         let lines = ["this is a file with no attributes"];
//         assert_eq!(
//             filter_site(lines.join("\n").as_str(), "neoengine")
//                 .unwrap()
//                 .1,
//             false
//         );
//     }
//     #[test]
//     pub fn basic_output_dir_name() {
//         let source = PathBuf::from("/some/posts/rust- Basic Path Example.neo");
//         let id = String::from("1234qwer");
//         let expected = Ok(("", "rust-basic-path-example--1234qwer".to_string()));
//         let results = output_dir_name(source.file_stem().unwrap().to_str().unwrap(), id.as_str());
//         assert_eq!(results, expected);
//     }
//     #[test]
//     pub fn dir_with_dashes_that_are_not_followed_by_a_space() {
//         let source = PathBuf::from("alfa-bravo");
//         let id = String::from("9876rewq");
//         let expected = Ok(("", "alfa-bravo--9876rewq".to_string()));
//         let results = output_dir_name(source.file_stem().unwrap().to_str().unwrap(), id.as_str());
//         assert_eq!(results, expected);
//     }
//     #[test]
//     pub fn file_id_basic() {
//         let lines = ["", "-- attributes", "-- id: 1234alfa"].join("\n");
//         assert_eq!(file_id(lines.as_str()).unwrap().1, "1234alfa");
//     }
//     #[test]
//     pub fn file_id_with_trailing_white_space() {
//         let lines = [
//             "",
//             "-- attributes",
//             "-- id: 6789bravo ",
//             "-- status: published",
//             "",
//         ]
//         .join("\n");
//         assert_eq!(file_id(lines.as_str()).unwrap().1, "6789bravo");
//     }
//     #[test]
//     pub fn get_override_path() {
//         let lines = ["", "-- attributes", "-- path: index.neo", ""].join("\n");
//         assert_eq!(
//             override_path(lines.as_str()).unwrap().1,
//             Some("index.neo".to_string())
//         );
//     }
//     #[test]
//     pub fn valid_noce_test() {
//         let name = PathBuf::from("d3- alfa bravo");
//         assert_eq!(true, valid_nonce(name));
//     }
//     #[test]
//     pub fn valid_noce_test_skip() {
//         let name = PathBuf::from("skipthis- charlie delta");
//         assert_eq!(false, valid_nonce(name));
//     }
// }

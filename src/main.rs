//! `kito-zcode`: ZCode, the coding assistant of Z.ai, set up to work
//! through Kito.
//!
//! A plugin of Kito (ADR 0005 there). In a repository that enabled it:
//!
//! - `kito assistant add zcode` (which runs `kito-zcode assistant setup`)
//!   writes what ZCode reads in a project: the rules in `AGENTS.md` and
//!   Kito's tools in `.zcode/config.json`;
//! - `kito assistant remove zcode` takes exactly that out again;
//! - `kito zcode hooks` says whether ZCode is held to the rules on this
//!   machine. ZCode runs no hooks of a project: the ones that hold it to
//!   the rules are a person's own, in `~/.zcode/cli/config.json`, and are
//!   written there only when asked (`kito zcode hooks install`).
//!
//! The hooks are Kito's own commands (`kito hook …`): ZCode says what a
//! tool is about to do in the words Claude Code uses, which Kito reads.

mod setup;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// The repository, as Kito says it to its plugins.
const ENV_REPO: &str = "KITO_REPO";

fn repository() -> Result<PathBuf, String> {
    std::env::var_os(ENV_REPO)
        .filter(|root| !root.is_empty())
        .map(PathBuf::from)
        .filter(|root| root.is_dir())
        .ok_or_else(|| {
            "this is a plugin of Kito: `kito plugin enable zcode` in a repository, then `kito assistant add zcode`"
                .to_owned()
        })
}

fn said(root: &Path, done: Vec<setup::Done>, nothing: &str) {
    if done.is_empty() {
        println!("{nothing}");
    }
    for one in done {
        let path = one.path.strip_prefix(root).unwrap_or(&one.path);
        println!("  {} {}", one.what, path.display());
    }
}

/// What stands between ZCode and the rules on this machine, in a line or two.
fn about_hooks() -> Result<(), String> {
    let path = setup::own_config()?;
    match setup::hooks_there(&path)? {
        (3, true) => println!(
            "ZCode is held to the rules on this machine: Kito's hooks are in {}.",
            path.display()
        ),
        (found, _) => {
            println!("ZCode is NOT held to the rules on this machine: {found} of Kito's 3 hooks are in {}.", path.display());
            println!("Until they are, the rules of AGENTS.md are advice, checked when a lane is integrated.");
            println!("ZCode runs no hooks of a project, only your own: `kito zcode hooks install` writes them there.");
        }
    }
    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let words: Vec<&str> = args.iter().map(String::as_str).collect();
    let done = match words.as_slice() {
        ["assistant", "setup"] => repository().and_then(|root| {
            let done = setup::setup(&root)?;
            println!("ZCode is set up for Kito in this repository:");
            said(&root, done, "  everything was there already");
            println!("`kito` and `kito-mcp` must be on the PATH ZCode runs with.");
            about_hooks()
        }),
        ["assistant", "remove"] => repository().and_then(|root| {
            let done = setup::remove(&root)?;
            println!("ZCode is no longer set up for Kito in this repository:");
            said(&root, done, "  nothing of it was there");
            println!("Your own hooks are left as they are: `kito zcode hooks remove` takes Kito's out.");
            Ok(())
        }),
        ["hooks"] => about_hooks().map(|()| {
            println!("What `install` adds to your configuration:");
            println!("{}", serde_json::to_string_pretty(&setup::hooks_wanted()).unwrap_or_default());
        }),
        ["hooks", "install"] => setup::own_config().and_then(|path| {
            match setup::hooks(&path, true)? {
                Some(done) => println!("{} {}", done.what, done.path.display()),
                None => println!("Kito's hooks were there already ({}).", path.display()),
            }
            println!("They run for every project you open in ZCode, and say nothing where Kito is not.");
            println!("ZCode now runs the hooks of this file (`hooks.enabled`): yours too, if you had some.");
            Ok(())
        }),
        ["hooks", "remove"] => setup::own_config().and_then(|path| {
            match setup::hooks(&path, false)? {
                Some(done) => println!("{} {}", done.what, done.path.display()),
                None => println!("Kito's hooks were not there ({}).", path.display()),
            }
            Ok(())
        }),
        ["--version"] => {
            println!("kito-zcode {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        _ => Err("it is `kito assistant add zcode`, `kito assistant remove zcode`, or `kito zcode hooks [install | \
                  remove]`"
            .to_owned()),
    };
    match done {
        Ok(()) => ExitCode::SUCCESS,
        Err(why) => {
            eprintln!("kito-zcode: {why}");
            ExitCode::from(1)
        }
    }
}

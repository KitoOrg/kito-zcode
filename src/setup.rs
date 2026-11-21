//! What ZCode reads, written for Kito and taken out again.
//!
//! In a project:
//! - `AGENTS.md`: the rules, between two marks, after whatever the
//!   project says there itself;
//! - `.zcode/config.json`: Kito's tools (`mcp.servers.kito`).
//!
//! For a person, and only when they ask (`kito zcode hooks install`):
//! - `~/.zcode/cli/config.json`: what ZCode runs before an edit, after a
//!   shell command and when a session starts. ZCode runs no hooks of a
//!   project, so they cannot be written there.
//!
//! What is written is only added to what is there, and taking it out
//! leaves the rest.

use serde_json::{json, Map, Value};
use std::path::{Path, PathBuf};

/// One thing that was done to a file.
pub struct Done {
    pub what: &'static str,
    pub path: PathBuf,
}

/// The rules: Kito's own text.
pub const RULES: &str = include_str!("../rules.md");
/// How the marks begin, whichever of Kito's plugins wrote them:
/// `AGENTS.md` is read by more than one assistant.
const MARK: &str = "<!-- kito: begin";
const BEGIN: &str = "<!-- kito: begin (written by Kito for the assistants that read this file; edit outside these marks) -->";
const END: &str = "<!-- kito: end -->";
/// The MCP server, as ZCode names it and as it is started.
const SERVER: &str = "kito";
const SERVER_PROGRAM: &str = "kito-mcp";
/// How long a hook may take, in milliseconds, before ZCode goes on without it.
const HOOK_TIMEOUT_MS: u64 = 30_000;
/// The hooks: the event, the tools it is for, the command.
const HOOKS: [(&str, Option<&str>, &str); 3] = [
    ("PreToolUse", Some("Write|Edit"), "kito hook before-edit"),
    ("PostToolUse", Some("Bash"), "kito hook after-shell"),
    ("SessionStart", None, "kito hook session-start"),
];

fn read(path: &Path) -> Result<Option<String>, String> {
    match std::fs::read_to_string(path) {
        Ok(text) => Ok(Some(text)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(format!("{}: {e}", path.display())),
    }
}

/// Writes `text` to `path`, or removes the file when nothing is left to
/// say in it. `None`: it was that way already.
fn write(path: &Path, was: Option<&str>, text: &str) -> Result<Option<Done>, String> {
    let said = |e: std::io::Error| format!("{}: {e}", path.display());
    if was == Some(text) || (was.is_none() && text.is_empty()) {
        return Ok(None);
    }
    if text.trim().is_empty() {
        std::fs::remove_file(path).map_err(said)?;
        // Its directory with it, when it was the last file of a project's `.zcode`.
        if let Some(dir) = path.parent().filter(|dir| dir.ends_with(".zcode")) {
            let _ = std::fs::remove_dir(dir);
        }
        return Ok(Some(Done {
            what: "removed",
            path: path.to_owned(),
        }));
    }
    std::fs::create_dir_all(path.parent().unwrap_or(path)).map_err(said)?;
    std::fs::write(path, text).map_err(said)?;
    let what = if was.is_some() { "changed" } else { "wrote" };
    Ok(Some(Done {
        what,
        path: path.to_owned(),
    }))
}

/// `text` without the rules Kito put between its marks.
fn without_rules(text: &str) -> String {
    let (Some(from), Some(to)) = (text.find(MARK), text.find(END)) else {
        return text.to_owned();
    };
    if to < from {
        return text.to_owned();
    }
    let (before, after) = (text[..from].trim_end(), text[to + END.len()..].trim_start());
    match (before.is_empty(), after.is_empty()) {
        (true, _) => after.to_owned(),
        (false, true) => format!("{before}\n"),
        (false, false) => format!("{before}\n\n{after}"),
    }
}

/// `true` if another assistant that reads `AGENTS.md` is set up for Kito
/// here: the rules are its too.
fn shared(root: &Path) -> bool {
    let names_kito = |file: &str| {
        read(&root.join(file))
            .ok()
            .flatten()
            .is_some_and(|text| text.contains("kito"))
    };
    names_kito(".codex/config.toml") || names_kito(".codex/hooks.json")
}

fn rules(root: &Path, wanted: bool) -> Result<Option<Done>, String> {
    let path = root.join("AGENTS.md");
    let was = read(&path)?;
    let now = was.as_deref().unwrap_or_default();
    if !wanted && shared(root) {
        return Ok(None);
    }
    let rest = without_rules(now);
    let text = match wanted {
        false => rest,
        // Rules another of Kito's plugins wrote are the same rules.
        true if now.contains(MARK) && now.contains(RULES) => return Ok(None),
        true if rest.trim().is_empty() => format!("{BEGIN}\n{RULES}{END}\n"),
        true => format!("{}\n\n{BEGIN}\n{RULES}{END}\n", rest.trim_end()),
    };
    write(&path, was.as_deref(), &text)
}

/// Reads a file of settings: an object, or nothing yet.
fn settings_of(path: &Path, by_hand: &str) -> Result<(Option<String>, Map<String, Value>), String> {
    let was = read(path)?;
    let all = match was.as_deref().filter(|text| !text.trim().is_empty()) {
        Some(text) => serde_json::from_str::<Value>(text)
            .map_err(|e| format!("{} is not JSON ({e}): {by_hand}", path.display()))?,
        None => json!({}),
    };
    match all {
        Value::Object(all) => Ok((was, all)),
        _ => Err(format!("{} is not an object: {by_hand}", path.display())),
    }
}

fn written(
    path: &Path,
    was: Option<String>,
    all: Map<String, Value>,
) -> Result<Option<Done>, String> {
    let text = match all.is_empty() {
        true => String::new(),
        false => {
            serde_json::to_string_pretty(&Value::Object(all)).map_err(|e| e.to_string())? + "\n"
        }
    };
    // The same settings written another way are the same settings.
    let same = was
        .as_deref()
        .and_then(|was| serde_json::from_str::<Value>(was).ok())
        == serde_json::from_str::<Value>(&text).ok();
    if same && was.is_some() && !text.is_empty() {
        return Ok(None);
    }
    write(path, was.as_deref(), &text)
}

fn server(path: &Path, wanted: bool) -> Result<Option<Done>, String> {
    let by_hand = format!(
        "add \"{SERVER}\": {{ \"command\": \"{SERVER_PROGRAM}\" }} to its mcp.servers by hand"
    );
    let (was, mut all) = settings_of(path, &by_hand)?;
    let wrong = || {
        format!(
            "{}: its mcp.servers is not an object: {by_hand}",
            path.display()
        )
    };
    if wanted {
        let mcp = all
            .entry("mcp")
            .or_insert_with(|| json!({}))
            .as_object_mut()
            .ok_or_else(wrong)?;
        let servers = mcp
            .entry("servers")
            .or_insert_with(|| json!({}))
            .as_object_mut()
            .ok_or_else(wrong)?;
        // One that is there is the project's own business, whatever it runs.
        if !servers.contains_key(SERVER) {
            servers.insert(
                SERVER.to_owned(),
                json!({ "command": SERVER_PROGRAM, "args": [], "env": {} }),
            );
        }
    } else if let Some(mcp) = all.get_mut("mcp").and_then(Value::as_object_mut) {
        if let Some(servers) = mcp.get_mut("servers").and_then(Value::as_object_mut) {
            if servers
                .get(SERVER)
                .is_some_and(|server| server["command"] == SERVER_PROGRAM)
            {
                servers.remove(SERVER);
            }
            if servers.is_empty() {
                mcp.remove("servers");
            }
        }
        if mcp.is_empty() {
            all.remove("mcp");
        }
    }
    written(path, was, all)
}

/// Writes what ZCode reads of a project to work through Kito in it.
pub fn setup(root: &Path) -> Result<Vec<Done>, String> {
    let done = [
        rules(root, true)?,
        server(&root.join(".zcode").join("config.json"), true)?,
    ];
    Ok(done.into_iter().flatten().collect())
}

/// Takes out what `setup` wrote, and nothing else.
pub fn remove(root: &Path) -> Result<Vec<Done>, String> {
    let done = [
        server(&root.join(".zcode").join("config.json"), false)?,
        rules(root, false)?,
    ];
    Ok(done.into_iter().flatten().collect())
}

// ------------------------------------------------------------- the hooks

/// The person's own configuration of ZCode: the only place it runs hooks from.
pub fn own_config() -> Result<PathBuf, String> {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .filter(|home| !home.is_empty())
        .ok_or("where your own files are is not known (no HOME)")?;
    Ok(PathBuf::from(home)
        .join(".zcode")
        .join("cli")
        .join("config.json"))
}

/// The hooks, as ZCode's configuration has them.
pub fn hooks_wanted() -> Value {
    let mut events = Map::new();
    for (event, matcher, command) in HOOKS {
        let mut group = Map::new();
        if let Some(matcher) = matcher {
            group.insert("matcher".to_owned(), matcher.into());
        }
        let hook = json!({ "type": "command", "command": command, "enabled": true, "timeoutMs": HOOK_TIMEOUT_MS });
        group.insert("hooks".to_owned(), json!([hook]));
        events.insert(event.to_owned(), json!([group]));
    }
    json!({ "hooks": { "enabled": true, "events": events } })
}

/// `true` if a group of hooks runs `command`.
fn runs(group: &Value, command: &str) -> bool {
    group["hooks"]
        .as_array()
        .is_some_and(|hooks| hooks.iter().any(|hook| hook["command"] == command))
}

/// How many of the three hooks a configuration has, and whether ZCode
/// runs hooks at all there.
pub fn hooks_there(path: &Path) -> Result<(usize, bool), String> {
    let (_, all) = settings_of(path, "look at it by hand")?;
    let hooks = all.get("hooks").cloned().unwrap_or(Value::Null);
    let has = |(event, _, command): &(&str, Option<&str>, &str)| {
        hooks["events"][event]
            .as_array()
            .is_some_and(|groups| groups.iter().any(|group| runs(group, command)))
    };
    Ok((
        HOOKS.iter().filter(|hook| has(hook)).count(),
        hooks["enabled"] == true,
    ))
}

/// Adds the hooks to a person's configuration, or takes them out.
pub fn hooks(path: &Path, wanted: bool) -> Result<Option<Done>, String> {
    let by_hand = "add the hooks to it by hand (`kito zcode hooks` prints them)";
    let (was, mut all) = settings_of(path, by_hand)?;
    let wrong = || {
        format!(
            "{} is not written as ZCode writes hooks: {by_hand}",
            path.display()
        )
    };
    if wanted {
        let hooks = all
            .entry("hooks")
            .or_insert_with(|| json!({}))
            .as_object_mut()
            .ok_or_else(wrong)?;
        // Without this ZCode runs none of them.
        hooks.insert("enabled".to_owned(), true.into());
        let events = hooks
            .entry("events")
            .or_insert_with(|| json!({}))
            .as_object_mut()
            .ok_or_else(wrong)?;
        let ours = hooks_wanted();
        for (event, _, command) in HOOKS {
            let groups = events
                .entry(event)
                .or_insert_with(|| json!([]))
                .as_array_mut()
                .ok_or_else(wrong)?;
            if !groups.iter().any(|group| runs(group, command)) {
                groups.push(ours["hooks"]["events"][event][0].clone());
            }
        }
    } else if let Some(hooks) = all.get_mut("hooks").and_then(Value::as_object_mut) {
        if let Some(events) = hooks.get_mut("events").and_then(Value::as_object_mut) {
            for (event, _, command) in HOOKS {
                let Some(groups) = events.get_mut(event).and_then(Value::as_array_mut) else {
                    continue;
                };
                for group in groups.iter_mut() {
                    if let Some(hooks) = group.get_mut("hooks").and_then(Value::as_array_mut) {
                        hooks.retain(|hook| hook["command"] != command);
                    }
                }
                groups.retain(|group| {
                    group["hooks"]
                        .as_array()
                        .is_none_or(|hooks| !hooks.is_empty())
                });
                if groups.is_empty() {
                    events.remove(event);
                }
            }
            if events.is_empty() {
                hooks.remove("events");
            }
        }
        // Whether ZCode runs hooks is the person's to say: left as it is
        // while anything else is there, and gone with the last of it.
        if hooks.keys().all(|key| key == "enabled") {
            all.remove("hooks");
        }
    }
    written(path, was, all)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_rules_go_in_and_out_between_their_marks() {
        let own = "# Shop\n\nRun `npm test` before you say it works.\n";
        let with = format!("{}\n\n{BEGIN}\n{RULES}{END}\n", own.trim_end());
        assert_eq!(without_rules(&with), own);
        assert_eq!(without_rules(&format!("{BEGIN}\n{RULES}{END}\n")), "");
        // Marks another of Kito's plugins wrote are Kito's marks.
        let codex = format!("<!-- kito: begin (written by `kito assistant add codex`) -->\nold\n{END}\n\n## After\n");
        assert_eq!(without_rules(&codex), "## After\n");
        assert_eq!(without_rules(own), own);
    }
}

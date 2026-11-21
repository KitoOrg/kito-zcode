//! `kito-zcode`: what it writes for ZCode in a project, and in a
//! person's own configuration when asked.
//!
//! The last test runs it under Kito itself, and needs `kito` and `kitod`
//! in `KITO_BIN` or in `../kito/target/debug`.

use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Runs the plugin for a repository, with a home of the test's own.
fn plugin(root: &Path, home: &Path, args: &[&str]) -> (bool, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_kito-zcode"))
        .args(args)
        .env("KITO_REPO", root)
        .env("HOME", home)
        .env("USERPROFILE", home)
        .output()
        .unwrap();
    let said = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    (out.status.success(), said.replace('\\', "/"))
}

fn text(path: PathBuf) -> String {
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn json_of(path: PathBuf) -> Value {
    serde_json::from_str(&text(path)).unwrap()
}

#[test]
fn it_writes_what_zcode_reads_of_a_project_and_takes_it_out_again() {
    let tmp = tempfile::tempdir().unwrap();
    let (root, home) = (tmp.path().join("shop"), tmp.path().join("home"));
    std::fs::create_dir_all(&root).unwrap();
    let (ok, said) = plugin(&root, &home, &["assistant", "setup"]);
    assert!(ok, "{said}");
    assert!(
        said.contains("wrote AGENTS.md") && said.contains("wrote .zcode/config.json"),
        "{said}"
    );
    // What it cannot write in a project is said, not kept quiet.
    assert!(
        said.contains("NOT held to the rules on this machine")
            && said.contains("kito zcode hooks install"),
        "{said}"
    );
    assert!(
        !home.exists(),
        "nothing of the person's own is written unasked"
    );

    let rules = text(root.join("AGENTS.md"));
    assert!(
        rules.starts_with("<!-- kito: begin") && rules.contains("## Version control: Kito"),
        "{rules}"
    );
    assert!(
        rules.contains("Work in a lane.") && rules.trim_end().ends_with("<!-- kito: end -->"),
        "{rules}"
    );
    assert_eq!(
        json_of(root.join(".zcode/config.json")),
        json!({ "mcp": { "servers": { "kito": { "command": "kito-mcp", "args": [], "env": {} } } } })
    );

    let (ok, said) = plugin(&root, &home, &["assistant", "setup"]);
    assert!(
        ok && said.contains("everything was there already"),
        "{said}"
    );
    let (ok, said) = plugin(&root, &home, &["assistant", "remove"]);
    assert!(ok && said.contains("removed AGENTS.md"), "{said}");
    assert!(!root.join("AGENTS.md").exists() && !root.join(".zcode").exists());
    let (ok, said) = plugin(&root, &home, &["assistant", "remove"]);
    assert!(ok && said.contains("nothing of it was there"), "{said}");
}

#[test]
fn what_the_project_says_itself_is_left_as_it_is() {
    let tmp = tempfile::tempdir().unwrap();
    let (root, home) = (tmp.path().join("shop"), tmp.path().join("home"));
    let own_rules = "# Shop\n\nRun `npm test` before you say it works.\n";
    let own_config = json!({
        "model": "glm-5",
        "mcp": { "servers": { "memory": { "command": "npx", "args": ["-y", "memory"], "env": {} } } },
    });
    std::fs::create_dir_all(root.join(".zcode")).unwrap();
    std::fs::write(root.join("AGENTS.md"), own_rules).unwrap();
    std::fs::write(root.join(".zcode/config.json"), own_config.to_string()).unwrap();

    let (ok, said) = plugin(&root, &home, &["assistant", "setup"]);
    assert!(ok && said.contains("changed AGENTS.md"), "{said}");
    assert!(text(root.join("AGENTS.md")).starts_with(own_rules));
    let config = json_of(root.join(".zcode/config.json"));
    assert_eq!(
        (
            config["model"].as_str(),
            config["mcp"]["servers"]["memory"]["command"].as_str()
        ),
        (Some("glm-5"), Some("npx"))
    );
    assert_eq!(config["mcp"]["servers"]["kito"]["command"], "kito-mcp");

    let (ok, said) = plugin(&root, &home, &["assistant", "remove"]);
    assert!(ok, "{said}");
    assert_eq!(text(root.join("AGENTS.md")), own_rules);
    assert_eq!(json_of(root.join(".zcode/config.json")), own_config);

    // `AGENTS.md` is Codex's too: rules another of Kito's plugins needs stay.
    plugin(&root, &home, &["assistant", "setup"]);
    std::fs::create_dir_all(root.join(".codex")).unwrap();
    std::fs::write(
        root.join(".codex/config.toml"),
        "[mcp_servers.kito]\ncommand = \"kito-mcp\"\n",
    )
    .unwrap();
    let (ok, said) = plugin(&root, &home, &["assistant", "remove"]);
    assert!(ok && !said.contains("AGENTS.md"), "{said}");
    assert!(text(root.join("AGENTS.md")).contains("## Version control: Kito"));

    // What cannot be read is not touched, and said.
    std::fs::write(root.join(".zcode/config.json"), "not json").unwrap();
    let (ok, said) = plugin(&root, &home, &["assistant", "setup"]);
    assert!(
        !ok && said.contains("is not JSON") && said.contains("by hand"),
        "{said}"
    );
}

#[test]
fn the_hooks_are_a_person_s_own_and_written_only_when_asked() {
    let tmp = tempfile::tempdir().unwrap();
    let (root, home) = (tmp.path().join("shop"), tmp.path().join("home"));
    std::fs::create_dir_all(&root).unwrap();
    let file = home.join(".zcode/cli/config.json");

    // Asked what they are: said, and nothing written.
    let (ok, said) = plugin(&root, &home, &["hooks"]);
    assert!(
        ok && said.contains("NOT held to the rules") && said.contains("\"kito hook before-edit\""),
        "{said}"
    );
    assert!(!file.exists());

    // The person had a configuration, with a hook of their own that ZCode did not run.
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    let own = json!({
        "theme": "dark",
        "hooks": { "enabled": false, "events": { "PreToolUse": [
            { "matcher": "Bash", "hooks": [{ "type": "command", "command": "./mine.sh" }] },
        ]}},
    });
    std::fs::write(&file, own.to_string()).unwrap();
    let (ok, said) = plugin(&root, &home, &["hooks", "install"]);
    assert!(
        ok && said.contains("changed") && said.contains("yours too"),
        "{said}"
    );
    let now = json_of(file.clone());
    assert_eq!(
        (now["theme"].as_str(), now["hooks"]["enabled"].as_bool()),
        (Some("dark"), Some(true))
    );
    let before = now["hooks"]["events"]["PreToolUse"].as_array().unwrap();
    assert_eq!(before[0]["hooks"][0]["command"], "./mine.sh");
    assert_eq!(
        before[1],
        json!({ "matcher": "Write|Edit", "hooks": [
            { "type": "command", "command": "kito hook before-edit", "enabled": true, "timeoutMs": 30000 },
        ]})
    );
    assert_eq!(
        now["hooks"]["events"]["PostToolUse"][0]["hooks"][0]["command"],
        "kito hook after-shell"
    );
    assert_eq!(
        now["hooks"]["events"]["SessionStart"][0]["hooks"][0]["command"],
        "kito hook session-start"
    );

    let (ok, said) = plugin(&root, &home, &["hooks", "install"]);
    assert!(ok && said.contains("were there already"), "{said}");
    assert!(plugin(&root, &home, &["hooks"])
        .1
        .contains("ZCode is held to the rules on this machine"));
    assert!(plugin(&root, &home, &["assistant", "setup"])
        .1
        .contains("is held to the rules"));

    // Taken out: the person's own hook stays, and so does what they said about running hooks.
    let (ok, said) = plugin(&root, &home, &["hooks", "remove"]);
    assert!(ok && said.contains("changed"), "{said}");
    let now = json_of(file.clone());
    assert_eq!(now["hooks"]["events"], own["hooks"]["events"]);
    assert_eq!(now["theme"], "dark");
    assert!(plugin(&root, &home, &["hooks", "remove"])
        .1
        .contains("were not there"));

    // With nothing else in it, the file goes with the hooks.
    std::fs::remove_file(&file).unwrap();
    plugin(&root, &home, &["hooks", "install"]);
    plugin(&root, &home, &["hooks", "remove"]);
    assert!(!file.exists());
}

fn kito_bin() -> PathBuf {
    let dir = std::env::var_os("KITO_BIN")
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../kito/target/debug"));
    let kito = dir.join(format!("kito{}", std::env::consts::EXE_SUFFIX));
    assert!(
        kito.is_file(),
        "{} is not there: this test runs kito-zcode under Kito. Build it (`cargo build -p kito-cli -p kitod` in \
         the kito repository) or set KITO_BIN to where `kito` and `kitod` are",
        kito.display()
    );
    dir.canonicalize().unwrap()
}

#[test]
fn kito_sets_zcode_up_and_its_hook_answers_what_zcode_says() {
    let bin = kito_bin();
    let tmp = tempfile::tempdir().unwrap();
    let base = tmp.path().canonicalize().unwrap();
    let (root, config, home) = (base.join("shop"), base.join("config"), base.join("home"));
    std::fs::create_dir_all(root.join("src")).unwrap();
    let kito = |cwd: &Path, args: &[&str], input: Option<&Value>| {
        use std::io::Write;
        let mut child = Command::new(bin.join(format!("kito{}", std::env::consts::EXE_SUFFIX)))
            .args(args)
            .current_dir(cwd)
            .env(
                "KITOD_BIN",
                bin.join(format!("kitod{}", std::env::consts::EXE_SUFFIX)),
            )
            .env("KITO_NO_CLAUDE", "1")
            .env("KITO_AUTHOR", "ada")
            .env("KITO_CONFIG_DIR", &config)
            .env("HOME", &home)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        let mut stdin = child.stdin.take().unwrap();
        if let Some(input) = input {
            stdin.write_all(format!("{input}\n").as_bytes()).unwrap();
        }
        drop(stdin);
        let out = child.wait_with_output().unwrap();
        let (said, errors) = (
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr),
        );
        assert!(out.status.success(), "kito {args:?}: {said}{errors}");
        said.into_owned()
    };
    kito(&root, &["init"], None);
    std::fs::write(root.join(".gitignore"), ".kito/\n").unwrap();
    std::fs::write(root.join("src/shop.ts"), "export const price = 1;\n").unwrap();
    kito(&root, &["checkpoint", "-m", "first version"], None);

    let source = base.join("plugin");
    std::fs::create_dir_all(&source).unwrap();
    let program = Path::new(env!("CARGO_BIN_EXE_kito-zcode"));
    std::fs::copy(program, source.join(program.file_name().unwrap())).unwrap();
    std::fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("kito-plugin.toml"),
        source.join("kito-plugin.toml"),
    )
    .unwrap();
    kito(
        &root,
        &["plugin", "add", "--path", source.to_str().unwrap()],
        None,
    );
    kito(&root, &["plugin", "enable", "zcode"], None);
    assert!(kito(&root, &["assistant", "list"], None).contains("zcode  (plugin zcode"));
    let set_up = kito(&root, &["assistant", "add", "zcode"], None);
    assert!(
        set_up.contains("ZCode is set up for Kito") && set_up.contains("NOT held to the rules"),
        "{set_up}"
    );
    assert!(root.join(".zcode/config.json").is_file());

    // Its command is Kito's, and writes the hooks where ZCode runs them from.
    let installed = kito(&root, &["zcode", "hooks", "install"], None);
    assert!(installed.contains("wrote"), "{installed}");
    assert!(home.join(".zcode/cli/config.json").is_file());

    // The hook is Kito's own command, given what ZCode's documentation
    // says ZCode sends before it writes a file: one line of JSON.
    kito(
        &root,
        &[
            "lane",
            "open",
            "alice",
            "--intent",
            "a cheaper price",
            "--scope",
            "src/shop.ts",
        ],
        None,
    );
    let work = root.join(".kito/lanes/1/work");
    let asked = |cwd: &Path, file: &str| -> Value {
        let input = json!({
            "session_id": "s", "transcript_path": "t", "cwd": cwd, "permission_mode": "default",
            "hook_event_name": "PreToolUse", "tool_name": "Write", "tool_use_id": "u",
            "tool_input": { "file_path": file, "content": "export const price = 0.5;\n" },
        });
        let said = kito(cwd, &["hook", "before-edit"], Some(&input));
        if said.trim().is_empty() {
            Value::Null
        } else {
            serde_json::from_str(said.trim()).unwrap()
        }
    };
    let refused = asked(&root, "src/shop.ts");
    assert_eq!(
        refused["hookSpecificOutput"]["permissionDecision"], "deny",
        "{refused}"
    );
    assert!(asked(&work, "src/shop.ts").is_null());
    assert_eq!(
        asked(&work, "README.md")["hookSpecificOutput"]["permissionDecision"],
        "deny"
    );

    let removed = kito(&root, &["assistant", "remove", "zcode"], None);
    assert!(removed.contains("removed AGENTS.md"), "{removed}");
    kito(&root, &["daemon", "stop"], None);
}

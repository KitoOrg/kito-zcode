# kito-zcode

[ZCode](https://zcode.z.ai), the coding assistant of Z.ai, set up to work
through [Kito](https://github.com/KitoOrg/kito). A plugin of Kito.

```console
$ cd your-repository
$ kito assistant add zcode
Set up for ZCode:
  enabled the plugin zcode here: it may read
  ZCode is set up for Kito in this repository:
    wrote AGENTS.md
    wrote .zcode/config.json
    wrote ~/.zcode/cli/config.json: Kito's hooks, for every project you open in ZCode
```

The same command as for every other assistant (Kito 0.26). It installs
the plugin when it is not there, enables it here, and runs it.
`kito assistant remove zcode` takes out what was written in the project;
`kito zcode hooks remove`, what was written for you.

## What it writes in a project

- **`AGENTS.md`**: the rules of a project kept by Kito (work in a lane,
  change what the lane declared, know why code is the way it is), between
  two marks, after whatever the project says there itself. Codex reads
  the same file: the rules are written once, and stay while either of
  the two is set up.
- **`.zcode/config.json`**: Kito's tools, `mcp.servers.kito` running
  `kito-mcp`. ZCode connects a project's servers when a session starts.

What is there already stays. A server called `kito` that is not Kito's
is neither replaced nor taken out.

## The hooks are yours, not the project's

ZCode runs no hooks of a project: only those of your own configuration,
`~/.zcode/cli/config.json`. So what holds ZCode to the rules cannot come
with the repository: `kito assistant add zcode` writes them there, and
says so.

- `kito zcode hooks` says whether Kito's hooks are there, and prints them.
- `kito zcode hooks install` adds them: `kito hook before-edit` before
  `Write` and `Edit`, `kito hook after-shell` after `Bash`, `kito hook
  session-start` when a session starts. It turns on `hooks.enabled`,
  without which ZCode runs none: hooks of your own that were there run
  from then on too.
- `kito zcode hooks remove` takes Kito's out and leaves yours.

They run for every project you open in ZCode, and say nothing where Kito
is not. Until they are there, the rules of `AGENTS.md` are advice, and
what holds ZCode to them is Kito itself when a lane is integrated.

`kito` and `kito-mcp` must be on the PATH ZCode runs with.

## What is not tried

Nothing here has been run under ZCode: it is written from its
documentation (3.18 was current). The tests give Kito's hook what that
documentation says ZCode sends before it writes a file, and run the
plugin under Kito itself. Not confirmed: what ZCode sends for `Edit`
(taken to be what Claude Code sends, as the rest is), and that it asks
nothing before it uses the tools of a project's server.

## Development

```console
$ cargo test
```

The last test wants `kito` and `kitod` in `KITO_BIN` or in
`../kito/target/debug`. `rules.md` is Kito's own text, copied from it.

## License

MIT or Apache-2.0, at your option.

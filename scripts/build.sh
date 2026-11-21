#!/usr/bin/env bash
# Builds the plugin and leaves it ready to install in `plugin/`:
#
#   scripts/build.sh            # a debug build
#   scripts/build.sh --dist     # the build of a release
#   kito plugin add --path plugin
set -euo pipefail
cd "$(dirname "$0")/.."

profile=dev
dir=debug
if [ "${1:-}" = "--dist" ]; then
  profile=dist
  dir=dist
fi
cargo build --locked --profile "$profile"

suffix=""
case "$(uname -s)" in MINGW* | MSYS* | CYGWIN*) suffix=".exe" ;; esac
rm -rf plugin
mkdir plugin
cp "target/$dir/kito-zcode$suffix" kito-plugin.toml plugin/
echo "The plugin is in plugin/: kito plugin add --path $(pwd)/plugin"

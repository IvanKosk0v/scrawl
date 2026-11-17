#!/usr/bin/env bash
# PostToolUse hook: format edited .rs files. Costs zero tokens, never blocks.
f=$(python3 -c 'import sys,json;print(json.load(sys.stdin).get("tool_input",{}).get("file_path",""))' 2>/dev/null)
case "$f" in
  *.rs) rustfmt --edition 2021 "$f" >/dev/null 2>&1 ;;
esac
exit 0

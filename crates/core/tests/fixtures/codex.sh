#!/bin/sh
# A fake `codex`, given `--record <dir>` as the provider's own arguments.
schema= out= skip= prev=
for a in "$@"; do
  case "$prev" in
    --record) out=$a ;;
    --output-schema) schema=$a ;;
  esac
  [ "$a" = --skip-git-repo-check ] && skip=1
  prev=$a
done
printf '%s\n' "$@" > "$out/args"
cat > "$out/stdin"
pwd > "$out/cwd"
ls -A > "$out/ls"
(stat -c %a . 2>/dev/null || stat -f %Lp .) > "$out/mode"
(stat -c %a "$schema" 2>/dev/null || stat -f %Lp "$schema") > "$out/schema-mode"
cat "$schema" > "$out/schema"
touch left-behind
[ -f "$out/hang" ] && exec sleep 30
if [ -z "$skip" ]; then
  echo 'Not inside a trusted directory and --skip-git-repo-check was not specified.' >&2
  exit 1
fi
echo '{"said":true}'

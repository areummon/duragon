#!/bin/sh
# A fake ACP agent: `acp-agent.sh <dir>`. Sends each line of <dir>/asks during the prompt, keeps the replies.
out=$1
read -r first; echo "$first" > "$1/initialize"
echo '{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":1}}'
read -r line
echo "$line" > "$out/session-new"
pwd > "$out/cwd"
ls -A > "$out/ls"
(stat -c %a . 2>/dev/null || stat -f %Lp .) > "$out/mode"
[ -f .agents/hooks.json ] && cp .agents/hooks.json "$out/hooks"
[ -f "$out/linger" ] && sleep 30 > /dev/null &
echo '{"jsonrpc":"2.0","id":2,"result":{"sessionId":"s1"}}'
read -r _
touch "$out/left-behind" left-behind
[ -f "$out/hang" ] && exec sleep 30
while read -r ask <&3; do
  echo "$ask"
  read -r reply
  echo "$reply" >> "$out/replies"
done 3< "$out/asks"
echo '{"jsonrpc":"2.0","method":"session/update","params":{"sessionId":"s1","update":{"sessionUpdate":"agent_message_chunk","content":{"type":"text","text":"{\"said\":true}"}}}}'
echo '{"jsonrpc":"2.0","id":3,"result":{"stopReason":"end_turn"}}'

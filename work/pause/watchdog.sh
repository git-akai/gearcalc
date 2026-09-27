#!/usr/bin/env bash
# Memory guard: kill any process over 5 GB resident; when the machine has under
# 3 GB available, kill the largest process that is not Claude Code, nix-daemon,
# node or this guard. Every kill is logged.
LOG=/home/user/.cache/gearcalc-work/watchdog.log
while true; do
  ps -eo pid=,rss=,comm= | awk '$2 > 5*1024*1024' | grep -v -E ' (claude|nix-daemon|watchdog.sh)$' | while read pid rss comm; do
    kill -9 "$pid" 2>/dev/null && echo "$(date +%T) killed $pid $comm rss=$((rss/1024))MB (over 5 GB)" >> $LOG
  done
  avail=$(awk '/MemAvailable/ {print int($2/1024)}' /proc/meminfo)
  if [ "$avail" -lt 3072 ]; then
    read pid rss comm < <(ps -eo pid=,rss=,comm= --sort=-rss | grep -v -E ' (claude|nix-daemon|watchdog.sh|node)$' | head -n 1)
    kill -9 "$pid" 2>/dev/null && echo "$(date +%T) killed $pid $comm rss=$((rss/1024))MB (machine at ${avail} MB available)" >> $LOG
  fi
  sleep 0.3
done

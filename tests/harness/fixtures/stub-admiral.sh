#!/bin/sh
# A stub runtime for a two-node workflow: plan -> build -> done. It keeps the current node in
# .stub-state and logs every advance (arguments and result file) to .stub-log, in the working directory.
state=.stub-state
node=plan
[ -f "$state" ] && node=$(cat "$state")

show() {
  case "$1" in
    plan) printf '{"node":"plan","kind":"agent","task":"write the plan"}\n' ;;
    build) printf '{"node":"build","kind":"agent","task":"build the plan"}\n' ;;
    *) printf '{"node":"done","kind":"end"}\n' ;;
  esac
}

case "$1" in
  next)
    show "$node"
    ;;
  advance)
    target=$2
    if [ "$target" != "$node" ]; then
      echo "stub: advance for $target but the current node is $node" >&2
      exit 1
    fi
    printf 'advance %s\n' "$*" >> .stub-log
    while [ $# -gt 0 ]; do
      if [ "$1" = "--result" ]; then
        cat "$2" >> .stub-log
        printf '\n' >> .stub-log
      fi
      shift
    done
    case "$node" in
      plan) node=build ;;
      *) node=done ;;
    esac
    printf '%s' "$node" > "$state"
    show "$node"
    ;;
  *)
    echo "stub: unknown command $1" >&2
    exit 2
    ;;
esac

if [[ $- == *i* && -z ${__synt_integration-} ]]; then
  __synt_integration=1
  __synt_running=0
  __synt_ready=0
  __synt_history=

  __synt_precmd() {
    local code=$?
    if [[ $__synt_running == 1 ]]; then
      printf '\e]133;D;%s\a' "$code"
      __synt_running=0
    fi
    __synt_history=$(HISTTIMEFORMAT= builtin history 1)
    __synt_ready=1
    printf '\e]133;A\a'
  }

  __synt_in_prompt_command() {
    local entry part parts
    for entry in ${PROMPT_COMMAND[@]+"${PROMPT_COMMAND[@]}"}; do
      IFS=$';\n' read -r -d '' -a parts <<< "$entry"
      for part in "${parts[@]}"; do
        part=${part#"${part%%[![:space:]]*}"}
        part=${part%"${part##*[![:space:]]}"}
        [[ $part == "$1" ]] && return 0
      done
    done
    return 1
  }

  __synt_command_line() {
    local entry pattern='^ *[0-9]+\*? +(.*)$'
    entry=$(HISTTIMEFORMAT= builtin history 1)
    if [[ $entry != "$__synt_history" && $entry =~ $pattern ]]; then
      printf '%s' "${BASH_REMATCH[1]}"
    else
      printf '%s' "$1"
    fi
  }

  __synt_preexec() {
    local command=$BASH_COMMAND line
    [[ $__synt_ready == 1 ]] || return
    __synt_in_prompt_command "$command" && return
    __synt_ready=0
    __synt_running=1
    line=$(__synt_command_line "$command")
    line=${line//\%/%25}
    line=${line//;/%3B}
    line=${line//[[:cntrl:]]/ }
    printf '\e]133;C;cmdline_url=%s\a' "$line"
  }

  trap '__synt_preexec' DEBUG
  if (( BASH_VERSINFO[0] > 5 || (BASH_VERSINFO[0] == 5 && BASH_VERSINFO[1] >= 1) )); then
    PROMPT_COMMAND=(__synt_precmd ${PROMPT_COMMAND[@]+"${PROMPT_COMMAND[@]}"})
  else
    PROMPT_COMMAND="__synt_precmd${PROMPT_COMMAND:+;$PROMPT_COMMAND}"
  fi
  PS1="$PS1\[$(printf '\e]133;B\a')\]"
fi

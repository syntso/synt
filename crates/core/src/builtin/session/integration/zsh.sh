if [[ -o interactive && -z ${__synt_integration-} ]]; then
  typeset -g __synt_integration=1
  typeset -gi __synt_running=0

  __synt_precmd() {
    local code=$?
    if (( __synt_running )); then
      printf '\e]133;D;%s\a' "$code"
      __synt_running=0
    fi
    printf '\e]133;A\a'
  }

  __synt_preexec() {
    local line=${1//\%/%25}
    line=${line//;/%3B}
    line=${line//[[:cntrl:]]/ }
    __synt_running=1
    printf '\e]133;C;cmdline_url=%s\a' "$line"
  }

  precmd_functions=(__synt_precmd $precmd_functions)
  preexec_functions+=(__synt_preexec)
  PS1="$PS1%{$(printf '\e]133;B\a')%}"
fi

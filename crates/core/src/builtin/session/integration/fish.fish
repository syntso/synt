if status is-interactive; and not set -q __synt_integration
    set -g __synt_integration 1

    function __synt_prompt --on-event fish_prompt
        printf '\e]133;A\a'
    end

    function __synt_preexec --on-event fish_preexec
        set -l line (string join ' ' -- $argv)
        set line (string replace -a '%' '%25' -- $line)
        set line (string replace -a ';' '%3B' -- $line)
        set line (string replace -ra '[[:cntrl:]]' ' ' -- $line)
        printf '\e]133;C;cmdline_url=%s\a' "$line"
    end

    function __synt_postexec --on-event fish_postexec
        printf '\e]133;D;%s\a' $status
    end
end

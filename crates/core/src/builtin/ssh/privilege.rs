const ESCALATORS: [&str; 13] = [
    "sudo",
    "sudoedit",
    "su",
    "doas",
    "pkexec",
    "run0",
    "runuser",
    "machinectl",
    "systemd-run",
    "chroot",
    "setpriv",
    "ssh",
    "nsenter",
];
const INDIRECTIONS: [&str; 21] = [
    "sh", "bash", "zsh", "dash", "ksh", "fish", "csh", "tcsh", "busybox", "eval", "exec", "source",
    ".", "env", "xargs", "python", "perl", "ruby", "node", "php", "base64",
];
const VERSIONED_INTERPRETERS: [&str; 3] = ["python", "perl", "ruby"];
const SEPARATORS: [char; 8] = [';', '&', '|', '(', ')', '<', '>', '\n'];
const EXPANSIONS: [char; 10] = ['$', '`', '\\', '{', '}', '!', '*', '?', '[', ']'];
const QUOTES: [char; 2] = ['"', '\''];

pub(super) fn escalates(command: &str) -> bool {
    if command.contains(EXPANSIONS) {
        return true;
    }

    command
        .split(|character: char| character.is_whitespace() || SEPARATORS.contains(&character))
        .filter(|word| !word.is_empty())
        .any(word_escalates)
}

pub(crate) fn is_root(user: &str) -> bool {
    user == "root"
}

fn word_escalates(word: &str) -> bool {
    if has_inner_quote(word) {
        return true;
    }

    let program = program_name(word);

    ESCALATORS.contains(&program) || INDIRECTIONS.contains(&program) || is_versioned(program)
}

fn has_inner_quote(word: &str) -> bool {
    let last = word.chars().count().saturating_sub(1);

    word.chars()
        .enumerate()
        .any(|(index, character)| QUOTES.contains(&character) && index != 0 && index != last)
}

fn program_name(word: &str) -> &str {
    let unquoted = word.trim_matches(QUOTES);

    unquoted.rsplit('/').next().unwrap_or(unquoted)
}

fn is_versioned(program: &str) -> bool {
    VERSIONED_INTERPRETERS.iter().any(|interpreter| {
        program.strip_prefix(interpreter).is_some_and(|version| {
            version
                .chars()
                .all(|character| character.is_ascii_digit() || character == '.')
        })
    })
}

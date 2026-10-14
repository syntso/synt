const LONGEST: usize = 64;

pub(crate) fn is_param_name(name: &str) -> bool {
    let mut characters = name.chars();

    let Some(first) = characters.next() else {
        return false;
    };

    let starts_well = first.is_ascii_alphabetic() || first == '_';
    let continues_well =
        characters.all(|character| character.is_ascii_alphanumeric() || character == '_');

    starts_well && continues_well && name.len() <= LONGEST
}

#[cfg(test)]
mod tests {
    use super::is_param_name;

    #[test]
    fn names_are_identifiers() {
        assert!(is_param_name("host"));
        assert!(is_param_name("_tag"));
        assert!(is_param_name("service_2"));

        assert!(!is_param_name(""));
        assert!(!is_param_name("2nd"));
        assert!(!is_param_name("dry-run"));
        assert!(!is_param_name("a b"));
        assert!(!is_param_name(&"a".repeat(65)));
    }
}

const LONGEST_NAME: usize = 64;

pub(crate) fn check_name(name: &str) -> Result<(), String> {
    let allowed = |character: char| character.is_ascii_alphanumeric() || "-_.".contains(character);

    if name.is_empty() || name.len() > LONGEST_NAME {
        return Err(format!("a name must be 1 to {LONGEST_NAME} characters"));
    }

    if name.starts_with('.') || !name.chars().all(allowed) {
        return Err(format!(
            "{name:?} is not a valid name: use letters, digits, '-', '_' and '.', not first"
        ));
    }

    Ok(())
}

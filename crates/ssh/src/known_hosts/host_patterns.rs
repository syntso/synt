use super::HashedHost;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct HostPatterns {
    list: String,
}

impl HostPatterns {
    pub(super) fn new(list: &str) -> Self {
        Self {
            list: list.to_owned(),
        }
    }

    pub(super) fn matches(&self, name: &str) -> bool {
        let mut matched = false;

        for pattern in self.list.split(',') {
            let (negated, pattern) = match pattern.strip_prefix('!') {
                Some(rest) => (true, rest),
                None => (false, pattern),
            };

            if !pattern_matches(pattern, name) {
                continue;
            }
            if negated {
                return false;
            }
            matched = true;
        }

        matched
    }
}

fn pattern_matches(pattern: &str, name: &str) -> bool {
    if HashedHost::is_hashed(pattern) {
        return HashedHost::parse(pattern).is_some_and(|hashed| hashed.matches(name));
    }

    let pattern: Vec<char> = pattern.to_lowercase().chars().collect();
    let name: Vec<char> = name.to_lowercase().chars().collect();

    glob(&pattern, &name)
}

fn glob(pattern: &[char], name: &[char]) -> bool {
    let (mut pattern_index, mut name_index) = (0, 0);
    let mut backtrack: Option<(usize, usize)> = None;

    while name_index < name.len() {
        match pattern.get(pattern_index) {
            Some('*') => {
                backtrack = Some((pattern_index, name_index));
                pattern_index += 1;
            }
            Some(&symbol) if symbol == '?' || symbol == name[name_index] => {
                pattern_index += 1;
                name_index += 1;
            }
            _ => {
                let Some((star, consumed)) = backtrack else {
                    return false;
                };
                backtrack = Some((star, consumed + 1));
                pattern_index = star + 1;
                name_index = consumed + 1;
            }
        }
    }

    pattern[pattern_index..].iter().all(|&symbol| symbol == '*')
}

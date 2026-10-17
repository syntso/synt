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

            if !Self::pattern_matches(pattern, name) {
                continue;
            }
            if negated {
                return false;
            }
            matched = true;
        }

        matched
    }

    fn pattern_matches(pattern: &str, name: &str) -> bool {
        if HashedHost::is_hashed(pattern) {
            return HashedHost::parse(pattern).is_some_and(|hashed| hashed.matches(name));
        }

        let pattern: Vec<char> = pattern.to_lowercase().chars().collect();
        let name: Vec<char> = name.to_lowercase().chars().collect();

        Self::glob(&pattern, &name)
    }

    fn glob(pattern: &[char], name: &[char]) -> bool {
        let (mut pattern_index, mut name_index) = (0, 0);
        let mut star: Option<usize> = None;
        let mut consumed = 0;

        while name_index < name.len() {
            match pattern.get(pattern_index) {
                Some('*') => {
                    star = Some(pattern_index);
                    consumed = name_index;
                    pattern_index += 1;
                }
                Some(&symbol) if symbol == '?' || symbol == name[name_index] => {
                    pattern_index += 1;
                    name_index += 1;
                }
                _ => {
                    let Some(at) = star else {
                        return false;
                    };

                    consumed += 1;
                    pattern_index = at + 1;
                    name_index = consumed;
                }
            }
        }

        pattern[pattern_index..].iter().all(|&symbol| symbol == '*')
    }
}

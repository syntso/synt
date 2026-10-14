use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

const BASH: &str = include_str!("bash.sh");
const ZSH: &str = include_str!("zsh.sh");
const FISH: &str = include_str!("fish.fish");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum IntegrationShell {
    Bash,
    Zsh,
    Fish,
}

impl IntegrationShell {
    pub const ALL: [Self; 3] = [Self::Bash, Self::Zsh, Self::Fish];

    pub fn snippet(self) -> &'static str {
        match self {
            Self::Bash => BASH,
            Self::Zsh => ZSH,
            Self::Fish => FISH,
        }
    }

    pub fn startup_file(self) -> &'static str {
        match self {
            Self::Bash => "~/.bashrc",
            Self::Zsh => "~/.zshrc",
            Self::Fish => "~/.config/fish/config.fish",
        }
    }
}

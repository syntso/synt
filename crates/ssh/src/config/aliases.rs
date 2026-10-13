use super::UserConfig;
use crate::SshError;

pub fn config_aliases() -> Result<Vec<String>, SshError> {
    let config = UserConfig::load()?;
    let mut aliases = config.aliases();

    aliases.sort();
    aliases.dedup();

    Ok(aliases)
}

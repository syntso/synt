use synt_ssh::Target;

use super::{HostResource, ProjectInfo, Resolved, Resource, ResourceEntry, ResourceListing};
use crate::CoreError;

pub(crate) const PERSONAL: &str = "personal";

const NAME: &str = "Personal";
const CONFIG: &str = ".ssh/config";

pub(crate) struct Personal;

impl Personal {
    pub(crate) fn info() -> Result<ProjectInfo, CoreError> {
        Ok(ProjectInfo {
            id: PERSONAL.to_owned(),
            name: NAME.to_owned(),
            slug: PERSONAL.to_owned(),
            path: config_path(),
            read_only: true,
            resources: aliases()?.len(),
            problems: Vec::new(),
        })
    }

    pub(crate) fn listing() -> Result<ResourceListing, CoreError> {
        let resources = aliases()?
            .iter()
            .map(|alias| Self::resource(alias))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(ResourceListing {
            project: PERSONAL.to_owned(),
            resources,
            problems: Vec::new(),
        })
    }

    pub(crate) fn resource(alias: &str) -> Result<ResourceEntry, CoreError> {
        let target = Self::target(alias)?;

        Ok(ResourceEntry {
            name: alias.to_owned(),
            resource: Resource::Host(host_resource(&target)),
        })
    }

    pub(crate) fn resolve(alias: &str) -> Result<Resolved, CoreError> {
        Ok(Resolved::Host(Self::target(alias)?))
    }

    pub(crate) fn read_only() -> CoreError {
        CoreError::Project(format!(
            "the Personal project is read-only; edit {} instead",
            config_path()
        ))
    }

    fn target(alias: &str) -> Result<Target, CoreError> {
        if !aliases()?.iter().any(|known| known == alias) {
            return Err(CoreError::Project(format!(
                "there is no Host {alias} in {}",
                config_path()
            )));
        }

        Ok(Target::resolve(alias)?)
    }
}

fn aliases() -> Result<Vec<String>, CoreError> {
    Ok(synt_ssh::config_aliases()?)
}

fn host_resource(target: &Target) -> HostResource {
    let via = target
        .jumps()
        .iter()
        .map(Target::label)
        .collect::<Vec<_>>()
        .join(",");

    HostResource {
        address: target.host().to_owned(),
        port: Some(target.port()),
        user: Some(target.user().to_owned()),
        via: (!via.is_empty()).then_some(via),
        identity: target
            .identity_files()
            .first()
            .map(|path| path.display().to_string()),
    }
}

fn config_path() -> String {
    synt_platform::home_directory().map_or_else(
        |_| format!("~/{CONFIG}"),
        |home| home.join(CONFIG).display().to_string(),
    )
}

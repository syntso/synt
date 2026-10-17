use std::borrow::Cow;
use std::num::NonZeroU16;

use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde::Deserialize;
use synt_actions::ActionError;
use synt_ssh::Destination;

use crate::CoreError;
use crate::projects::Resolved;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(try_from = "Fields")]
pub struct OpenParams {
    remote: Option<Destination>,
    local_port: Option<NonZeroU16>,
}

#[derive(Deserialize, JsonSchema)]
struct Fields {
    #[serde(default)]
    remote_host: Option<String>,

    #[serde(default)]
    remote_port: Option<u16>,

    #[serde(default)]
    local_port: Option<u16>,
}

impl OpenParams {
    pub(super) fn plan(&self, resolved: Resolved) -> Result<Resolved, ActionError> {
        match resolved {
            Resolved::Host(host) => {
                let remote = self.remote.clone().ok_or_else(|| {
                    ActionError::InvalidParams(
                        "remote_host and remote_port are needed unless the target is a tunnel \
                         resource"
                            .to_owned(),
                    )
                })?;

                Ok(Resolved::Tunnel {
                    host,
                    remote,
                    local: self.local_port,
                })
            }
            Resolved::Tunnel {
                host,
                remote,
                local,
            } => {
                self.check_matches_resource(&remote, local)?;

                Ok(Resolved::Tunnel {
                    host,
                    remote,
                    local: local.or(self.local_port),
                })
            }
            Resolved::Cluster { .. } => Err(ActionError::InvalidParams(
                "the target is a cluster; forward a port of it with k8s.port_forward".to_owned(),
            )),
        }
    }

    fn check_matches_resource(
        &self,
        remote: &Destination,
        local: Option<NonZeroU16>,
    ) -> Result<(), ActionError> {
        if self.remote.as_ref().is_some_and(|asked| asked != remote) {
            return Err(ActionError::InvalidParams(format!(
                "the tunnel resource forwards to {remote}; leave remote_host and remote_port out"
            )));
        }

        if let (Some(fixed), Some(asked)) = (local, self.local_port)
            && fixed != asked
        {
            return Err(ActionError::InvalidParams(format!(
                "the tunnel resource listens on local port {fixed}; leave local_port out"
            )));
        }

        Ok(())
    }

    fn remote(host: Option<String>, port: Option<u16>) -> Result<Option<Destination>, CoreError> {
        let (host, port) = match (host, port) {
            (Some(host), Some(port)) => (host, port),
            (None, None) => return Ok(None),
            _ => return Err(CoreError::RemoteApart),
        };

        if port == 0 {
            return Err(CoreError::RemotePortZero);
        }

        let destination = Destination::new(host.trim(), port)
            .map_err(|_| CoreError::NotARemoteHost(host.clone()))?;

        Ok(Some(destination))
    }
}

impl JsonSchema for OpenParams {
    fn schema_name() -> Cow<'static, str> {
        Cow::Borrowed("OpenParams")
    }

    fn json_schema(generator: &mut SchemaGenerator) -> Schema {
        Fields::json_schema(generator)
    }
}

impl TryFrom<Fields> for OpenParams {
    type Error = CoreError;

    fn try_from(fields: Fields) -> Result<Self, CoreError> {
        let remote = Self::remote(fields.remote_host, fields.remote_port)?;

        let local_port = match fields.local_port {
            Some(port) => Some(NonZeroU16::new(port).ok_or(CoreError::LocalPortZero)?),
            None => None,
        };

        Ok(Self { remote, local_port })
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::OpenParams;

    fn refusal(params: Value) -> String {
        serde_json::from_value::<OpenParams>(params)
            .unwrap_err()
            .to_string()
    }

    #[test]
    fn a_remote_needs_both_halves_and_a_named_host() {
        let open: OpenParams =
            serde_json::from_value(json!({ "remote_host": " db ", "remote_port": 5432 })).unwrap();

        assert_eq!(open.remote.unwrap().to_string(), "db:5432");
        assert_eq!(
            refusal(json!({ "remote_host": "db" })),
            "remote_host and remote_port go together"
        );
        assert_eq!(
            refusal(json!({ "remote_host": "two words", "remote_port": 1 })),
            "remote_host must be a host name or address, got \"two words\""
        );
        assert_eq!(
            refusal(json!({ "remote_host": "db", "remote_port": 0 })),
            "remote_port must be between 1 and 65535"
        );
    }

    #[test]
    fn a_local_port_of_zero_is_refused() {
        assert!(refusal(json!({ "local_port": 0 })).starts_with("local_port must be"));
        assert!(serde_json::from_value::<OpenParams>(json!({})).is_ok());
    }
}

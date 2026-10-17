macro_rules! share_types {
    ($($types:ident)::+) => {
        impl From<$($types)::+::Action> for crate::interface::DescribedAction {
            fn from(action: $($types)::+::Action) -> Self {
                Self::new(
                    action.id,
                    action.title,
                    action.risk.into(),
                    action.takes_target,
                    action.params_schema,
                )
            }
        }

        impl From<$($types)::+::Risk> for synt_actions::Risk {
            fn from(risk: $($types)::+::Risk) -> Self {
                match risk {
                    $($types)::+::Risk::Read => Self::Read,
                    $($types)::+::Risk::Write => Self::Write,
                    $($types)::+::Risk::Privileged => Self::Privileged,
                }
            }
        }

        impl From<$($types)::+::Level> for crate::interface::Level {
            fn from(level: $($types)::+::Level) -> Self {
                match level {
                    $($types)::+::Level::Trace => Self::Trace,
                    $($types)::+::Level::Debug => Self::Debug,
                    $($types)::+::Level::Info => Self::Info,
                    $($types)::+::Level::Warn => Self::Warn,
                    $($types)::+::Level::Error => Self::Error,
                }
            }
        }

        impl From<$($types)::+::Error> for crate::interface::GuestError {
            fn from(error: $($types)::+::Error) -> Self {
                Self::new(error.code.into(), error.message)
            }
        }

        impl From<crate::interface::GuestError> for $($types)::+::Error {
            fn from(error: crate::interface::GuestError) -> Self {
                Self {
                    code: error.code().to_string(),
                    message: error.message().to_owned(),
                }
            }
        }

        impl crate::runtime::Template for ExtensionPre<crate::runtime::Sandbox> {
            fn instance(
                &self,
                store: &mut wasmtime::Store<crate::runtime::Sandbox>,
            ) -> wasmtime::Result<Box<dyn crate::runtime::Exports>> {
                Ok(Box::new(self.instantiate(store)?))
            }
        }
    };
}

pub(super) use share_types;

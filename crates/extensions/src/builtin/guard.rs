use synt_actions::{ActionError, Context};
use synt_journal::Actor;

pub(super) fn refuse_extensions(context: &Context) -> Result<(), ActionError> {
    if matches!(context.actor(), Actor::Plugin(_)) {
        return Err(ActionError::failed(
            "an extension cannot reach the views, panels or actions of an extension",
        ));
    }

    Ok(())
}

pub(super) fn calling_extension(context: &Context) -> Result<String, ActionError> {
    match context.actor() {
        Actor::Plugin(name) => Ok(name.as_str().to_owned()),
        _ => Err(ActionError::failed(
            "only an extension sends requests through extension.http",
        )),
    }
}

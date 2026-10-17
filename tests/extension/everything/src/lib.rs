mod actions;
mod documents;
mod files;
mod merge;
mod panel;
mod params;
mod reach;
mod records;
mod views;

use synt_extension_api::{ActionInfo, Error, Event, Extension, register_extension};

use crate::params::Params;

struct Everything;

impl Extension for Everything {
    fn describe() -> Vec<ActionInfo> {
        actions::all()
    }

    fn run(action: &str, _target: Option<&str>, params: &str) -> Result<String, Error> {
        let params = Params::parse(params)?;

        let answer = match action {
            "every.ping" => reach::ping()?,
            "every.fetch" => reach::fetch(&params)?,
            "every.settings" => reach::settings()?,
            "every.remember" => records::remember(&params)?,
            "every.recall" => records::recall(&params)?,
            "every.arm" => records::arm(&params)?,
            "every.disarm" => records::disarm(&params)?,
            "every.heard" => records::heard(),
            "every.write" => documents::write(&params)?,
            "every.read" => documents::read(&params)?,
            "every.list" => documents::list(&params)?,
            "every.remove" => documents::remove(&params)?,
            "every.files" => files::chosen(),
            "every.read_file" => files::read(&params)?,
            "every.write_file" => files::write(&params)?,
            "every.patch" => views::patch()?,
            "every.post" => panel::post(&params)?,
            other => return Err(Error::new("unknown_action", other)),
        };

        Ok(answer.to_string())
    }

    fn start() -> Result<(), Error> {
        records::started()
    }

    fn render(view: &str, target: Option<&str>, _params: &str) -> Result<String, Error> {
        views::render(view, target)
    }

    fn panel_message(panel: &str, message: &str) -> Result<String, Error> {
        panel::answer(panel, message)
    }

    fn asset(path: &str) -> Option<Vec<u8>> {
        panel::asset(path)
    }

    fn on_event(event: &Event) -> Result<(), Error> {
        records::hear(event)
    }

    fn on_timer(name: &str) -> Result<(), Error> {
        records::beat(name)
    }

    fn merge(
        kind: &str,
        base: Option<&[u8]>,
        ours: &[u8],
        theirs: &[u8],
    ) -> Result<Vec<u8>, Error> {
        merge::lines(kind, base, ours, theirs)
    }
}

register_extension!(Everything);

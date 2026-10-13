use synt_actions::ActionError;
use synt_journal::{JournalError, JournalReader, JournalWriter};
use tokio::task;

pub(super) async fn read<T, Query>(journal: &JournalWriter, query: Query) -> Result<T, ActionError>
where
    T: Send + 'static,
    Query: FnOnce(&JournalReader) -> Result<T, JournalError> + Send + 'static,
{
    let journal = journal.clone();

    let reading = task::spawn_blocking(move || {
        let reader = journal.reader()?;
        query(&reader)
    });

    let read = reading
        .await
        .map_err(|_| ActionError::failed("reading the journal stopped unexpectedly"))?;

    read.map_err(|error| ActionError::failed(format!("the journal cannot be read: {error}")))
}

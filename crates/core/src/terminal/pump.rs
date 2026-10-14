use std::sync::Arc;

use super::{ConsoleOutput, Terminal, Terminals};

pub(super) async fn pump(terminals: Terminals, terminal: Arc<Terminal>, mut output: ConsoleOutput) {
    loop {
        let data = tokio::select! {
            biased;
            () = terminal.stopped().cancelled() => break,
            data = output.next() => data,
        };

        let Some(data) = data else {
            break;
        };

        terminal.show(data).await;
    }

    let ending = terminal.end(output.end()).await;
    terminals.retire(&terminal, &ending).await;

    terminal.mark_finished();
}

use std::io::{self, BufWriter, Write};

use crate::error::JournalError;
use crate::filter::Filter;
use crate::model::Seq;
use crate::reader::JournalReader;

const PAGE: u32 = 1_000;

pub(crate) fn jsonl(
    reader: &JournalReader,
    filter: &Filter,
    out: impl Write,
) -> Result<u64, JournalError> {
    let mut out = BufWriter::new(out);
    let mut after = Seq::ORIGIN;
    let mut written = 0;

    loop {
        let page = reader.page(filter, Some(after), PAGE)?;

        let Some(last) = page.last() else {
            break;
        };
        after = last.seq();

        for event in &page {
            serde_json::to_writer(&mut out, event).map_err(io::Error::from)?;
            out.write_all(b"\n")?;
            written += 1;
        }
    }

    out.flush()?;
    Ok(written)
}

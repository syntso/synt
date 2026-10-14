use futures_util::TryStreamExt;
use mysql_async::prelude::Protocol;
use mysql_async::{QueryResult, Row};

use super::cell::Cell;
use crate::paging::Collector;
use crate::{DbError, Query, Rows};

pub(super) async fn collect<P>(
    mut result: QueryResult<'_, '_, P>,
    query: Query<'_>,
) -> Result<Rows, DbError>
where
    P: Protocol + Unpin,
{
    let columns = result.columns().unwrap_or_default();
    let names = columns
        .iter()
        .map(|column| column.name_str().into_owned())
        .collect();

    let mut collector = Collector::new(query.page);

    if let Some(mut stream) = result.stream::<Row>().await? {
        while let Some(row) = stream.try_next().await? {
            if collector.wants_more() {
                collector.offer(Cell::read_row(row));
            } else if query.access.is_read_only() {
                break;
            }
        }
    }

    let affected = (!query.access.is_read_only()).then(|| result.affected_rows());
    result.drop_result().await?;

    Ok(collector.finish(names, affected))
}

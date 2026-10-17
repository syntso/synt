use super::{Damage, Verification};
use crate::anchors::Mark;
use crate::chain::Chain;
use crate::store::RowView;

pub(crate) struct Walk<'a> {
    chain: &'a Chain,
    head: Mark,
    previous: Mark,
    checked: u64,
    scratch: Vec<u8>,
}

impl<'a> Walk<'a> {
    pub(crate) fn new(chain: &'a Chain, start: Mark, head: Mark) -> Self {
        Self {
            chain,
            head,
            previous: start,
            checked: 0,
            scratch: Vec::with_capacity(512),
        }
    }

    pub(crate) fn step(&mut self, row: &RowView<'_>) -> Result<(), Verification> {
        let expected = self.previous.seq.next();

        if row.seq != expected {
            return Err(Verification::Broken {
                at: expected,
                damage: Damage::Missing,
            });
        }

        let altered = Verification::Broken {
            at: row.seq,
            damage: Damage::Altered,
        };
        let fields = row.fields().ok_or(altered)?;

        fields.write(&mut self.scratch);
        let link = self.chain.link(&self.previous.link, &self.scratch);

        if row.link != Some(link) {
            return Err(altered);
        }
        if row.seq == self.head.seq && link != self.head.link {
            return Err(altered);
        }

        self.previous = Mark::new(row.seq, link);
        self.checked += 1;

        Ok(())
    }

    pub(crate) fn last(&self) -> Mark {
        self.previous
    }

    pub(crate) fn finish(self) -> Verification {
        if self.previous.seq < self.head.seq {
            return Verification::Truncated {
                last: self.previous.seq,
                head: self.head.seq,
            };
        }

        Verification::Intact {
            checked: self.checked,
        }
    }
}

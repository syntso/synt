use std::ops::ControlFlow;

use super::JournalId;
use crate::anchors::{Anchors, Mark};
use crate::chain::Chain;
use crate::error::JournalError;
use crate::seq::Seq;
use crate::store::{Store, Tail, Writable};
use crate::verify::{Damage, Verification, Walk};

pub(super) struct Recovery<'a> {
    store: &'a Store<Writable>,
    chain: &'a Chain,
}

impl<'a> Recovery<'a> {
    pub(super) fn new(store: &'a Store<Writable>, chain: &'a Chain) -> Self {
        Self { store, chain }
    }

    pub(super) fn anchors(
        &self,
        journal: JournalId,
        stored: Option<Anchors>,
        tail: Option<Tail>,
    ) -> Result<Anchors, JournalError> {
        let known = match stored {
            Some(anchors) => anchors,
            None if tail.is_none() => Anchors::genesis(self.chain.genesis()),
            None => return Err(JournalError::AnchorsMissing(journal)),
        };

        let head = self.head(known)?;

        Ok(known.with_head(head))
    }

    fn head(&self, anchors: Anchors) -> Result<Mark, JournalError> {
        let start = self.before_head(anchors)?;
        let mut walk = Walk::new(self.chain, start, anchors.head);

        let broken = self.store.scan(start.seq, |row| match walk.step(row) {
            Ok(()) => ControlFlow::Continue(()),
            Err(broken) => ControlFlow::Break(broken),
        })?;

        if let Some(broken) = broken {
            return Err(JournalError::Tampered(broken));
        }
        if walk.last().seq < anchors.head.seq {
            return Err(JournalError::Tampered(walk.finish()));
        }

        Ok(walk.last())
    }

    fn before_head(&self, anchors: Anchors) -> Result<Mark, JournalError> {
        let head = anchors.head;

        if head.seq <= anchors.floor.seq {
            return Ok(head);
        }

        let previous = Seq::new(head.seq.get() - 1);

        if previous == anchors.floor.seq {
            return Ok(anchors.floor);
        }

        match self.store.link_at(previous)? {
            Some(link) => Ok(Mark::new(previous, link)),
            None => Err(JournalError::Tampered(self.missing(previous)?)),
        }
    }

    fn missing(&self, previous: Seq) -> Result<Verification, JournalError> {
        let last = self.store.tail()?.map_or(Seq::ORIGIN, |tail| tail.seq);

        if last < previous {
            return Ok(Verification::Truncated {
                last,
                head: previous.next(),
            });
        }

        Ok(Verification::Broken {
            at: previous,
            damage: Damage::Missing,
        })
    }
}

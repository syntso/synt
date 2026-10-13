# synt-journal

Append-only journal of everything the Synt core does.

Every action leaves events here: requested, approved or denied, started, finished. Events can be
appended and read, never changed or removed. Each one carries a keyed hash of the one before it,
so a later edit anywhere in the history breaks the chain and shows up on verification.

| Type | What it is |
| --- | --- |
| `Seq` | Position of an event in the journal |

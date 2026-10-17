use synt_extension_api::Error;

pub fn lines(
    kind: &str,
    _base: Option<&[u8]>,
    ours: &[u8],
    theirs: &[u8],
) -> Result<Vec<u8>, Error> {
    if kind != "ext.every.note" {
        return Err(Error::new("unknown_kind", kind));
    }

    let ours = String::from_utf8_lossy(ours);
    let theirs = String::from_utf8_lossy(theirs);

    let mut merged: Vec<&str> = ours.lines().collect();
    for line in theirs.lines() {
        if !merged.contains(&line) {
            merged.push(line);
        }
    }

    Ok(merged.join("\n").into_bytes())
}

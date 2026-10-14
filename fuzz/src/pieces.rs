pub fn pieces(data: &[u8]) -> (&[u8], Vec<&[u8]>) {
    let Some((&seed, rest)) = data.split_first() else {
        return (data, Vec::new());
    };

    let size = usize::from(seed % 16) + 1;
    let split = rest.chunks(size).collect();

    (rest, split)
}

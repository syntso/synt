macro_rules! log {
    ($($message:tt)+) => {
        eprintln!("synt-core: {}", format_args!($($message)+))
    };
}

pub(crate) use log;

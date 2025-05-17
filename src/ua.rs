/// User agent for this library. Prepend with your own application user_agent, using [`ua`].
pub const UA: &str = concat!(
    "mw.rs/",
    env!("CARGO_PKG_VERSION"),
    " (https://github.com/fee1-dead/mw)"
);

#[macro_export]
macro_rules! ua {
    () => {
        $crate::ua!(concat!(
            env!("CARGO_PKG_NAME"),
            "/",
            env!("CARGO_PKG_VERSION")
        ))
    };
    ($prefix: expr) => {{
        const PREFIX: &str = $prefix;
        const SEMI: &str = "; ";
        const LEN: usize = PREFIX.len() + SEMI.len() + $crate::UA.len();
        // excuse me for using 1.87.0 const-stabilized features owo
        const OUT: [u8; LEN] = {
            let mut x = [0; LEN];
            let (prefix, rest) = x.split_at_mut(PREFIX.len());
            let (semi, rest) = rest.split_at_mut(SEMI.len());
            prefix.copy_from_slice(PREFIX.as_bytes());
            semi.copy_from_slice(SEMI.as_bytes());
            rest.copy_from_slice($crate::UA.as_bytes());
            x
        };

        const UA: &str = match ::core::str::from_utf8(&OUT) {
            Ok(s) => s,
            Err(_) => unreachable!(),
        };
        UA
    }};
}

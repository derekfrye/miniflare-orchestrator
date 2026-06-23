use std::net::TcpListener;
use std::sync::atomic::{AtomicU32, Ordering};

static NEXT_PORT: AtomicU32 = AtomicU32::new(20_000);

pub(crate) fn available_port_block(len: u16) -> u16 {
    let (base, listeners) = reserved_port_block(len);
    drop(listeners);
    base
}

pub(crate) fn reserved_port_block(len: u16) -> (u16, Vec<TcpListener>) {
    let len = u32::from(len.max(1));
    for _ in 0..40_000 {
        let base = NEXT_PORT.fetch_add(len + 8, Ordering::SeqCst);
        if base + len >= 60_000 {
            NEXT_PORT.store(20_000, Ordering::SeqCst);
            continue;
        }
        if let Some(listeners) = bind_port_block(base as u16, len as u16) {
            return (base as u16, listeners);
        }
    }
    panic!("no available port block");
}

fn bind_port_block(base: u16, len: u16) -> Option<Vec<TcpListener>> {
    (0..len)
        .map(|offset| TcpListener::bind(("0.0.0.0", base + offset)))
        .collect::<Result<Vec<_>, _>>()
        .ok()
}

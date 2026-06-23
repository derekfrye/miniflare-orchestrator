use std::net::TcpListener;

#[must_use]
pub fn available_port_ranges(port_count: u16) -> ((u16, u16), (u16, u16)) {
    assert!(port_count > 0, "port count must be positive");
    let total = port_count.checked_mul(2).expect("port count overflow");
    let mut seed = random_available_port();

    for _ in 0..10_000 {
        let base = normalize_port_base(seed, total);
        if let Some(ranges) = available_contiguous_ranges(base, port_count, total) {
            return ranges;
        }
        seed = seed.wrapping_add(total).wrapping_add(17);
    }
    panic!("no available port ranges");
}

fn random_available_port() -> u16 {
    TcpListener::bind(("127.0.0.1", 0))
        .expect("bind ephemeral port")
        .local_addr()
        .expect("ephemeral local addr")
        .port()
}

fn normalize_port_base(port: u16, total: u16) -> u16 {
    const MIN_TEST_PORT: u16 = 10_000;
    // Keep explicit test allocations below Linux's default ephemeral range
    // (32768..60999). Miniflare/workerd also binds internal sockets with
    // port 0, and those ephemeral allocations must not collide with the
    // lane-local worker/inspector ranges used by stress tests.
    const MAX_TEST_PORT: u16 = 30_000;
    assert!(
        total < MAX_TEST_PORT - MIN_TEST_PORT,
        "requested port block is too large for the non-ephemeral test range"
    );
    let span = MAX_TEST_PORT - MIN_TEST_PORT - total;
    MIN_TEST_PORT + (port.saturating_sub(MIN_TEST_PORT) % span)
}

fn available_contiguous_ranges(
    base: u16,
    port_count: u16,
    total: u16,
) -> Option<((u16, u16), (u16, u16))> {
    let worker_ports = base..base + port_count;
    let inspector_ports = base + port_count..base + total;
    let listeners = worker_ports
        .clone()
        .map(|port| TcpListener::bind(("0.0.0.0", port)))
        .chain(
            inspector_ports
                .clone()
                .map(|port| TcpListener::bind(("127.0.0.1", port))),
        )
        .collect::<Result<Vec<_>, _>>();
    if listeners.is_err() {
        return None;
    }
    Some((
        (base, base + port_count - 1),
        (base + port_count, base + total - 1),
    ))
}

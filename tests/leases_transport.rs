#[path = "support/basic.rs"]
mod basic;
#[path = "support/lease_redirect_fixtures.rs"]
mod lease_redirect_fixtures;
#[path = "support/lease_shared_fixtures.rs"]
mod lease_shared_fixtures;
#[path = "support/plan_fixture.rs"]
mod plan_fixture;
#[path = "support/request_json.rs"]
mod request_json;

#[path = "leases_transport/env_isolation.rs"]
mod env_isolation;
#[path = "leases_transport/miniflare.rs"]
mod miniflare;
#[path = "leases_transport/miniflare_stress.rs"]
mod miniflare_stress;
#[path = "leases_transport/protocols.rs"]
mod protocols;
#[path = "leases_transport/support.rs"]
mod support;

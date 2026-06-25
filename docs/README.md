# Miniflare Orchestrator

`worker-runtime-host-gen` runs local Cloudflare Worker projects inside a shared
container and exposes a lease-based control plane, so automated tests can start and 
stop locally hosted `miniflare` services on demand (and without requiring `wrangler` and 
its overhead). It is designed to support automated test environments where many Worker 
runtimes need isolated ports, state, logs, and restart lifecycles.

The workspace also includes `miniflare-lease-client`, a small blocking Rust
client for creating, bundling, restarting, probing, and deleting test leases.

## What it provides

- a control-plane server for starting and stopping Miniflare through leases (see [lease workflow](#lease-workflow) for details)
- per-lease runtime, static, state, and log directories to isolate workers and inspect state
- an example Quadlet unit for starting the orchestrator
- Miniflare and Wrangler-backed launch modes

## Why it exists
- local-only, multi-tenant control plane for Cloudflare worker test runtimes
- rapidly deploy, start, and stop Miniflare-backed for integration tests that need network access to a realistic runtime
- faster and more isolated than running several `wrangler dev` processes in parallel

## Supporting files

- `Dockerfile` - runtime host image build
- `miniflare.container` - example Quadlet unit for a Podman-based host

## Binaries

- `worker-runtime-host-gen` - validate a manifest, write a debug plan, and render `s6` service directories
- `worker-runtime-host-docs` - serve the runtime host documentation and lease API
- `worker-runtime-host-init` - initialize generated service directories at container startup
- `worker-runtime-host-worker` - run one Worker runtime service
- `worker-runtime-host-watch` - watch a reload token and restart one Worker service

## Container Image

Build the runtime host image from the repository root:

```sh
podman build -t localhost/miniflare:latest -f Dockerfile .
```

The provided Quadlet unit is an example. Its image name, network, IP addresses,
published ports, and volume names should be adjusted for your local Podman
setup.

By default, the container runs in `leases-only` mode. Set
`WORKER_RUNTIME_HOST_MODE=manifest_and_leases` and provide
`WORKER_RUNTIME_HOST_MANIFEST=/path/to/projects.json` to prewire fixed projects
from a manifest.

## Generator Usage

The main generator accepts two subcommands:

- `validate` - validate the manifest and write the plan file
- `generate` - validate the manifest, write the plan file, and render service directories

Common flags:

- `--manifest` - manifest file path, default `/work/host/config/projects.json`
- `--output-dir` - generated `s6-rc` service source directory, default `/etc/s6-overlay/s6-rc.d`
- `--plan-file` - debug plan JSON output, default `/work/host/config/projects.plan.json`
- `--service-root` - service root used by watcher restart commands, default `/run/service`
- `--log-level` - Wrangler log level, default `warn`

Example:

```sh
cargo run --bin worker-runtime-host-gen -- generate \
  --manifest /work/host/config/projects.json \
  --output-dir /etc/s6-overlay/s6-rc.d \
  --plan-file /work/host/config/projects.plan.json
```

## Lease Workflow

The docs service is the control plane for lease-based automated tests. A
typical lease flow is:

1. `POST /leases` to allocate a Worker request port, inspector port, and project root.
2. `POST /leases/{id}/bundle` to upload prebuilt runtime files and static assets.
3. `POST /leases/{id}/restart` to launch the Worker.
4. Poll `GET /leases/{id}` until `status.state` becomes `ready`.
5. Use the returned `base_url` or `health_url` for tests.
6. `DELETE /leases/{id}` to stop and release the lease.

Useful inspection endpoints include:

- `GET /leases/{id}/debug`
- `GET /leases/{id}/probe`
- `GET /leases/{id}/filesystem-snapshot`
- `GET /leases/{id}/logs`
- `GET /leases/{id}/logs/tail?lines=N`

The service contract is also available from the running docs server through
`/instructions.json`, `/instructions.html`, and `/openapi.yaml`.

## Bundle Contract

Lease launches serve uploaded prebuilt artifacts. Wrangler build hooks in
uploaded `wrangler.toml` files are skipped, so callers should build Worker
artifacts before uploading a bundle.

At minimum, a runnable bundle should include:

- `wrangler.toml`
- `worker_entry.mjs` or the configured Worker entrypoint
- any built JavaScript, Wasm, or static assets imported by the entrypoint

Bundle metadata can include `source_root`, `build_command`, `artifact_paths`,
`source_paths`, `source_fingerprint`, and `bundle_description`. When source
metadata is present, lease responses can report diagnostics about stale bundles.
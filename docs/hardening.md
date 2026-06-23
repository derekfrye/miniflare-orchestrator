# Runtime Host Hardening

This project runs worker code and test-provided bundles inside a shared runtime host. The code treats lease filesystems, bundle payloads, and runtime metadata as potentially hostile boundaries.

## Filesystem Boundaries

Lease-owned trees are handled through directory capabilities from `cap-std` where practical. After the configured root directory is opened, operations inside a lease use directory-relative APIs instead of repeatedly resolving absolute paths.

This applies to:

- bundle extraction into runtime and static directories
- recursive log collection
- filesystem snapshots
- state directory clearing before non-persistent restarts
- lease deletion
- log file creation for spawned workers

Opening or creating the configured root directories is still necessarily ambient because those roots come from configuration. Once opened, sensitive subtree work is scoped to the directory handle.

## Bundle Extraction

Uploaded bundle paths are normalized before use. Absolute paths, `..`, prefixes, and empty paths are rejected. Duplicate normalized paths are rejected before writing.

Bundle files are created with `create_new(true)` and `0600` permissions. Bundle directories are created with `0700` permissions. Replacing a bundle clears the already-open directory contents rather than trusting a path across a delete and recreate sequence.

## Permissions

Runtime-created private directories are created with `0700` where the platform supports Unix modes. Runtime-created private files are created with `0600`.

Generated executable service scripts are created with the executable mode at open time. The code avoids create-then-chmod for the security-sensitive paths it controls.

## Path Checks

Path validation uses `Path` operations instead of string-prefix comparisons. For lease metadata paths such as `artifact_paths` and `source_paths`, only normalized relative paths are accepted for filesystem checks.

The code avoids lossy path conversion at Unix boundaries where bytes matter. Envdir values that contain paths are written as Unix bytes, not through `to_string_lossy`.

## Text And Bytes

Unix filesystem paths and file contents are not assumed to be UTF-8 unless the API contract requires text. Log endpoints return text, so invalid UTF-8 in log files is surfaced as an error instead of silently replacing bytes with lossy output.

Shell script generation still requires UTF-8 paths because the script itself is text. Non-UTF-8 service paths fail validation rather than being lossy-converted.

## Errors

Filesystem errors that affect lease lifecycle are propagated instead of silently ignored. Some best-effort diagnostic reads, such as retained failure-report log tails, may still degrade gracefully because the primary operation should continue.

## Remaining Trust Boundaries

The runtime host still relies on external container isolation, configured mount layout, and the configured root paths being sensible. The hardening in this crate narrows filesystem authority after those roots are opened; it is not a replacement for container, user, mount, or network isolation.

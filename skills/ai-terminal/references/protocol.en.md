# Protocol, exit codes and troubleshooting

[简体中文](protocol.md) | English

Commands emit one UTF-8 JSON object; --help/--version emit text. Top-level fields are schema_version (currently 1), request_id, instance_id, ok, and result or error. Branch on error.code; message explains it. Parsing/connection failures may have empty identity fields. The console CLI initializes no graphical window, fonts or GPU.

| Exit | Common cause | Action |
| --- | --- | --- |
| 0 | Success, including dry-run | Inspect result |
| 1 | Launch/save/transport failure or outcome_unknown | Read error.code; recover receipts/state before retrying timeouts |
| 2 | Invalid argument/directory/schema/size | Correct the input; invalid batches change nothing |
| 3 | not_running | Open the matching CLI-enabled manager; do not edit configuration as a substitute |
| 4 | not_found / no_current_session | Query UUIDs; external callers cannot use current |
| 5 | conflict / session_running / request_id_reused | Inspect state and intent; do not blindly force-close or retry with a new ID |
| 6 | busy | Full queue, shutdown or expiry before execution; check state later |

## Timeouts and deduplication

Mutations accept --request-id <UUID>. The manager caches the latest 64 mutation receipts, at most 4 MiB total, only in the current process. The same ID and complete request return the original receipt; a changed request with the same ID returns request_id_reused. Queries/dry-runs do not consume mutation receipts. This is not a persistent job queue and cannot guarantee deduplication after restart or eviction.

A timeout may occur before or after execution. First use request show <UUID> against the same manager and inspect session list/workspace show. If the receipt remains, reuse the identical ID and parameters to recover it. If evicted or restarted, inspect actual objects before deciding what to do; do not blindly rebuild with a new ID.

Generate and retain request IDs before mutations when recovery matters. An automatically generated ID cannot be recovered if no response arrives. Do not reuse a dry-run ID for committing, or change caller_session/parameters under an existing ID.

## Limits and privacy

A separate management channel listens on a random 127.0.0.1 port, independent of lifecycle hooks. The token exists in plaintext only in manager memory and owned child-process environments; the instance file uses current-user Windows DPAPI encryption. Do not inspect or display tokens: discovery/decryption belongs to the CLI. Explicit --data-dir wins. A stale inherited instance never silently falls back to another workspace. New-version managers allow one writer per data directory; previews need separate directories. Older versions lack this lease and must not run against the same data directory concurrently.

Requests are bounded at 256 KiB, responses at 1 MiB, with 32 pending management requests, 16 queued connections and four connection workers. Reads and execution waits have deadlines. Paginate large lists with --offset/--limit; batches have at most 128 operations. There is no idle polling: events wake the root viewport and apply through background logic even while minimized. A running manager executes mutations; the CLI does not edit sessions.json offline.

0.14.0-preview.2 adds native_controls discovery, settings/viewing and explicit terminal interaction. Receipts retain only SHA-256 request fingerprints and acknowledgements; read contents are not cached. Lifecycle events remain authoritative for activity; running/readiness=process_started never implies task completion or readiness.

The newer CLI preflights native_controls through Info. Older managers return unsupported_command (exit 2), without attempting new commands or opening another instance. Captures are excluded from receipts; use window capture-status.

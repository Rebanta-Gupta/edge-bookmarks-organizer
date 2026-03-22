# Edge Bookmarks Organizer

A safe Rust CLI for cleaning and reorganizing Microsoft Edge bookmarks.

It can:

- inspect bookmark stats
- detect and remove duplicate URLs
- check and remove dead links
- reorganize bookmarks by domain or topic
- create/list/restore/prune backups
- run profile safety checks before destructive operations

Built for Windows Edge profile files, with guardrails to reduce data-loss risk.

## Why This Tool

Edge stores bookmarks in a JSON file that can become messy over time:

- duplicates from imports/sync
- dead links
- flat structures that are hard to browse

This tool parses the file, performs cleanup/rebuild operations, and writes changes only when you opt in with `--apply`.

## Features

- Dry-run by default for destructive commands.
- Automatic backups before writes.
- Restore and prune backup support.
- Topic-based organization with subfolders.
- Domain grouping.
- Dead-link checks with configurable timeout/concurrency.
- Doctor command for preflight diagnostics.
- Safety guard: apply/save commands are blocked if parsed bookmark count is zero.

## Current Command Set

- `import`
- `list-domains`
- `list-duplicates`
- `remove-duplicates`
- `check-dead`
- `remove-dead`
- `rebuild`
- `save`
- `backup`
- `assign-topics`
- `doctor`

## Requirements

- Rust (stable)
- Cargo
- Microsoft Edge installed (for auto-detected default profile path)

## Installation

### Option 1: Run without installing globally

```powershell
git clone https://github.com/<your-user>/edge-bookmarks-organizer.git
cd edge-bookmarks-organizer
cargo run -- --help
```

### Option 2: Install as a CLI binary

```powershell
cargo install --path .
edge-bookmarks --help
```

## Edge Bookmark File Paths (Windows)

- Default profile: `C:\Users\<you>\AppData\Local\Microsoft\Edge\User Data\Default\Bookmarks`
- Other profile example: `C:\Users\<you>\AppData\Local\Microsoft\Edge\User Data\Profile 3\Bookmarks`

You can pass a path with `--bookmarks-file`, or rely on auto-detection for default profile.

## Safe Quick Start

1. Close Edge.
2. Run doctor.
3. Run dry-run commands first.
4. Apply only after output looks right.
5. Verify in Edge.

Example:

```powershell
$bf="C:\Users\<you>\AppData\Local\Microsoft\Edge\User Data\Default\Bookmarks"
cargo run -- doctor --bookmarks-file "$bf"
cargo run -- rebuild --strategy topic --bookmarks-file "$bf"
cargo run -- rebuild --strategy topic --apply --bookmarks-file "$bf"
cargo run -- import --bookmarks-file "$bf"
```

## Reorganization Strategies

`rebuild --strategy domain`

- Top-level folders grouped by domain.

`rebuild --strategy preserve`

- Rewrites file while preserving original folder hierarchy.

`rebuild --strategy topic`

- Assigns topics, then groups bookmarks by Topic -> Domain subfolders.

## Cleanup Workflows

### Remove duplicates

Dry-run:

```powershell
cargo run -- remove-duplicates --bookmarks-file "$bf"
```

Apply:

```powershell
cargo run -- remove-duplicates --apply --bookmarks-file "$bf"
```

Keep most recently used duplicate instead of first:

```powershell
cargo run -- remove-duplicates --apply --keep-recent --bookmarks-file "$bf"
```

### Check dead links

```powershell
cargo run -- check-dead --timeout 5 --concurrency 10 --bookmarks-file "$bf"
```

Only show dead/unreachable:

```powershell
cargo run -- check-dead --only-dead --bookmarks-file "$bf"
```

### Remove dead links

Dry-run:

```powershell
cargo run -- remove-dead --bookmarks-file "$bf"
```

Apply:

```powershell
cargo run -- remove-dead --apply --timeout 5 --concurrency 10 --bookmarks-file "$bf"
```

## Backup and Restore

List backups:

```powershell
cargo run -- backup --list --bookmarks-file "$bf"
```

Create backup:

```powershell
cargo run -- backup --bookmarks-file "$bf"
```

Restore backup:

```powershell
cargo run -- backup --restore "Bookmarks.backup_YYYYMMDD_HHMMSS" --bookmarks-file "$bf"
```

Prune old backups (keep latest 10):

```powershell
cargo run -- backup --prune 10 --bookmarks-file "$bf"
```

## Import Output

`import` prints:

- summary counts
- top domains
- top-level folder counts
- one level of subfolder counts under each top-level folder

This helps verify that reorganization generated the expected nested structure.

## Doctor Command

Use doctor before apply operations:

```powershell
cargo run -- doctor --bookmarks-file "$bf"
```

Doctor reports:

- target bookmarks file
- file size
- parsed bookmark count
- backup count/latest
- current target profile vs Edge last-used profile

## Troubleshooting

### I changed file contents but Edge UI still looks unchanged

- You may be looking at a different Edge profile.
- Sync may override local changes.
- Close Edge before running apply operations.
- Re-open Edge using the same profile as your target file.

### Output shows zero bookmarks

- The tool intentionally blocks apply/save when parsed bookmarks are zero.
- Restore from backup first.
- Re-run doctor and verify target file path.

### Dead-link checking is slow

- Increase concurrency carefully (max supported is 100).
- Use lower timeout for faster failure on bad hosts.

## Development

Run tests:

```powershell
cargo test
```

Run strict lint:

```powershell
cargo clippy --all-targets --all-features -- -D warnings
```

## License

MIT
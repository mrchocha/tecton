# Tecton Roadmap

Status as of 2026-10-23. Update and commit every Sunday.
Full learning plan (schedule, Patterns, SadServers, OSS): `~/Documents/Rahul/LEARNING-PLAN.md`
Architecture and rationale for every item below: [`rfcs/002-architecture.md`](rfcs/002-architecture.md)

## Direction
Maximum systems depth: sync core with our own threads and an `Io` trait (no tokio inside the engine), seqno-based internal keys, immutable `Version` snapshots, lock-free arena memtable, group-commit WAL, manifest, leveled compaction, and fault-injection testing. Every layer is hand-built and measured.

## Known gaps (`crates/storage/src/engine.rs`)
1. `get` only reads the memtable, no SST read path.
2. Memtable never rotates (`new_mem_table()` is never called; `flush()` only drains immutables).
3. No fsync anywhere, so the WAL is not truly durable.
4. `delete` mutates the memtable before writing the WAL.
5. `put` checks size and flushes before inserting; no group commit.
6. Missing: manifest, WAL replay and rotation, compaction, compression, GC.
7. `&'a StorageOptions` and `Rc` block background threads; move to `Arc`.
8. `tokio::fs` hides syscalls (it is `spawn_blocking`); engine I/O should go through our own `Io` trait.
9. `term`/`index` are `u32` and keys are `String`; no seqno/internal-key, so MVCC and correct tombstones are impossible.
10. `new_mem_table()` allocates a second `BTreeMemTable` and discards the first replacement.
11. No checksums on SST blocks or format version/magic.

## Phase 0, Weeks 1-2: Foundations (do before anything else, they touch every format)
- [ ] `SeqNo = u64`, `InternalKey` (user_key asc, seqno desc, kind), `Bytes` keys and values in the API
- [ ] Single `Error` enum with `Corruption { file, offset }`
- [ ] New `io` crate: `Fs` / `RandomAccess` / `Writable` traits, `PosixFs` (`pread`, `fdatasync`, `F_FULLFSYNC` on macOS, `fallocate`)
- [ ] Drop tokio from wal/sst/storage; engine runs on `std::thread` + channels
- [ ] `Arc<Db>` with `Arc<Options>`; remove `&'a` options and `Rc`
- [ ] Write `docs/format.md` (WAL record, SST block, footer, manifest edit)
- [ ] Metrics skeleton: bytes written/read, write amp counters

## Phase 1, Weeks 3-4: Make it correct
- [ ] WAL first, then memtable (fix `delete`)
- [ ] WAL: 32 KiB block framing (FULL/FIRST/MIDDLE/LAST) with CRC32C
- [ ] fsync policy option: per group / periodic / none, durability promise documented
- [ ] Group commit (leader/follower, one write + one fsync per group)
- [ ] Memtable rotation + immutable queue (no discarded allocation)
- [ ] SST v1: restart points + prefix compression, index block, bloom filter block, footer (magic, version, crc)
- [ ] SST read path: memtable -> immutables -> L0 newest first -> L1+ (bloom + index)
- [ ] WAL replay on startup (tolerate torn tail, error on mid-log corruption)
- [ ] `proptest` model test vs `BTreeMap`, including reopen
- [ ] Crash test: `kill -9` loop, verify no acknowledged write is lost

## Phase 2, Weeks 5-6: Manifest and file lifecycle
- [ ] `VersionEdit` log + `CURRENT` pointer (see SlateDB manifest design)
- [ ] `Version` / `VersionSet` with `arc-swap`; files deleted only when unreferenced
- [ ] WAL rotation and deletion after flush; orphan file cleanup on open
- [ ] Atomic replace: write temp, fsync, rename, fsync directory
- [ ] Background flush thread
- [ ] `FaultFs`: drop unsynced writes, torn write at byte N, EIO/ENOSPC; crash at every syscall boundary and compare to model
- [ ] `tecton-dump` tool for WAL/SST/manifest

## Phase 3, Weeks 7-9: Compaction
- [ ] Merge iterator (BinaryHeap, then loser tree)
- [ ] Leveled compaction (L0 file-count trigger, size ratio 10x)
- [ ] Tombstone dropping (bottom level / no overlap) and shadowed-version GC
- [ ] Background compaction pool, write slowdown and stop thresholds, compaction rate limiter
- [ ] Size-tiered compaction behind a strategy trait; compare write/read/space amp on the same workload
- [ ] Snapshots (`Db::snapshot()` pins `Version` + seqno) and forward/reverse iterators

## Phase 4, Weeks 10-11: Performance
- [ ] Sharded block cache (CLOCK, byte-charged, `Arc<Block>` pinned, zero-copy values)
- [ ] Compression: LZ4 then zstd, per block, skip if saving < 12%
- [ ] Register-blocked bloom filter (one cache line per probe)
- [ ] `criterion` micro-benchmarks, `db_bench`-style macro workloads (fillseq, fillrandom, readrandom, overwrite, scan)
- [ ] Flamegraphs (`perf` / `samply`), `strace -c`, HDR latency histograms (p50/p99/p99.9)
- [ ] `MemoryBudget` across memtables + cache + compaction buffers

## Phase 5, Weeks 12-14: Unsafe and concurrency depth
- [ ] Arena allocator + lock-free skiplist memtable (Release/Acquire), validated against the BTree oracle
- [ ] `loom` tests for skiplist and group commit; `miri` clean on all `unsafe`
- [ ] `cargo-fuzz` on WAL, SST, manifest decoders
- [ ] False-sharing audit (`repr(align(64))`), atomics ordering review
- [ ] SIMD CRC32C / varint / bloom with runtime feature detection

## Phase 6, Weeks 15-18: OS and hardware depth
- [ ] I/O experiments behind the `Fs` trait: `pread` + cache vs `mmap` vs `O_DIRECT` (aligned buffers) vs `io_uring` (fixed files/buffers); write up numbers
- [ ] Allocator comparison (system vs jemalloc vs mimalloc)
- [ ] Run inside jailer with cgroup v2 limits (`io.max`, `memory.max`); observe stalls and OOM behavior
- [ ] Filesystem comparison (ext4 / xfs / tmpfs): fsync cost, preallocation
- [ ] Deterministic simulation harness (seeded scheduler + `FaultFs`)

## Phase 7, stretch
- [ ] MVCC transactions / optimistic conflict detection on seqnos
- [ ] Range deletes, merge operator, TTL
- [ ] Thread-per-core sharded keyspace (Glommio/Seastar style)
- [ ] Raft wrapper (needs `term` back in WAL records)
- [ ] Object-store backend (SlateDB style)
- [ ] Design README with benchmark results and amplification plots

## Reading order
LevelDB `impl.md` + `table_format.md` -> RocksDB wiki (WAL, MANIFEST, write stalls) -> LSM paper, Monkey, Dostoevsky -> "Are You Sure You Want to Use MMAP in Your DBMS?" -> "All File Systems Are Not Created Equal" -> *Rust Atomics and Locks*. Full list in RFC 002 section 11.

## Progress log
| Week | Date | Done | Next |
|---|---|---|---|
| 1 | | | |

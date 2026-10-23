# RFC 002: Architecture for maximum systems-programming depth

Goal: every layer is something you build and measure yourself, down to syscalls, memory ordering and on-disk bytes. Each section says what to build, the trade-off, and what you learn.

## 1. Principles

1. **Own the I/O.** `tokio::fs` is `spawn_blocking` around `read`/`write`. It hides syscalls, buffering and fsync cost. The engine core is synchronous, thread-based, and talks to disk through an `Io` trait you write.
2. **Immutable snapshots, mutable edges.** Everything readers touch (memtables, SST list) lives in an immutable `Version` swapped atomically. Only the write path and background jobs mutate.
3. **Every byte on disk is specified.** Magic, version, checksum on every file, block and record. Write the format down in `docs/format.md` before coding it.
4. **Failure is a first-class input.** Test through a fault-injecting filesystem. Crash-consistency bugs are the main thing an LSM teaches.
5. **Measure before and after.** Write/read/space amplification counters exist from week 1, not week 8.

## 2. Runtime model: sync core, threads, channels

```
 clients ──► Db::put ──► [write queue] ──► WAL writer thread (group commit, fsync)
                                              │ ack via per-writer condvar/oneshot
                                              ▼
                                         memtable insert (concurrent skiplist)
 Db::get ──► load Arc<Version> (lock-free) ──► memtable → immutables → L0 → L1..Ln
 flush thread:      immutable memtable ─► SST ─► manifest edit ─► delete WAL
 compaction pool:   picker ─► k-way merge ─► new SSTs ─► manifest edit ─► delete inputs
```

- Threads: 1 WAL writer, 1 flush, N compaction. `std::thread` + `crossbeam-channel` (or write your own bounded MPSC later as an exercise).
- An optional `async` facade on top (`spawn_blocking` / oneshot) is fine for the public API. Do not put tokio inside the engine.
- Learn: thread lifecycle, `Send`/`Sync`, back-pressure, shutdown ordering, priority inversion.

## 3. Core types (fix these first, everything depends on them)

- `SeqNo = u64`, global, monotonic, assigned by the WAL writer. Replace the `u32 term/index` fields (keep `term` only if Raft is attempted).
- `InternalKey = user_key ++ (seqno << 8 | kind)` encoded big-endian-inverted so bytes sort `(user_key asc, seqno desc)`. `kind` = Put / Delete / (later) Merge, RangeDelete.
- Keys and values are `&[u8]` / `bytes::Bytes` in the API, not `String`. Values returned by reads are `Bytes` slices into cached blocks (zero copy, refcounted).
- One `Error` enum with `io::ErrorKind` preserved and a `Corruption { file, offset }` variant. No `Box<dyn Error>` across crate boundaries.

Seqnos plus internal keys give MVCC, snapshots, correct tombstone dropping and Raft hooks almost for free. Adding them later means rewriting every format.

## 4. Crate layout

```
common     types, InternalKey, varint, checksums, Error
io         trait Fs/File (pread, append, sync, rename, lock); impls: PosixFs, FaultFs, later UringFs
mem-table  arena + concurrent skiplist (keep BTree as a test oracle)
wal        block-framed log writer/reader, group commit
sst        block, builder, reader, bloom, index, footer
cache      sharded CLOCK/LRU block cache
manifest   VersionEdit log, Version, VersionSet, CURRENT
compaction picker, merge iterator, rate limiter
storage    Db: ties it all together, public API
bench      criterion + a db_bench-style workload driver
tools      `tecton-dump` (wal/sst/manifest inspector), `tecton-crash` (kill -9 harness)
```

Dependency rule: lower crates never depend on `storage`. `io` is the only crate that touches `std::fs`.

## 5. Component designs

### 5.1 `io` layer
```rust
trait Fs { fn open_read(..)->Box<dyn RandomAccess>; fn create(..)->Box<dyn Writable>;
           fn rename(..); fn sync_dir(..); fn list(..); fn lock(..); }
trait RandomAccess { fn read_at(&self, off: u64, buf: &mut [u8]) -> Result<usize>; }
trait Writable { fn append(&mut self, &[u8]); fn sync(&mut self); fn preallocate(&mut self, n: u64); }
```
- `PosixFs`: `pread`, `O_APPEND`, `fdatasync`, `fallocate` (Linux) / `F_PREALLOCATE` (macOS).
- **macOS trap:** `fsync` does not flush the drive cache. You need `fcntl(F_FULLFSYNC)`. Implement both and measure the difference (~100x).
- `FaultFs`: wraps another `Fs`; can drop unsynced writes at "crash", tear a write at byte N, return EIO/ENOSPC, flip bits. This is your crash-test engine.
- Later `UringFs`, `DirectFs` (`O_DIRECT`, aligned buffers, `posix_memalign`) behind the same trait, so experiments are benchmarks, not rewrites.
- Learn: page cache, dirty writeback, fsync semantics, directory fsync, alignment, short reads/writes, EINTR.

### 5.2 Memtable
- Lock-free (single-writer / multi-reader first, then multi-writer CAS) skiplist, nodes bump-allocated from an `Arena` (fixed 4 MiB chunks, freed all at once when the memtable drops).
- Node = height + key/value inline + `[AtomicPtr<Node>; h]`. Insert publishes with `Release`, readers traverse with `Acquire`.
- Memtable is `Arc`-shared; "immutable" is just "no more writers", no copy.
- Replace the `Vec<usize>` index skiplist (`skip_list_mem_table.rs`) once the arena version passes the BTree oracle in a property test.
- Learn: `unsafe`, `MaybeUninit`, atomics and memory ordering, ABA, false sharing (`#[repr(align(64))]`), `loom` model-checking, `miri`.

### 5.3 WAL
- LevelDB-style 32 KiB blocks, records fragmented as FULL/FIRST/MIDDLE/LAST with CRC32C. A torn block only loses the tail, and a reader can resync at the next block boundary.
- **Group commit:** writers push `(batch, waker)` to a queue. The first becomes leader, drains the queue, writes once, fsyncs once, wakes followers. This is the biggest write-throughput lever.
- Policies: `Sync` (fsync per group), `Periodic(ms)`, `None`. Document the exact durability promise of each.
- Recycle/preallocate segments to avoid metadata journaling on each append.
- Replay: stop at first bad CRC in the final segment (torn tail is expected), error on corruption in earlier segments.
- Learn: torn writes, write ordering, why `write` returning ≠ durable, CRC32C hardware instructions (`_mm_crc32_u64` / ARM `crc32c`).

### 5.4 SST format (v1, versioned)
```
[data block]* [filter block] [index block] [properties] [footer: handles, format_version, magic, crc]
data block: entries with prefix compression + restart points every 16 keys, trailer = compression_type + crc32c
```
- Reader: binary search restart points, then linear scan. Index block = last key per data block (shortest separator, not full key).
- Bloom: block-split / register-blocked bloom (one cache line per probe) with xxhash64 double hashing. Later partitioned filters.
- Compression: none → LZ4 → zstd, per block, skipped when it saves < 12%.
- Read path choices to benchmark against each other: `pread` + block cache, `mmap`, `O_DIRECT` + own cache, `io_uring` batched reads.
- Learn: binary formats, varint, endianness, SIMD bloom probes, cache-line layout, mmap page-fault cost and why databases (see "Are you sure you want to use mmap in your DBMS?") often avoid it.

### 5.5 Block cache
- Sharded (16–64) by hash, each shard a CLOCK ring with `Arc<Block>`; pinned blocks cannot be evicted. Charge by bytes, not entries.
- Iterators hold `Arc<Block>` so values are zero-copy slices.
- Then add a **row cache** or TinyLFU admission as an experiment.
- Learn: lock contention, sharding, CLOCK vs LRU, scan resistance, `Arc` refcount traffic on hot paths.

### 5.6 Manifest and versions
- `VersionEdit` = {add file(level, meta), delete file, set log number, set last seqno, next file number}. Appended to a manifest log (same record framing as the WAL) and fsynced before the edit takes effect.
- `CURRENT` points to the live manifest, updated by write-temp → fsync → rename → fsync-dir.
- `Version` = immutable per-level file lists. `VersionSet` applies edits and publishes the new `Arc<Version>` with `arc-swap`. Files are deleted only when no live `Version` references them (refcount via `Arc::drop`).
- Recovery: read CURRENT → replay manifest → replay WALs newer than `log_number` → delete orphan files.
- Learn: crash consistency, atomic rename, ordering of metadata vs data durability, reference-counted file lifetimes.

### 5.7 Compaction
- Start with the simplest correct thing: leveled, L0→L1 on file count, Ln→Ln+1 on size ratio (10x).
- Merge iterator over N sorted runs: `BinaryHeap` first, then a loser tree. Drop shadowed versions older than the oldest live snapshot. Drop tombstones only at the bottom level or when no older data overlaps.
- Write stalls: slowdown at L0 ≥ 8 files, stop at ≥ 12; token-bucket rate limiter on compaction bytes/s. Understand why stalls are a feature.
- Then size-tiered and a tunable "compaction strategy" trait to compare write amp vs read amp vs space amp on the same workload.
- Learn: amplification trade-offs (RUM conjecture), scheduling, I/O priority, backpressure.

### 5.8 Snapshots and iterators
- `Db::snapshot()` pins `(Arc<Version>, seqno)`. Reads filter `seqno <= snapshot`.
- Iterator = merging iterator over memtables + SSTs holding the pinned `Version`; supports forward, then reverse.
- Learn: lifetimes with self-referential-ish iterators (`Arc` owners rather than borrows), `Iterator` vs lending iterators.

## 6. Testing and verification ladder (this is where the depth is)

| Layer | Tool | What it proves |
|---|---|---|
| Unit | `cargo test` | formats round-trip |
| Model | `proptest` vs `BTreeMap` oracle, random op sequences incl. reopen | engine == model |
| Fuzz | `cargo-fuzz` on WAL/SST/manifest decoders | no panics, no UB on garbage |
| Concurrency | `loom` on skiplist + group commit; `miri` on all `unsafe` | no data races / UB |
| Crash | `FaultFs`: crash at every syscall boundary, torn writes, recover, compare to model | durability promise holds |
| Process | `tecton-crash`: `kill -9` loop with a verifier | real-world durability |
| Deterministic sim | single-threaded scheduler + `FaultFs` + seeded RNG (FoundationDB / TigerBeetle style) | reproducible concurrency + fault bugs |
| Perf | `criterion` micro, `db_bench` macro, `perf`/`flamegraph`/`samply`, `strace -c`, `bpftrace` | where time actually goes |

## 7. Observability
- Counters: bytes written to WAL / SST / compaction, bytes read, user bytes in → compute write amp, read amp, space amp live.
- Histograms (HDR) for put/get latency, fsync latency, stall time. Report p50/p99/p99.9, watch tail latency.
- `tracing` spans on flush and compaction.

## 8. Unsafe and low-level progression
1. Safe everything, correct baseline.
2. `unsafe` arena + skiplist (miri + loom clean).
3. SIMD bloom / CRC / varint decode (`std::arch`, runtime feature detection).
4. Aligned buffers + `O_DIRECT`.
5. `io_uring` (raw `io-uring` crate, then register buffers / fixed files, SQ polling).
6. Custom allocator experiments (`#[global_allocator]` jemalloc vs mimalloc; per-thread arenas).
7. Stretch: thread-per-core, shard keyspace, no shared locks (Seastar / Glommio style).

## 9. Process-level systems work
- Run under cgroup v2 limits (`memory.max`, `io.max`), observe stalls, OOM behavior, and tune memtable and cache budgets.
- Pin threads, test NUMA effects (Linux box or VM).
- Compare `ext4` vs `xfs` vs `tmpfs` behavior for fsync and preallocation.
- Memory budget: one `MemoryBudget` object accounting memtables + cache + compaction buffers.

## 10. Decisions and non-goals
- Keep tokio out of the engine core (decided). Revisit only for a network server crate.
- Pick one memtable (skiplist) for production; BTree remains as test oracle.
- Not in scope: SQL, secondary indexes, distributed transactions. Raft is a stretch goal after snapshots.

## 11. Recommended reading (in this order)
1. LevelDB `doc/impl.md` and `table_format.md`, then read `db_impl.cc`, `version_set.cc`.
2. RocksDB wiki: WAL, MANIFEST, Compaction, Write Stalls, Block Cache.
3. "The Log-Structured Merge-Tree" (O'Neil), "Monkey" (bloom allocation), "Dostoevsky" (lazy leveling).
4. "Are You Sure You Want to Use MMAP in Your DBMS?" (Crotty et al.).
5. "All File Systems Are Not Created Equal" (Pillai et al.) and Rebello et al. on fsync failure semantics.
6. SlateDB source for manifest and object-store variant; `sled` and `fjall` for Rust idioms.
7. *Rust Atomics and Locks* (Bos), for the memtable.

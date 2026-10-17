# Adversarial Challenge Report — Milestone 2: Storage IPC Commands

**Author**: Challenger 1 (Replacement)  
**Target File**: `apps/desktop/src-tauri/src/commands/storage.rs`  
**Verdict**: **APPROVE**  
**Date**: 2026-10-16  
**Working Directory**: `d:/HOCC++/pc-doctor/.agents/teamwork/challenger_m2_1_rep`

---

## 1. Observation

### 1.1 Direct Source Code Observations
- **File**: `apps/desktop/src-tauri/src/commands/storage.rs` (Lines 15-40):
  - `scan_cleanable_storage()` uses `tokio::task::spawn_blocking` to execute `scan_cleanable_storage_impl(&engine)`. Any thread panic is trapped by Tokio's `JoinError` and mapped into a structured `Result<StorageScanReport, String>`.
  - `clean_storage_targets(request: CleanRequest)` uses `tokio::task::spawn_blocking(move || ...)` delegating to `clean_storage_targets_impl(&engine, &request)`.
  - Pure synchronous counterparts `scan_cleanable_storage_impl` and `clean_storage_targets_impl` decouple execution from Tauri state for deterministic unit testing.
- **File**: `crates/pc-doctor-core/src/storage/purge.rs`:
  - Line 23-30: Iterates over requested `target_ids`. If a target ID is not in `TargetCatalog`, pushes `"Target ID '<id>' is unknown"` to `summary.errors` and skips further processing without panicking.
  - Line 33-36: Checks `if target.root_path.is_empty() || !root.exists()`. If missing, appends `target_id` to `summary.targets_processed` and skips gracefully with zero errors.
  - Line 39-45: Enforces `safety_guard.validate(root)`. Rejects system roots, drive roots, and personal user directories.
  - Line 47-52: Dry-run branch accumulates `summary.files_deleted += target.file_count` and `summary.bytes_freed += target.total_bytes` without calling `fs::remove_file` or `fs::remove_dir`.
  - Line 100-102: Enforces `safety_guard.validate_purge_child(target_root, &path)`. Stops deletion if path attempts directory traversal outside `target_root`.
  - Line 145: Only removes child subdirectories (`let _ = fs::remove_dir(&path)`), never `target_root`.
  - Line 153-159: Automatically detects read-only permissions on files and clears the read-only flag prior to deletion.
  - Line 181-192: `is_locked_error` traps Windows OS errors `32` (`ERROR_SHARING_VIOLATION`), `33` (`ERROR_LOCK_VIOLATION`), and `5` (`ERROR_ACCESS_DENIED`), as well as `io::ErrorKind::PermissionDenied`. Locked files increment `summary.files_skipped_locked += 1` and do not abort the clean operation or push to `summary.errors`.

### 1.2 Empirical Test Execution
Commands executed on host:
1. `cargo test -p desktop --test adversarial_storage_ipc_tests -- --nocapture`
   - Output: `test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.35s`
2. `cargo test --workspace`
   - Output:
     - `desktop_lib` unit tests: `26 passed; 0 failed`
     - `adversarial_process_ipc` integration tests: `10 passed; 0 failed`
     - `adversarial_storage_ipc_tests` integration tests: `12 passed; 0 failed`
     - `pc_doctor_core` unit tests: `27 passed; 0 failed`
     - `adversarial_process_tests` integration tests: `16 passed; 0 failed`
     - `adversarial_storage_tests` integration tests: `20 passed; 0 failed`
     - Total: **111 passed; 0 failed; 0 ignored; 100% pass rate**.
3. `npm run build` in `apps/desktop`:
   - Output: `tsc && vite build` succeeded in 223ms with 0 TypeScript errors.

---

## 2. Logic Chain

1. **Async Runtime Non-Blocking Invariant**:
   - Filesystem scans and deletions are synchronous I/O bound. If executed directly on the Tokio async worker thread, UI commands and Axum companion server requests would freeze.
   - `storage.rs` offloads all scanning and purging to `tokio::task::spawn_blocking`, preserving event-loop responsiveness and isolating thread panics.
2. **Deterministic Dry-Run Guarantee**:
   - `test_adv_dry_run_oracle_absolute_preservation` constructed 30 nested files of varying byte lengths (totalling exact known bytes).
   - In dry-run mode (`dry_run: true`), `clean_storage_targets_impl` returned exact expected file counts and byte counts.
   - Empirical disk inspection after dry-run verified that 100% of the files persisted with identical byte content and zero mutations.
3. **Graceful Handling of Locked & In-Use Files**:
   - On Windows, active applications hold files with exclusive share modes (denying delete access).
   - `test_adv_locked_files_and_open_handles` acquired an exclusive file lock (`share_mode(0)`).
   - During live purge, the purger caught OS error 32 without panicking, recorded `files_skipped_locked: 1`, deleted unlocked siblings (freeing their exact byte count), and left the locked file intact.
4. **Target Root Directory Preservation**:
   - `StoragePurger` ensures that the root folder of each cleanable target (e.g. `%TEMP%`, `CrashDumps`) is preserved on disk (`test_adv_live_clean_deletes_files_and_cleans_subdirs_while_preserving_root`).
   - Only empty child subdirectories are pruned. If a subdirectory contains a locked file, the subdirectory is also preserved (`test_adv_purge_subdir_with_locked_file_not_deleted`).
5. **Edge Case Robustness**:
   - Empty requests return zero counts and empty error lists (`test_adv_empty_target_ids_clean_request`).
   - Hostile target IDs (`../../etc/passwd`, `C:\Windows\System32`, `\0nullbyte`) are safely reported in `summary.errors` without crashing (`test_adv_unknown_and_malicious_target_ids`).
   - Non-existent target directories are marked processed without error (`test_adv_nonexistent_directory_handling`).
   - Hostile targets pointing to protected OS directories (`C:\Windows\System32`) trigger safety violations and prevent deletion (`test_adv_boundary_violation_in_clean_request_blocked`).
6. **Concurrency Resilience**:
   - 20 concurrent IPC invocations (10 async scans + 10 async dry-run cleans) executed simultaneously across worker threads without deadlocks, data races, or failures (`test_adv_concurrent_ipc_calls`).

---

## 3. Caveats

- **Operating System Environment**: Windows-specific error codes (32, 33, 5) were verified directly on the Windows host. On non-Windows platforms, fallback error handling relies on `ErrorKind::PermissionDenied`.
- **System Admin Privileges**: PC Doctor targets are strictly user-level safe temporary caches. The tests were run under standard developer privileges, verifying that standard user operations succeed and restricted OS boundaries remain impassable.

---

## 4. Conclusion

**Verdict**: **APPROVE**

The Storage IPC implementation in `apps/desktop/src-tauri/src/commands/storage.rs`:
1. Satisfies all requirements from `ORIGINAL_REQUEST.md` (R2) and interface contracts in `PROJECT.md`.
2. Fully guarantees dry-run non-mutation with exact byte counts.
3. Deletes targets cleanly while preserving root directory boundaries.
4. Correctly catches Windows sharing violations (error 32) without panicking.
5. Blocks malicious target IDs and boundary escapes.
6. Operates safely under concurrent load across Tokio worker threads.

---

## 5. Verification Method

To independently verify this assessment, execute the following commands in powershell:

```powershell
# 1. Verify desktop crate storage & process IPC tests
cargo test -p desktop -- --nocapture

# 2. Verify adversarial storage IPC integration tests
cargo test -p desktop --test adversarial_storage_ipc_tests -- --nocapture

# 3. Verify entire workspace suite (111 tests)
cargo test --workspace

# 4. Verify frontend bundle
cd apps/desktop
npm run build
```

Expected result: All tests pass with 0 failures, 0 unhandled panics, and 0 TypeScript build errors.

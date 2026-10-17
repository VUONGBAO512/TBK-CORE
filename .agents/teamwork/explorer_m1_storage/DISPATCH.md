## 2026-10-16T07:05:05Z
You are an Explorer for Milestone 1 Storage Engine in PC Doctor (TBK Core).
Your working directory is d:/HOCC++/pc-doctor/.agents/teamwork/explorer_m1_storage.
You MUST read the original request before starting work:
Path to original request: d:/HOCC++/pc-doctor/.agents/teamwork/ORIGINAL_REQUEST.md
Path to project plan: d:/HOCC++/pc-doctor/.agents/teamwork/PROJECT.md

Task:
Analyze and design the exact implementation for `pc-doctor-core::storage`:
1. Storage target definitions and dynamic path resolution for Windows (%TEMP%, CrashDumps, WER, Explorer thumbnails, application logs).
2. Strict safety boundary enforcement: safe root whitelist, canonicalization, rejection of forbidden system folders (%WINDIR%, System32, Desktop, Documents, .. traversal).
3. Scanning engine computing exact non-negative byte sizes and file counts.
4. Purge engine with dry-run support, locked/in-use file skip handling without panics (catching sharing violations/access denied), and detailed summary generation.
5. Unit test suite specifications to guarantee full coverage of all safety rules and edge cases.
Write your complete analysis and implementation blueprint to d:/HOCC++/pc-doctor/.agents/teamwork/explorer_m1_storage/handoff.md. Update progress.md with your liveness.
Send a message to your caller upon completion with the path to your handoff report.

# Bug Analysis: Pyenv Rehash Lock File Persistence

This document analyzes the `lock already exists` rehash failure observed during automated tooling and CLI invocations, detailing the root cause, immediate workarounds, and proposed structural code fixes in `pyenv-native`.

---

## 1. Observed Error

During CLI operations or background script compilation, the following error prints repeatedly:

```text
pyenv: cannot rehash: lock C:\Users\Roy\.pyenv\shims\.pyenv-shims.lock already exists
```

This error blocks any command requiring a shim rehash (such as setting versions, creating virtual environments, or running shell hooks) until the lock is released or manually deleted.

---

## 2. Root Cause Analysis

The core of the issue lies in the locking implementation inside the `pyenv-core` crate:

* **File Paths**:
  - Lock File: `C:\Users\Roy\.pyenv\shims\.pyenv-shims.lock`
  - Source Location: [rehash.rs](file:///C:/Users/Roy/Desktop/AI/pyenv/pyenv-native/crates/pyenv-core/src/shim/rehash.rs) and [types.rs](file:///C:/Users/Roy/Desktop/AI/pyenv/pyenv-native/crates/pyenv-core/src/shim/types.rs).

* **How the Lock Works**:
  1. `rehash_shims` calls `acquire_rehash_lock(&shims_dir)` at the start of execution.
  2. `acquire_rehash_lock` creates `.pyenv-shims.lock` using `OpenOptions::new().create_new(true)`.
  3. On success, it returns a `RehashLockGuard` which implements `Drop`.
  4. The `Drop::drop` implementation deletes the lock file:
     ```rust
     impl Drop for RehashLockGuard {
         fn drop(&mut self) {
             let _ = fs::remove_file(&self.path);
         }
     }
     ```

* **The Problem (Orphaned Lock Files)**:
  - If a `pyenv` process is **terminated abruptly** (e.g., via Ctrl+C, process kill, IDE task cancellation, or a system panic), the `drop()` destructor is **never executed** by the operating system.
  - The lock file remains in the filesystem indefinitely.
  - **Staleness Check**: The lock is only considered stale if it has been there for more than `SHIM_LOCK_STALE_SECS` (currently set to **10 minutes** / `60 * 10` seconds in `types.rs`):
    ```rust
    pub(super) const SHIM_LOCK_STALE_SECS: u64 = 60 * 10;
    ```
  - For up to **10 minutes** after a crash or cancellation, any subsequent `pyenv` invocation fails with the locking error, causing massive disruptions in automated environments or fast developer inner-loops.

---

## 3. Immediate Workarounds

If you hit this error, you can immediately recover by manually removing the lock file:

### PowerShell
```powershell
Remove-Item "$env:USERPROFILE\.pyenv\shims\.pyenv-shims.lock" -ErrorAction SilentlyContinue
```

### Bash / Zsh
```bash
rm -f ~/.pyenv/shims/.pyenv-shims.lock
```

---

## 4. Proposed Structural Bugfixes (For `pyenv-native`)

To prevent this issue from recurring and to make the native version manager extremely resilient, we should implement one of the following strategies in [rehash.rs](file:///C:/Users/Roy/Desktop/AI/pyenv/pyenv-native/crates/pyenv-core/src/shim/rehash.rs):

### Approach A: Active PID Validation (Recommended)
Since the lock file payloads already contain the owner's Process ID (`pid=...`), we can check if that process is still actively running on the OS. If the process is dead, the lock file can be immediately removed without waiting 10 minutes.

* **Implementation Logic**:
  ```rust
  fn lock_file_is_stale(path: &Path) -> bool {
      let Ok(contents) = fs::read_to_string(path) else {
          return false;
      };
      
      // Parse the owning PID from the lock file
      let pid = contents
          .lines()
          .find_map(|line| line.strip_prefix("pid="))
          .and_then(|value| value.parse::<u32>().ok());

      if let Some(pid) = pid {
          // If the process is no longer running, the lock is immediately stale/orphaned
          if !process_exists(pid) {
              return true;
          }
      }

      // Fallback to standard time-based staleness check
      let Some(created_at) = contents
          .lines()
          .find_map(|line| line.strip_prefix("created_at="))
          .and_then(|value| value.parse::<u64>().ok())
      else {
          return false;
      };
      
      let now = SystemTime::now()
          .duration_since(UNIX_EPOCH)
          .unwrap_or_default()
          .as_secs();
      now.saturating_sub(created_at) > SHIM_LOCK_STALE_SECS
  }

  // Simple cross-platform check for process existence in Rust
  fn process_exists(pid: u32) -> bool {
      #[cfg(unix)]
      {
          // On Unix, sending signal 0 checks if a process exists
          unsafe { libc::kill(pid as libc::pid_t, 0) == 0 }
      }
      #[cfg(windows)]
      {
          use windows_sys::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};
          use windows_sys::Win32::Foundation::CloseHandle;
          
          let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
          if handle != 0 {
              unsafe { CloseHandle(handle) };
              true
          } else {
              false
          }
      }
  }
  ```

### Approach B: Significantly Shorten Stale Time
In standard CLI workflows, a `rehash` completes in under 1 second (writing shims is very fast). Keeping a lock file active for 10 minutes is excessively conservative.
* **Suggested Fix**: Reduce `SHIM_LOCK_STALE_SECS` in `types.rs` from **10 minutes** to **10 seconds** or **5 seconds**.
  ```rust
  pub(super) const SHIM_LOCK_STALE_SECS: u64 = 5;
  ```
  This guarantees that any orphaned lock file naturally expires after only 5 seconds, allowing subsequent shell triggers or scripts to recover automatically.

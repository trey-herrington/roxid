# Before vs After: Parallel Jobs Display

## The Problem

When running parallel jobs, the TUI previously showed all output in a single interleaved stream:

```
🎭 Stage: build (#1) started
  🔧 Job: build_windows (#1) started
      [Step 1/2] Echo platform
  🔧 Job: build_linux (#1) started
      [Step 1/2] Echo platform
        Building on Windows
        Building on Linux
        ✓ Completed in 0.00s
        ✓ Completed in 0.00s
      [Step 2/2] Show time
      [Step 2/2] Show date
        00:16:58
        Thu Nov  6 12:16:58 AM CST 2025
        ✓ Completed in 0.00s
        ✓ Completed in 0.00s
```

**Issues:**
- Hard to tell which output belongs to which job
- Steps from different jobs are mixed together
- Difficult to track individual job progress
- Confusing when jobs have similar output

## The Solution

Now when jobs run in parallel, each gets its own column:

```
┌───────────────────────────────────────────────────────────────┐
│ Progress: [████████████████████] Step 4/7                     │
├─────────────────────────────┬─────────────────────────────────┤
│ Job: build_linux            │ Job: build_windows              │
│                             │                                 │
│ 🔧 Job: build_linux started │ 🔧 Job: build_windows started   │
│ [Step 1/2] Echo platform    │ [Step 1/2] Echo platform        │
│   Building on Linux         │   Building on Windows           │
│   ✓ Completed in 0.00s      │   ✓ Completed in 0.00s          │
│ [Step 2/2] Show date        │ [Step 2/2] Show time            │
│   Thu Nov 6 12:16:58...     │   00:16:58                      │
│   ✓ Completed in 0.00s      │   ✓ Completed in 0.00s          │
│ ✓ Job completed (2 steps)   │ ✓ Job completed (2 steps)       │
└─────────────────────────────┴─────────────────────────────────┘
```

**Benefits:**
✅ Each job's output is isolated and clear  
✅ Easy to track individual job progress  
✅ See all parallel jobs at once  
✅ No confusion about which output is from which job  

## Three Jobs Running in Parallel

```
┌─────────────────────────────────────────────────────────────────┐
│ Progress: [████████████] Step 6/7                               │
├───────────────────┬───────────────────┬─────────────────────────┤
│ Job: job1         │ Job: job2         │ Job: job3               │
│                   │                   │                         │
│ 🔧 Job started    │ 🔧 Job started    │ 🔧 Job started          │
│ [Step 1/2] ...    │ [Step 1/2] ...    │ [Step 1/2] ...          │
│   Job 1 exec...   │   Job 2 exec...   │   Job 3 exec...         │
│   ✓ Completed     │   ✓ Completed     │   ✓ Completed           │
│ [Step 2/2] ...    │ [Step 2/2] ...    │ [Step 2/2] ...          │
│   Job 1 complete  │   Job 2 complete  │   Job 3 complete        │
│   ✓ Completed     │   ✓ Completed     │   ✓ Completed           │
│ ✓ Job done        │ ✓ Job done        │ ✓ Job done              │
└───────────────────┴───────────────────┴─────────────────────────┘
```

## Automatic Behavior

The TUI automatically switches between layouts:

- **1 or 0 active jobs**: Full-width single pane (traditional view)
- **2+ active jobs**: Split into N equal columns (one per job)

No configuration needed - it just works!

## Try It Yourself

```bash
# Build the project
cargo build --release

# Start the TUI
./target/release/roxid-tui

# Navigate to "test-stages-pipeline" and press Enter
# Watch the split-view in action during the build and test stages!
```

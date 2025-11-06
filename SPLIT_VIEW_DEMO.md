# Split-View TUI Demo

## What Changed

The TUI now automatically displays parallel jobs side-by-side when multiple jobs run simultaneously. This replaces the previous behavior where all job outputs were interleaved in a single pane.

## How It Works

### Detection
- The TUI tracks `active_jobs` - a list of currently running jobs
- When `active_jobs.len() > 1`, the UI switches to split-pane mode

### Layout
- The output area is split horizontally into equal-width columns
- Each column shows one job's output with its own border and title
- When jobs complete and fall back to 0-1 active jobs, the view returns to single-pane

### Example Flow

**Stage with 2 Parallel Jobs:**
```
┌─────────────────────────────────────────────────────┐
│ Progress: [████████████████] Step 4/7               │
├──────────────────┬──────────────────────────────────┤
│ Job: build_linux │ Job: build_windows               │
│                  │                                  │
│ [Step 1/2]...    │ [Step 1/2]...                    │
│   Building...    │   Building...                    │
│ ✓ Completed      │   Building on Windows            │
│ [Step 2/2]...    │ ✓ Completed                      │
│   Thu Nov 6...   │ [Step 2/2]...                    │
│                  │   00:23:08                       │
└──────────────────┴──────────────────────────────────┘
```

**Stage with 3 Parallel Jobs:**
```
┌──────────────────────────────────────────────────────┐
│ Progress: [████████] Step 4/7                        │
├──────────┬─────────────┬─────────────────────────────┤
│ Job: j1  │ Job: j2     │ Job: j3                     │
│          │             │                             │
│ [Step..] │ [Step...]   │ [Step...]                   │
│ Output 1 │ Output 2    │ Output 3                    │
└──────────┴─────────────┴─────────────────────────────┘
```

## Testing

### With test-stages-pipeline.yaml:
```bash
cargo run --release --bin roxid-tui
# Select "test-stages-pipeline"
# Press Enter to execute
```

Watch for:
1. Stage 1: `build_linux` and `build_windows` side-by-side
2. Stage 2: `unit_tests` and `integration_tests` side-by-side

### With test-parallel-jobs.yaml:
```bash  
cargo run --release --bin roxid-tui
# Select "parallel-jobs-pipeline"
# Press Enter to execute
```

Watch for:
1. Stage 1: Three jobs (`job1`, `job2`, `job3`) in a 3-column layout
2. Stage 2: Single job `summary` in full-width view

## Code Changes

### `roxid-tui/src/app.rs`
- Added `active_jobs: Vec<String>` to track running jobs
- Added `job_outputs: HashMap<String, Vec<String>>` for per-job output buffers
- Modified event handlers to:
  - Add jobs to `active_jobs` on `JobStarted`
  - Route output to job-specific buffers
  - Remove jobs from `active_jobs` on `JobCompleted`

### `roxid-tui/src/ui/components.rs`
- Modified `render_execution_view()` to detect parallel jobs
- Added `render_parallel_jobs()` to create horizontal split layout
- Added `render_job_output()` to render individual job panes
- Extracted `format_output_line()` for consistent formatting

## Benefits

✅ Clear visualization of parallel execution  
✅ No more confusing interleaved output  
✅ Real-time monitoring of all parallel jobs  
✅ Automatic layout adaptation  
✅ Maintains full sequential log in background  

## Implementation Notes

- Uses Ratatui's `Layout` with `Percentage` constraints for dynamic splitting
- Each job pane independently scrolls as output grows
- Color formatting and icons are preserved in split view
- The main `output_lines` buffer still contains the complete sequential log

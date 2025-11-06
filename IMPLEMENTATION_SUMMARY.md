# Parallel Jobs Split-View Implementation Summary

## Problem
The TUI was not properly displaying per-job steps when jobs ran in parallel. All output was interleaved in a single pane, making it difficult to track individual job progress.

## Solution
Implemented an automatic split-view layout that displays parallel jobs side-by-side in separate panes.

## Changes Made

### 1. Enhanced State Management (`roxid-tui/src/app.rs`)

Added two new fields to `ExecutionState`:
- `active_jobs: Vec<String>` - Tracks currently running jobs
- `job_outputs: HashMap<String, Vec<String>>` - Stores per-job output buffers

Modified event handlers:
- **JobStarted**: Adds job to `active_jobs` and creates output buffer
- **StepStarted/StepOutput/StepCompleted**: Routes output to both global buffer and job-specific buffer
- **JobCompleted**: Removes job from `active_jobs` list

### 2. Dynamic UI Layout (`roxid-tui/src/ui/components.rs`)

Refactored `render_execution_view()`:
- Checks `active_jobs.len()` to determine layout mode
- Calls `render_parallel_jobs()` when multiple jobs are active
- Falls back to `render_single_output()` for single/sequential jobs

Added new functions:
- `render_parallel_jobs()`: Creates horizontal split layout with equal-width columns
- `render_job_output()`: Renders individual job pane with border and title
- `format_output_line()`: Extracted formatting logic for reuse

### 3. Visual Design

Each job pane features:
- Blue border for visual distinction
- Job name in the title bar
- Independent scrolling as output grows
- Preserved color formatting (✓/✗, bold headers, etc.)

Layout adapts dynamically:
- 2 jobs → 50/50 split
- 3 jobs → 33/33/33 split
- N jobs → evenly divided columns

## Testing

### Test Pipelines
1. **test-stages-pipeline.yaml**: 2 parallel jobs per stage
2. **test-parallel-jobs.yaml**: 3 parallel jobs in first stage

### Verification
```bash
# Build
cargo build --release

# Test with CLI (shows parallel execution)
./target/release/roxid run test-stages-pipeline.yaml

# Test with TUI (shows split-view)
./target/release/roxid-tui
# Select pipeline and press Enter
```

## Benefits

✅ **Clear Visualization**: Each job's output is isolated in its own pane  
✅ **Real-time Monitoring**: See all parallel jobs simultaneously  
✅ **Automatic Adaptation**: UI adjusts to number of parallel jobs  
✅ **Maintains History**: Global output buffer preserves complete log  
✅ **No Configuration**: Works out-of-the-box with existing pipelines  

## Technical Details

- Uses Ratatui's `Layout` with `Direction::Horizontal` and `Percentage` constraints
- Job panes dynamically calculated: `100 / num_jobs` percentage per column
- When jobs complete at different times, remaining jobs maintain their panes until < 2 active
- All output routing is non-blocking and handled in event processing loop

## Files Modified

1. `roxid-tui/src/app.rs` - State management and event handling
2. `roxid-tui/src/ui/components.rs` - UI rendering logic

## Documentation Added

1. `PARALLEL_JOBS_UI.md` - Detailed feature documentation
2. `SPLIT_VIEW_DEMO.md` - Visual demo and usage guide
3. `IMPLEMENTATION_SUMMARY.md` - This file

## Future Enhancements

Potential improvements:
- Configurable max columns with row wrapping
- Per-job progress indicators
- Color-coded borders based on job status
- Ability to focus/expand individual job pane
- Horizontal scrolling for narrow columns

# Parallel Jobs Side-by-Side UI Feature

## Overview

The TUI now displays parallel job execution side-by-side in split panes when multiple jobs are running simultaneously. This provides a much better visual representation of concurrent execution and makes it easier to monitor multiple jobs at once.

## Implementation Details

### Key Changes

1. **Enhanced ExecutionState** (`roxid-tui/src/app.rs`):
   - Added `active_jobs: Vec<String>` - Tracks which jobs are currently running
   - Added `job_outputs: HashMap<String, Vec<String>>` - Stores output lines per job

2. **Split-View Rendering** (`roxid-tui/src/ui/components.rs`):
   - When `active_jobs.len() > 1`, the UI automatically switches to horizontal split-pane layout
   - Each job gets its own bordered pane with job name in the title
   - Equal-width columns are created dynamically based on the number of parallel jobs
   - Each pane shows only that job's output (steps, output, completion status)

3. **Event Processing**:
   - `JobStarted`: Job is added to `active_jobs` list and a new output buffer is created
   - `StepStarted`: Step information is added to both the global output and the job-specific buffer
   - `StepOutput`: Command output is routed to the appropriate job's buffer
   - `StepCompleted`: Completion status is added to both buffers
   - `JobCompleted`: Job is removed from `active_jobs` list, triggering return to single-pane view

### Layout Behavior

- **Single Job or Sequential**: Traditional full-width output pane
- **2 Parallel Jobs**: Screen splits 50/50 vertically
- **3 Parallel Jobs**: Screen splits 33/33/33 vertically
- **N Parallel Jobs**: Screen splits evenly into N columns

### Visual Features

- Each job pane has a blue border with the job name in the title
- All the standard formatting still applies (colors for success/failure, bold for headers, etc.)
- Progress bar at the top tracks overall pipeline progress
- When jobs complete and drop below 2 active jobs, the view returns to single-pane

## Example Pipelines

### Test with Stages Pipeline

```bash
cargo run --release --bin roxid-tui
# Select "test-stages-pipeline"
# Press Enter
```

This pipeline has two stages:
1. **Build Stage**: `build_linux` and `build_windows` run in parallel
2. **Test Stage**: `unit_tests` and `integration_tests` run in parallel

When each stage executes, you'll see the jobs displayed side-by-side.

### Parallel Jobs Pipeline

```bash
cargo run --release --bin roxid-tui
# Select "parallel-jobs-pipeline"
# Press Enter
```

This pipeline has:
1. **Parallel Work Stage**: 3 jobs (`job1`, `job2`, `job3`) running simultaneously
2. **Final Stage**: Single summary job

You'll see a 3-way split during the first stage, then return to single-pane for the final stage.

## Benefits

1. **Better Visualization**: Clear view of which jobs are running and their individual progress
2. **Reduced Confusion**: No more interleaved output from parallel jobs
3. **Real-time Monitoring**: See all parallel jobs at once without scrolling
4. **Automatic Layout**: No configuration needed - the UI adapts to the number of parallel jobs
5. **Maintains Full Log**: The global output_lines still contains the complete sequential log for reference

## Technical Notes

- The split-view uses Ratatui's `Layout` with `Direction::Horizontal` and `Percentage` constraints
- Job outputs are collected separately but also added to the main output buffer for completeness
- The current implementation handles up to ~10 parallel jobs reasonably (beyond that, columns get narrow)
- When jobs complete at different times, the UI dynamically adjusts (removes completed jobs from split view)

## Future Enhancements

Possible improvements:
- Scrollable job panes for long output
- Configurable max columns (e.g., max 3 columns, then wrap to multiple rows)
- Job status indicators (progress per job)
- Color-coded borders based on job status (green=success, red=failed, blue=running)
- Ability to focus on one job and expand it to full width

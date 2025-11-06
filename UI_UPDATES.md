# UI Updates for Azure DevOps Pipeline Support

## Date: November 6, 2025

## Summary

Updated the UI (both TUI and CLI) to properly display and track the new stages and jobs structure added in Phase 1 of Azure DevOps pipeline support.

## Changes Made

### 1. Protocol Buffers (proto) Updates

**File: `pipeline-service/proto/pipeline.proto`**
- Added `Stage` and `Job` message types
- Added `StageStarted`, `StageCompleted`, `JobStarted`, `JobCompleted` events
- Added `StageResult` and `JobResult` types with status enums
- Updated `Pipeline` message to include both `steps` (legacy) and `stages` (new format)
- Both formats are now fully supported at the protocol level

### 2. gRPC Conversion Layer

**File: `pipeline-service/src/grpc.rs`**
- Added bidirectional conversions for `Stage`, `Job`, `StageResult`, `JobResult`
- Added status enum conversions for `StageStatus` and `JobStatus`
- **Removed flattening logic** - both formats now preserve their structure
- Proto now accurately represents whether a pipeline uses stages/jobs or just steps

### 3. TUI Updates

**File: `roxid-tui/src/app.rs`**
- Added `current_stage` and `current_job` fields to `ExecutionState`
- Updated event processing to handle `StageStarted`, `StageCompleted`, `JobStarted`, `JobCompleted`
- Added hierarchical output with indentation:
  - 🎭 Stages at root level
  - 🔧 Jobs indented under stages
  - Steps indented under jobs
- Updated step count logic to handle both formats

**File: `roxid-tui/src/ui.rs`**
- Updated header to show current stage and job: `Executing: pipeline | Stage: build | Job: build_linux`

**File: `roxid-tui/src/ui/components.rs`**
- Added color coding:
  - Magenta + bold for stage markers (🎭)
  - Blue + bold for job markers (🔧)
  - Yellow + bold for step markers
  - Green for success (✓)
  - Red for failures (✗)

### 4. CLI Updates

**File: `roxid-cli/src/main.rs`**
- Added handling for stage and job events
- Updated output to show hierarchical structure with proper indentation
- Added stage/job count in pipeline summary
- Shows both formats correctly (stages or legacy steps)

### 5. Proto File Synchronization

**File: `roxid-cli/proto/pipeline.proto`**
- Synchronized with main proto definition in pipeline-service
- Ensures CLI and service use identical protocol definitions

## Visual Output Examples

### Stages Pipeline with Parallel Jobs
```
🎭 Stage #1: build started
  🔧 Job #1: build_windows started
  🔧 Job #1: build_linux started
      [Step 1/...] Running: Echo platform
        | Building on Windows
        | Building on Linux
      [Step 1/...] Echo platform - Success (1ms, exit code: Some(0))
     Job 'build_linux' - Success (3ms, 2 steps)
     Job 'build_windows' - Success (3ms, 2 steps)
   Stage 'build' - Success (3ms, 2 jobs)
```

### Legacy Format (Backward Compatible)
```
🎭 Stage #1: default started
  🔧 Job #1: default started
      [Step 1/...] Running: Step 1
        | This is the old format
      [Step 1/...] Step 1 - Success (1ms, exit code: Some(0))
     Job 'default' - Success (5ms, 3 steps)
   Stage 'default' - Success (5ms, 1 jobs)
```

## Key Features

### ✅ Dual Format Support
- **Legacy format**: Simple pipelines with just `steps`
- **Stages format**: Full Azure DevOps format with `stages` → `jobs` → `steps`
- Both formats work seamlessly without conversion

### ✅ Visual Hierarchy
- Clear indentation shows relationship between stages, jobs, and steps
- Emojis provide quick visual identification
- Color coding highlights status and type

### ✅ Real-time Progress
- TUI header shows current stage and job
- Events stream in real-time as pipeline executes
- Progress bar tracks overall completion

### ✅ Parallel Execution Visibility
- Multiple jobs starting simultaneously are clearly visible
- Interleaved output shows true parallel execution
- Completion times demonstrate concurrency

## Backward Compatibility

### ✅ Fully Maintained
- Old pipelines with direct `steps` continue to work
- Automatically wrapped in default stage/job internally
- No breaking changes to existing pipelines
- gRPC API supports both formats

## Testing

All test pipelines verified:
- ✅ `test-stages-pipeline.yaml` - Multi-stage with dependencies
- ✅ `test-parallel-jobs.yaml` - Parallel job execution
- ✅ `test-legacy-pipeline.yaml` - Backward compatibility

## Architecture Benefits

1. **No Data Loss**: Proto now accurately represents both pipeline formats
2. **Clean Separation**: UI can distinguish between legacy and modern formats
3. **Extensibility**: Easy to add more Azure DevOps features (conditions, matrix, etc.)
4. **Performance**: Parallel execution is visible and measurable
5. **User Experience**: Clear visual hierarchy makes complex pipelines easy to understand

## Next Steps

With the UI properly reflecting stages and jobs, the foundation is complete for:
- Phase 2: Variables & Conditions
- Phase 3: Tasks
- Phase 4: Matrix strategies, containers
- Phase 5: Azure-specific task implementations

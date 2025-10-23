# Rust TUI Application Architecture

This workspace follows a typical Rust project structure with separation of concerns.

## Project Structure

```
rust-tui-app/
├── Cargo.toml              # Workspace manifest
├── tui/                    # Terminal UI application (binary)
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs         # Entry point
│       ├── app.rs          # Application state and logic
│       ├── events.rs       # Event handling (keyboard, mouse)
│       └── ui/             # UI rendering modules
│           ├── mod.rs
│           ├── components.rs  # Reusable UI components
│           └── layout.rs      # Layout definitions
├── service/                # Business logic layer (library)
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs          # Library entry point
│       ├── error.rs        # Error types
│       ├── models/         # Data models
│       │   ├── mod.rs
│       │   └── user.rs
│       └── services/       # Business logic
│           ├── mod.rs
│           └── user_service.rs
└── rpc/                    # RPC API layer (library)
    ├── Cargo.toml
    └── src/
        ├── lib.rs          # Library entry point
        ├── error.rs        # RPC-specific errors
        ├── api.rs          # RPC server setup
        └── handlers/       # RPC request handlers
            ├── mod.rs
            └── user_handler.rs
```

## Architecture Layers

### 1. **TUI Package** (`tui/`)
- **Purpose**: User interface layer
- **Type**: Binary crate (executable)
- **Dependencies**: Depends on `service` for business logic
- **Structure**:
  - `main.rs`: Application entry point
  - `app.rs`: Application state management
  - `events.rs`: User input handling
  - `ui/`: All rendering logic separated by concern

### 2. **Service Package** (`service/`)
- **Purpose**: Core business logic
- **Type**: Library crate
- **Dependencies**: None (pure business logic)
- **Structure**:
  - `models/`: Data structures and domain models
  - `services/`: Business logic implementations
  - `error.rs`: Domain-specific error types

### 3. **RPC Package** (`rpc/`)
- **Purpose**: Remote procedure call API
- **Type**: Library crate
- **Dependencies**: Depends on `service`
- **Structure**:
  - `api.rs`: RPC server setup
  - `handlers/`: Request handlers that call service layer
  - `error.rs`: RPC-specific error handling

## Module System

### Declaring Modules

Rust requires explicit module declaration. Two approaches:

**Approach 1: File-based (modern, preferred)**
```rust
// In lib.rs or main.rs
mod models;  // Looks for models.rs or models/mod.rs
mod services;

// Then use
use crate::models::User;
```

**Approach 2: Directory-based**
```
src/
├── lib.rs
└── models/
    ├── mod.rs    # Declares the module structure
    ├── user.rs
    └── product.rs
```

In `models/mod.rs`:
```rust
pub mod user;
pub mod product;

// Re-export commonly used items
pub use user::User;
pub use product::Product;
```

### Visibility Rules

- **Private by default**: All items are private unless marked `pub`
- **Module tree**: Modules form a tree starting from `lib.rs` or `main.rs`
- **Re-exports**: Use `pub use` to re-export items for cleaner APIs

Example:
```rust
// In lib.rs
pub mod models;    // Module is public
pub use models::User;  // Re-export User at crate root

// Now users can do:
use service::User;  // Instead of service::models::User
```

## Building and Running

```bash
# Build entire workspace
cargo build

# Build specific package
cargo build -p tui
cargo build -p service
cargo build -p rpc

# Run the TUI application
cargo run --bin tui

# Run tests for all packages
cargo test

# Run tests for specific package
cargo test -p rpc

# Check code without building
cargo check
```

## Adding New Modules

1. **Create the file**: `src/module_name.rs` or `src/module_name/mod.rs`
2. **Declare in parent**: Add `mod module_name;` in `lib.rs` or parent module
3. **Make public if needed**: Use `pub mod module_name;`
4. **Import where needed**: Use `use crate::module_name::Item;`

## Common Patterns

### Error Handling
Each layer has its own error types that convert between layers:
- `ServiceError` → `RpcError` (using `From` trait)
- Result type aliases: `ServiceResult<T>`, `RpcResult<T>`

### Dependency Direction
```
tui → service
rpc → service
```
Service has no dependencies on other packages (keeps business logic pure)

## Benefits of This Structure

1. **Separation of Concerns**: Each package has a single responsibility
2. **Reusability**: Service logic can be used by both TUI and RPC
3. **Testability**: Each layer can be tested independently
4. **Maintainability**: Clear boundaries make code easier to understand
5. **Scalability**: Easy to add new interfaces (web, CLI) without touching core logic

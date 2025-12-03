# asyncwrap

> Auto-generate async wrappers for blocking code via proc macros

## The Problem

When wrapping blocking/FFI code for async Rust, you end up writing tedious boilerplate:

```rust
// You write this blocking implementation once...
impl BlockingClient {
    pub fn get_data(&self) -> Result<Data, Error> { /* ... */ }
    pub fn send_command(&self, cmd: Command) -> Result<(), Error> { /* ... */ }
    // ... 20 more methods
}

// ...then duplicate it all for async:
impl AsyncClient {
    pub async fn get_data(&self) -> Result<Data, Error> {
        let inner = Arc::clone(&self.inner);
        tokio::task::spawn_blocking(move || inner.get_data()).await?
    }
    pub async fn send_command(&self, cmd: Command) -> Result<(), Error> {
        let inner = Arc::clone(&self.inner);
        tokio::task::spawn_blocking(move || inner.send_command(cmd)).await?
    }
    // ... 20 more identical wrappers 😫
}
```

## The Solution

With `asyncwrap`, you write:

```rust
use asyncwrap::blocking_impl;
use std::sync::Arc;

#[blocking_impl(AsyncClient)]
impl BlockingClient {
    #[async_wrap]
    pub fn get_data(&self) -> Result<Data, Error> { /* ... */ }
    
    #[async_wrap]
    pub fn send_command(&self, cmd: Command) -> Result<(), Error> { /* ... */ }
}

pub struct AsyncClient {
    inner: Arc<BlockingClient>,
}

// That's it! AsyncClient now has async versions of all marked methods.
```

## Installation

```sh
cargo add asyncwrap
```

## How It Works

1. `#[blocking_impl(AsyncType)]` processes your impl block
2. `#[async_wrap]` marks which methods should get async wrappers
3. The macro generates an `impl AsyncType` with async versions

## Strategies

Choose how blocking code is executed with the `strategy` parameter:

### `spawn_blocking` (default)

```rust
#[blocking_impl(AsyncClient)]  // or explicitly: strategy = "spawn_blocking"
impl BlockingClient { /* ... */ }

pub struct AsyncClient {
    inner: Arc<BlockingClient>,  // Arc required
}
```

- Runs blocking code on a dedicated thread pool
- Arguments must be `Send + 'static`
- Wraps return types: `T` → `Result<T, JoinError>`, `Result<T, E>` → `Result<T, AsyncWrapError<E>>`

### `block_in_place`

```rust
#[blocking_impl(AsyncClient, strategy = "block_in_place")]
impl BlockingClient { /* ... */ }

pub struct AsyncClient {
    inner: BlockingClient,  // No Arc needed!
}
```

- Runs blocking code on the current thread (tells tokio to move other tasks)
- **No `'static` requirement** — you can borrow data
- **No `Arc` needed** — simpler struct definition
- **Return types preserved exactly** — no wrapping
- Requires tokio multi-threaded runtime

### When to use which?

| Use case | Strategy |
|----------|----------|
| Long-running blocking I/O | `spawn_blocking` |
| Quick blocking calls | `block_in_place` |
| Need to borrow data | `block_in_place` |
| Single-threaded runtime | `spawn_blocking` |

## Return Types

With `spawn_blocking` (default), return types are wrapped:

| Blocking Return Type | Async Return Type |
|---------------------|-------------------|
| `Result<T, E>` | `Result<T, AsyncWrapError<E>>` |
| `T` (non-Result) | `Result<T, JoinError>` |
| `()` | `Result<(), JoinError>` |

With `block_in_place`, return types are **preserved exactly**.

## Configuration

### Custom field name

By default, the macro expects a field named `inner`. Use the `field` parameter to customize:

```rust
#[blocking_impl(AsyncClient, field = "client")]
impl BlockingClient { /* ... */ }

pub struct AsyncClient {
    client: Arc<BlockingClient>,  // Custom field name
}
```

You can combine with strategy:

```rust
#[blocking_impl(AsyncClient, strategy = "block_in_place", field = "client")]
```

## Requirements

- Methods must take `&self` (not `&mut self` or `self`)
- For `spawn_blocking`: arguments must be `Send + 'static`, struct needs `inner: Arc<BlockingType>` (or custom field)
- For `block_in_place`: struct needs `inner: BlockingType` (or custom field), requires multi-threaded runtime

### Non-Send types with `spawn_blocking`

With the default `spawn_blocking` strategy, arguments are moved to a separate thread. Types like `Rc<T>`, `&T`, or anything not `Send + 'static` will fail to compile:

```rust
// This won't compile - Rc is not Send
#[async_wrap]
pub fn with_rc(&self, rc: Rc<i32>) -> i32 { *rc }
```

If you need to pass non-Send types, use `strategy = "block_in_place"` instead.

## Generics

Generic types are supported:

```rust
#[blocking_impl(AsyncService<T>)]
impl<T: Clone + Send + Sync + 'static> BlockingService<T> {
    #[async_wrap]
    pub fn get_data(&self) -> T {
        self.data.clone()
    }
}

pub struct AsyncService<T> {
    inner: Arc<BlockingService<T>>,
}
```

## License

MIT OR Apache-2.0

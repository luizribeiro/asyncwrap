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
3. The macro generates an `impl AsyncType` with async versions that use `spawn_blocking`

## License

MIT OR Apache-2.0

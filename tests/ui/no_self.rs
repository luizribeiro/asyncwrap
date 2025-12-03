use asyncwrap::blocking_impl;

struct BlockingClient;

#[blocking_impl(AsyncClient)]
impl BlockingClient {
    #[async_wrap]
    pub fn bad_method() -> i32 {
        42
    }
}

pub struct AsyncClient {
    inner: std::sync::Arc<BlockingClient>,
}

fn main() {}

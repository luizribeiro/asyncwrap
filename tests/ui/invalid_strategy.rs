use asyncwrap::blocking_impl;

struct BlockingClient;

#[blocking_impl(AsyncClient, strategy = "invalid")]
impl BlockingClient {
    #[async_wrap]
    pub fn method(&self) -> i32 {
        42
    }
}

pub struct AsyncClient {
    inner: std::sync::Arc<BlockingClient>,
}

fn main() {}

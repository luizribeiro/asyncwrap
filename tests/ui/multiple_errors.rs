use asyncwrap::blocking_impl;

struct BlockingClient;

#[blocking_impl(AsyncClient)]
impl BlockingClient {
    #[async_wrap]
    pub fn mut_method(&mut self) -> i32 {
        42
    }

    #[async_wrap]
    pub async fn async_method(&self) -> i32 {
        42
    }
}

pub struct AsyncClient {
    inner: std::sync::Arc<BlockingClient>,
}

fn main() {}

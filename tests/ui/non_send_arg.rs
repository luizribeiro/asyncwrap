use asyncwrap::blocking_impl;
use std::rc::Rc;

struct BlockingClient;

#[blocking_impl(AsyncClient)]
impl BlockingClient {
    #[async_wrap]
    pub fn with_rc(&self, rc: Rc<i32>) -> i32 {
        *rc
    }
}

pub struct AsyncClient {
    inner: std::sync::Arc<BlockingClient>,
}

fn main() {
    // This should fail because Rc is not Send
    let _ = AsyncClient {
        inner: std::sync::Arc::new(BlockingClient),
    };
}

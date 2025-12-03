//! Integration tests for asyncwrap

#![allow(dead_code, clippy::wildcard_imports, clippy::unused_self)]

use asyncwrap::blocking_impl;
use std::sync::Arc;

mod block_in_place_strategy {
    use asyncwrap::blocking_impl;
    use thiserror::Error;

    struct BlockingService {
        value: i32,
    }

    #[blocking_impl(AsyncService, strategy = "block_in_place")]
    impl BlockingService {
        #[async_wrap]
        pub fn get_value(&self) -> i32 {
            self.value
        }

        #[async_wrap]
        pub fn add(&self, n: i32) -> i32 {
            self.value + n
        }
    }

    pub struct AsyncService {
        inner: BlockingService,
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_basic_block_in_place() {
        let async_svc = AsyncService {
            inner: BlockingService { value: 42 },
        };

        assert_eq!(async_svc.get_value().await, 42);
        assert_eq!(async_svc.add(8).await, 50);
    }

    #[derive(Error, Debug, PartialEq)]
    enum MyError {
        #[error("something failed")]
        Failed,
    }

    struct BlockingClient;

    #[blocking_impl(AsyncClient, strategy = "block_in_place")]
    impl BlockingClient {
        #[async_wrap]
        pub fn might_fail(&self, succeed: bool) -> Result<String, MyError> {
            if succeed {
                Ok("success".to_string())
            } else {
                Err(MyError::Failed)
            }
        }
    }

    pub struct AsyncClient {
        inner: BlockingClient,
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_result_types_preserved() {
        let client = AsyncClient {
            inner: BlockingClient,
        };

        let ok_result: Result<String, MyError> = client.might_fail(true).await;
        assert_eq!(ok_result.unwrap(), "success");

        let err_result: Result<String, MyError> = client.might_fail(false).await;
        assert_eq!(err_result.unwrap_err(), MyError::Failed);
    }

    struct UnitService;

    #[blocking_impl(AsyncUnitService, strategy = "block_in_place")]
    impl UnitService {
        #[async_wrap]
        pub fn do_nothing(&self) {}
    }

    pub struct AsyncUnitService {
        inner: UnitService,
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_unit_return() {
        let svc = AsyncUnitService {
            inner: UnitService,
        };
        svc.do_nothing().await;
    }
}

mod basic_wrapping {
    use super::*;

    struct BlockingService {
        value: i32,
    }

    #[blocking_impl(AsyncService)]
    impl BlockingService {
        #[async_wrap]
        pub fn get_value(&self) -> i32 {
            self.value
        }

        #[async_wrap]
        pub fn add(&self, n: i32) -> i32 {
            self.value + n
        }
    }

    pub struct AsyncService {
        inner: Arc<BlockingService>,
    }

    #[tokio::test]
    async fn test_basic_wrapping() {
        let blocking = BlockingService { value: 42 };
        let async_svc = AsyncService {
            inner: Arc::new(blocking),
        };

        assert_eq!(async_svc.get_value().await.unwrap(), 42);
        assert_eq!(async_svc.add(8).await.unwrap(), 50);
    }
}

mod result_types {
    use super::*;
    use thiserror::Error;

    #[derive(Error, Debug)]
    enum MyError {
        #[error("something failed")]
        Failed,
    }

    struct BlockingClient;

    #[blocking_impl(AsyncClient)]
    impl BlockingClient {
        #[async_wrap]
        pub fn might_fail(&self, succeed: bool) -> Result<String, MyError> {
            if succeed {
                Ok("success".to_string())
            } else {
                Err(MyError::Failed)
            }
        }
    }

    pub struct AsyncClient {
        inner: Arc<BlockingClient>,
    }

    #[tokio::test]
    async fn test_result_types() {
        let client = AsyncClient {
            inner: Arc::new(BlockingClient),
        };

        assert!(client.might_fail(true).await.is_ok());
        assert!(client.might_fail(false).await.is_err());
    }
}

mod multiple_args {
    use super::*;

    struct Calculator;

    #[blocking_impl(AsyncCalculator)]
    impl Calculator {
        #[async_wrap]
        pub fn compute(&self, a: i32, b: i32, op: char) -> i32 {
            match op {
                '+' => a + b,
                '-' => a - b,
                '*' => a * b,
                '/' => a / b,
                _ => 0,
            }
        }
    }

    pub struct AsyncCalculator {
        inner: Arc<Calculator>,
    }

    #[tokio::test]
    async fn test_multiple_args() {
        let calc = AsyncCalculator {
            inner: Arc::new(Calculator),
        };
        assert_eq!(calc.compute(10, 5, '+').await.unwrap(), 15);
        assert_eq!(calc.compute(10, 5, '-').await.unwrap(), 5);
        assert_eq!(calc.compute(10, 5, '*').await.unwrap(), 50);
        assert_eq!(calc.compute(10, 5, '/').await.unwrap(), 2);
    }
}

mod only_marked_methods {
    use super::*;

    struct Service;

    #[blocking_impl(AsyncService)]
    impl Service {
        #[async_wrap]
        pub fn public_method(&self) -> i32 {
            42
        }

        fn private_helper(&self) -> i32 {
            0
        }
    }

    pub struct AsyncService {
        inner: Arc<Service>,
    }

    #[tokio::test]
    async fn test_only_marked_methods() {
        let svc = AsyncService {
            inner: Arc::new(Service),
        };
        assert_eq!(svc.public_method().await.unwrap(), 42);
    }
}

mod generics {
    use super::*;

    struct GenericService<T> {
        data: T,
    }

    #[blocking_impl(AsyncGenericService<T>)]
    impl<T: Clone + Send + Sync + 'static> GenericService<T> {
        #[async_wrap]
        pub fn get_data(&self) -> T {
            self.data.clone()
        }
    }

    pub struct AsyncGenericService<T> {
        inner: Arc<GenericService<T>>,
    }

    #[tokio::test]
    async fn test_generics() {
        let svc = AsyncGenericService {
            inner: Arc::new(GenericService {
                data: "hello".to_string(),
            }),
        };
        assert_eq!(svc.get_data().await.unwrap(), "hello");
    }
}

mod unit_return {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};

    struct SideEffectService {
        called: AtomicBool,
    }

    #[blocking_impl(AsyncSideEffectService)]
    impl SideEffectService {
        #[async_wrap]
        pub fn do_work(&self) {
            self.called.store(true, Ordering::SeqCst);
        }
    }

    pub struct AsyncSideEffectService {
        inner: Arc<SideEffectService>,
    }

    #[tokio::test]
    async fn test_unit_return() {
        let svc = AsyncSideEffectService {
            inner: Arc::new(SideEffectService {
                called: AtomicBool::new(false),
            }),
        };
        svc.do_work().await.unwrap();
        assert!(svc.inner.called.load(Ordering::SeqCst));
    }
}

mod async_wrap_error {
    use thiserror::Error;

    #[derive(Error, Debug)]
    #[error("test error")]
    struct TestError;

    #[test]
    fn test_error_display() {
        let err: asyncwrap::AsyncWrapError<TestError> = asyncwrap::AsyncWrapError::Inner(TestError);
        assert_eq!(err.to_string(), "test error");
    }

    #[test]
    fn test_error_source() {
        use std::error::Error;

        let err: asyncwrap::AsyncWrapError<TestError> = asyncwrap::AsyncWrapError::Inner(TestError);
        assert!(err.source().is_some());
    }
}

mod visibility {
    use super::*;

    struct VisService;

    #[blocking_impl(AsyncVisService)]
    impl VisService {
        #[async_wrap]
        pub fn public_fn(&self) -> i32 {
            1
        }

        #[async_wrap]
        pub(crate) fn crate_fn(&self) -> i32 {
            2
        }
    }

    pub struct AsyncVisService {
        inner: Arc<VisService>,
    }

    #[tokio::test]
    async fn test_visibility_preserved() {
        let svc = AsyncVisService {
            inner: Arc::new(VisService),
        };
        assert_eq!(svc.public_fn().await.unwrap(), 1);
        assert_eq!(svc.crate_fn().await.unwrap(), 2);
    }
}

mod task_panic {
    use super::*;
    use thiserror::Error;

    #[derive(Error, Debug)]
    #[error("inner error")]
    struct InnerError;

    struct PanickingService;

    #[blocking_impl(AsyncPanickingService)]
    impl PanickingService {
        #[async_wrap]
        pub fn will_panic(&self) -> i32 {
            panic!("intentional panic for testing");
        }

        #[async_wrap]
        #[allow(clippy::unnecessary_wraps, clippy::manual_assert)]
        pub fn might_panic(&self, should_panic: bool) -> Result<i32, InnerError> {
            if should_panic {
                panic!("intentional panic for testing");
            }
            Ok(42)
        }
    }

    pub struct AsyncPanickingService {
        inner: Arc<PanickingService>,
    }

    #[tokio::test]
    async fn test_panic_returns_join_error() {
        let svc = AsyncPanickingService {
            inner: Arc::new(PanickingService),
        };

        let result = svc.will_panic().await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.is_panic());
    }

    #[tokio::test]
    async fn test_result_panic_returns_task_failed() {
        let svc = AsyncPanickingService {
            inner: Arc::new(PanickingService),
        };

        let result = svc.might_panic(true).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            asyncwrap::AsyncWrapError::TaskFailed(_)
        ));
    }

    #[tokio::test]
    async fn test_result_no_panic_returns_inner() {
        let svc = AsyncPanickingService {
            inner: Arc::new(PanickingService),
        };

        let result = svc.might_panic(false).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }
}

#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.compile_fail("ui/*.rs");
}

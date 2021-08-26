use crate::private::*;
use std::sync::Mutex;
use threadpool::ThreadPool;

struct MultiThreaded;
impl MultiThreaded {
    fn exec<A: Send + 'static, F: FnOnce(A) + Send + 'static>(f: F, args: A) {
        lazy_static! {
            static ref POOL: Mutex<ThreadPool> = Mutex::new(ThreadPool::new(8));
        }
        POOL.lock().unwrap().execute(|| f(args));
    }
}

impl ExecuteMethod for MultiThreaded {}

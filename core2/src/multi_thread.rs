use std::{
    sync::Mutex
}
use crate::*;
use threadpool;

pub struct MultiThread;
pub type MultiThreadCloneEvent<A> = ListenerManager<SharedListener<A>, Cloning, MultiThread>;
pub type MultiThreadMoveEvent<A> = ListenerManager<SharedListener<A>, Moving, MultiThread>;

lazy_static!{
    static ref POOL:Mutex<threadpool::ThreadPool>=
}

impl MultiThreadExecutor for MultiThread {
    #[inline]
    fn exec<A: Send + 'static>(f: SharedCallable<A>, args: A) {
        std::thread::spawn(|| f.call(args));
    }
}

#[test]
fn test2() {
    let mut a = SharedMoveEvent::<u32>::new();
    let mutex = std::sync::Arc::new(std::sync::Mutex::new(0));

    let m = mutex.clone();
    a.listen_once(move |num| {
        *m.lock().unwrap() += 1;
        assert_eq!(num, 1);
    });

    let m = mutex.clone();
    let ch = a.listen_once(move |num| {
        *m.lock().unwrap() += 2;
        assert_eq!(num, 2);
    });

    let m = mutex.clone();
    a.listen(move |num| {
        *m.lock().unwrap() += num;
        assert!(num == 2 || num == 3);
    });

    a.unlisten(ch);
    a.emit(1);
    a.emit(2);
    a.emit(3);
    std::thread::sleep(std::time::Duration::from_millis(100));
    assert_eq!(*mutex.lock().unwrap(), 6);
}

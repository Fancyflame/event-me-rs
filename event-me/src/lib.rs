use std::{
    future::Future,
    ops::Deref,
    pin::Pin,
    sync::{atomic::*, Arc},
    task::{Context, Poll, Waker},
    time::Duration,
};

use tokio::{sync::broadcast, task::JoinHandle};

pub struct EventTarget<A> {
    sender: broadcast::Sender<A>,
}

pub struct UnlistenHandle(JoinHandle<()>);

impl<A> EventTarget<A>
where
    A: Clone + Send + 'static,
{
    pub fn new(cap: usize) -> Self {
        let (tx, _) = broadcast::channel(cap);
        EventTarget { sender: tx }
    }

    pub fn emit(&self, args: A) {
        //忽略错误
        drop(self.sender.send(args))
    }

    pub fn listen<F>(&self, mut func: F)
    where
        F: FnMut(A) + Send + 'static,
    {
        let mut rx = self.sender.subscribe();
        tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(args) => {
                        func(args);
                    }
                    Err(_) => break,
                }
            }
        });
    }

    pub async fn wait(&self) -> Option<A> {
        let mut rx = self.sender.subscribe();
        rx.recv().await.ok()
    }

    pub fn unlisten(&self, h: UnlistenHandle) {
        h.0.abort();
    }
}

pub trait EventWithArgs<K, A>
where
    K: PartialEq + Eq,
    A: Clone + Send + 'static,
{
    fn on<F>(&self, ev_name: K, func: F)
    where
        F: FnMut(A) + Send + 'static;

    fn off(&self, ev_name: K, h: UnlistenHandle);

    fn once<F>(&self, ev_name: K, func: F)
    where
        K: Clone,
        F: FnMut(A) + Send + 'static;
}

#[tokio::main]
#[test]
async fn it_works() {
    let et = EventTarget::<u32>::new(5);
    let mut a = 20;
    et.listen(move |arg| {
        a += arg;
        println!("{}", a);
    });

    for _ in 0..5 {
        println!("fool");
        et.emit(5);
        tokio::time::sleep(Duration::from_millis(800)).await;
    }
}

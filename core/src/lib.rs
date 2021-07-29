use std::{
    future::Future,
    ops::Deref,
    pin::Pin,
    sync::{atomic::*, Arc},
    task::{Context, Poll, Waker},
    time::Duration,
};

use tokio::{sync::broadcast, task::JoinHandle};

mod runtime;

pub struct EventTarget<A> {
    sender: broadcast::Sender<A>,
}

pub struct UnlistenHandle(JoinHandle<()>);

pub struct EventName<const N: u64>;

impl UnlistenHandle {
    #[inline]
    pub fn cancel(&self) {
        self.0.abort();
    }
}

impl<const N: u64> EventName<N> {
    #[inline]
    pub fn new<const M: u64>() -> EventName<M> {
        EventName::<M>
    }
}

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

    pub fn listen_once<F>(&self, mut func: F) -> UnlistenHandle
    where
        F: FnMut(A) + Send + 'static,
    {
        let mut rx = self.sender.subscribe();
        let jh = tokio::spawn(async move {
            if let Ok(args) = rx.recv().await {
                func(args);
            }
        });
        UnlistenHandle(jh)
    }

    pub fn unlisten(&self, h: UnlistenHandle) {
        h.0.abort();
    }
}

pub trait EventTargetMarker {}

pub trait EventTargetAttachment {
    type Output: EventTargetMarker;
    fn event_target(&self) -> &Self::Output;
}

pub trait EventEmitter<A, const ID: u64>
where
    A: Clone + Send + 'static,
{
    fn on<F: FnMut(A)>(&self, name: EventName<ID>) -> UnlistenHandle;
    fn off(&self, handle: UnlistenHandle) -> bool;
}

pub trait OnceEventEmitter<A, const ID: u64>
where
    A: Clone + Send + 'static,
{
    fn once<F: FnMut(A)>(&self, name: EventName<ID>) -> UnlistenHandle;
    fn off(&self, handle: UnlistenHandle) -> bool;
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

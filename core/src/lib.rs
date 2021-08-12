use std::{
    future::Future,
    ops::Deref,
    pin::Pin,
    sync::{atomic::*, Arc, Mutex},
    task::{Context, Poll, Waker},
    time::Duration,
};

use tokio::{sync::broadcast, task::JoinHandle};

//mod fastlink;
mod runtime;

pub struct ListenerList<A>(Vec<Box<FnMut(A)->>>);

pub struct UnlistenHandle(u64);

pub struct EventName<const N: u64>;

impl UnlistenHandle {
    fn new()->Self{
        static COUNTER:AtomicU64 = AtomicU64::new(0);
        let mut g=0;
        COUNTER.fetch_update(|x|{
            g=x.add_checked(1).expect("No more unique handle is available");
            Some(g);
        }).unwrap();
        let uh=UnlistenHandle(*g);
    }
}

impl<const N: u64> EventName<N> {
    #[inline]
    pub fn new<const M: u64>() -> EventName<M> {
        EventName::<M>
    }
}

impl<A> ListenerList<A>
where
    A: Copy + 'static,
{
    pub fn new(cap: usize) -> Self {
        ListenerList(Vec::new())
    }

    pub fn emit(&mut self, args: A) {
        for func in self.0.iter_mut(){
            func(args);
        }
    }

    pub fn listen<F>(&self, mut func: F)->UnlistenHandle
    where
        F: FnMut(A) + Send + 'static,
    {
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
    A: Copy + 'static,
{
    fn on<F: FnMut(A)+Send>(&self, name: EventName<ID>, func:F) -> UnlistenHandle;
    fn off(&self, handle: UnlistenHandle) -> bool;
}

pub trait OnceEventEmitter<A, const ID: u64>
where
    A: Copy + 'static,
{
    fn once<F: FnMut(A)+Send>(&self, name: EventName<ID>, func:F) -> UnlistenHandle;
    fn off(&self, handle: UnlistenHandle) -> bool;
}

#[test]
fn example(){
    struct MyStruct{
        event1:ListenerList<(arg1,arg2)>,
        event2:ListenerList<(arg3,)>
    }
}

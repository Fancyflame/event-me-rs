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


    pub fn listen_once<F>(&self,mut func:F)->UnlistenHandle
    where
        F: FnMut(A) + Send + 'static
    {
        let mut rx = self.sender.subscribe();
        let jh=tokio::spawn(async move {
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


pub trait EventTargetAttachment{
    type Output;
    fn event_target(&self)->&EventTarget<Self::Arg>;
}


pub trait EventWithArgs<const N:u32, A>
where
    A: Clone + Send + 'static,
{

    fn event_target(&self)->&EventTarget<A>;


    #[inline]
    fn on<F>(&self, func: F)
    where
        F: FnMut(A) + Send + 'static
    {
        self.event_target().listen(func)
    }


    #[inline]
    fn off(&self, h: UnlistenHandle){
        self.event_target().unlisten(h)
    }


    #[inline]
    fn once<F>(&self, func: F)
    where
        F: FnMut(A) + Send + 'static
    {
        self.event_target().listen_once(func)
    }
}


#[macro_export]
macro_rules! event_target{
    {
        struct $struct:ident;
        enum $enum:ident;
        $($name:ident=>$ty:ty),*
    }=>{
        enum $enum{
            __ZeroStartHeadDoNotUseThis__=0u32,
            $($name),*
        }

        struct $struct{
            $( $name: $crate::EventTarget<$ty> ),*
        }

        impl $struct{
            fn new()->Self{
                $struct {
                    $($name:$crate::EventTarget<$ty>::new()),*
                }
            }
        }

        $(
            impl EventWithArgs<$enum::$name as u32,$ty> for $struct{
                fn event_target(&self)->&$crate::EventTarget{
                    &self.$name
                }
            }
        )*
    }
}


struct M<const N:u64>;



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

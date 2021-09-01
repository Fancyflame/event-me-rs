use std::{
    cmp::{Eq, PartialEq},
    collections::VecDeque,
    marker::PhantomData,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex
    },
    future::Future,
    pin::Pin,
    ops::Deref,
    rc::Rc,
    cell::RefCell,
    task::{Waker,Context,Poll}
};

#[cfg(feature = "thread-pool")]
#[macro_use]
extern crate lazy_static;

#[cfg(feature = "thread-pool")]
pub mod thread_pool;

#[cfg(feature = "thread-pool")]
pub use thread_pool::*;

/*#[cfg(features = "tokio-rt")]
pub mod tokio_rt;

#[cfg(features = "tokio-rt")]
pub use tokio_rt::*;*/

trait ExecArgsProcess {}
pub trait MultiThreadExecutor {
    fn exec<A: Send + 'static>(f: SharedCallable<A>, args: A);
}
pub trait LocalThreadExecutor {
    fn exec<A>(f: Callable<'_, A>, args: A);
}
trait RefCount<T> {
    unsafe fn get_mut_unchecked<'a>(&'a self)->&'a mut T;
}

pub struct Moving;
pub struct Cloning;
pub struct LocalThread;


pub struct AsyncListener<A>{
    inner:Rc<(RefCell<Option<A>>,RefCell<Option<Waker>>)>
}

pub struct SharedAsyncListener<A>{
    inner:Arc<(Mutex<Option<A>>,Mutex<Option<Waker>>)>
}


pub enum Listener<'a, A> {
    Once(Box<dyn FnOnce(A) + 'a>),
    Multiple(Box<dyn FnMut(A) + 'a>),
    Called,
}

pub enum SharedListener<A: Send + 'static> {
    Once(Box<dyn FnOnce(A) + Send + 'static>),
    Multiple(Arc<dyn Fn(A) + Send + Sync + 'static>),
    Called,
}


pub enum Callable<'a, A> {
    BoxedFnOnce(Box<dyn FnOnce(A) + 'a>),
    RefFnMut(&'a mut (dyn FnMut(A) + 'a)),
}

pub enum SharedCallable<A: Send + 'static> {
    BoxedFnOnce(Box<dyn FnOnce(A) + Send + 'static>),
    ArcFn(Arc<dyn Fn(A) + Send + Sync + 'static>),
}


pub struct EventTarget<F, P, E> {
    listeners: VecDeque<(CancelHandle, F)>,
    _marker: (PhantomData<P>, PhantomData<E>),
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct CancelHandle(u64);

impl CancelHandle {
    fn new() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        CancelHandle(COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}

//监听者管理

impl<'a, A> Listener<'a, A> {
    fn from_fn_mut<F: FnMut(A) + 'a>(f: F) -> Self {
        Listener::Multiple(Box::new(f))
    }

    fn from_fn_once<F: FnOnce(A) + 'a>(f: F) -> Self {
        Listener::Once(Box::new(f))
    }

    fn needs_drop(&self) -> bool {
        match self {
            Listener::Called => true,
            _ => false,
        }
    }

    fn get(&mut self) -> (Callable<'_, A>, bool) {
        match self {
            Listener::Once(_) => {
                let f = std::mem::replace(self, Listener::Called);
                match f {
                    Listener::Once(func) => (Callable::BoxedFnOnce(func), true),
                    _ => unreachable!(),
                }
            }
            Listener::Called => panic!("This function impls FnOnce and has been called"),
            Listener::Multiple(ref mut func) => (Callable::RefFnMut(func), false),
        }
    }
}

impl<A: Send> SharedListener<A> {
    fn from_fn<F: Fn(A) + Send + Sync + 'static>(f: F) -> Self {
        SharedListener::Multiple(Arc::new(f))
    }

    fn from_fn_once<F: FnOnce(A) + Send + 'static>(f: F) -> Self {
        SharedListener::Once(Box::new(f))
    }

    fn needs_drop(&self) -> bool {
        match self {
            SharedListener::Called => true,
            _ => false,
        }
    }

    fn get(&mut self) -> (SharedCallable<A>, bool) {
        match self {
            SharedListener::Once(_) => {
                let f = std::mem::replace(self, SharedListener::Called);
                match f {
                    SharedListener::Once(func) => (SharedCallable::BoxedFnOnce(func), true),
                    _ => unreachable!(),
                }
            }
            SharedListener::Called => panic!("This function only impls FnOnce and has been called"),
            SharedListener::Multiple(func) => (SharedCallable::ArcFn(func.clone()), false),
        }
    }
}

impl<F, P, E> EventTarget<F, P, E> {
    #[inline]
    pub fn new() -> Self {
        EventTarget {
            listeners: VecDeque::new(),
            _marker: Default::default(),
        }
    }

    #[inline]
    pub fn with_capacity(n: usize) -> Self {
        EventTarget {
            listeners: VecDeque::with_capacity(n),
            _marker: Default::default(),
        }
    }

    /*#[inline]
    fn iter_mut<'a>(&'a mut self)->std::collections::vec_deque::IterMut<'a,F>{
        self.listeners.iter_mut()
    }*/

    fn _listen(&mut self, f: F) -> CancelHandle {
        let ch = CancelHandle::new();
        self.listeners.push_back((ch.clone(), f));
        ch
    }

    fn _unlisten(&mut self, ch: CancelHandle) -> Option<F> {
        for (index, (ch_cmp, _)) in self.listeners.iter().enumerate() {
            if *ch_cmp == ch {
                return self.listeners.remove(index).map(|x| x.1);
            }
        }
        None
    }
}

//异步执行器

impl<A> Future for AsyncListener<A>{
    type Output=A;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>)->Poll<A>{
        let mut waker=self.inner.1.borrow_mut();
        match self.inner.0.borrow_mut().take(){
            Some(n)=>Poll::Ready(n),
            None=>{
                if waker.is_none(){
                    *waker=Some(cx.waker().clone());
                }
                Poll::Pending
            }
        }
    }
}

impl<A> Future for SharedAsyncListener<A>{
    type Output=A;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>)->Poll<A>{
        let mut waker=self.inner.1.lock().unwrap();
        match self.inner.0.lock().unwrap().take(){
            Some(n)=>Poll::Ready(n),
            None=>{
                if waker.is_none(){
                    *waker=Some(cx.waker().clone());
                }
                Poll::Pending
            }
        }
    }
}

//执行器

impl<'a, A> Callable<'a, A> {
    #[inline]
    pub fn call(self, args: A) {
        match self {
            Callable::BoxedFnOnce(func) => func(args),
            Callable::RefFnMut(func) => func(args),
        }
    }
}

impl<A: Send> SharedCallable<A> {
    #[inline]
    pub fn call(self, args: A) {
        match self {
            SharedCallable::BoxedFnOnce(func) => func(args),
            SharedCallable::ArcFn(func) => func(args),
        }
    }
}

//注册方法

impl<'a, A:'a, P, E:LocalThreadExecutor> EventTarget<Listener<'a, A>, P, E> {
    #[inline]
    pub fn listen(&mut self, f: impl FnMut(A) + 'a) -> CancelHandle {
        self._listen(Listener::from_fn_mut(f))
    }

    #[inline]
    pub fn listen_once(&mut self, f: impl FnOnce(A) + 'a) -> CancelHandle {
        self._listen(Listener::from_fn_once(f))
    }

    #[inline]
    pub fn unlisten(&mut self, ch: CancelHandle) -> Option<Listener<'a, A>> {
        self._unlisten(ch)
    }

    pub fn wait_for(&mut self)->AsyncListener<A>{
        let inner=Rc::new((RefCell::new(None),RefCell::new(None)));
        let al=AsyncListener{
            inner:inner.clone()
        };
        self.listen_once(move|args|{
            *inner.0.borrow_mut()=Some(args);
            if let Some(waker)=inner.1.borrow_mut().take(){
                waker.wake();
            }
        });
        al
    }
}

impl<A: Send, P, E: MultiThreadExecutor> EventTarget<SharedListener<A>, P, E> {
    #[inline]
    pub fn listen(&mut self, f: impl Fn(A) + Send + Sync + 'static) -> CancelHandle {
        self._listen(SharedListener::from_fn(f))
    }

    #[inline]
    pub fn listen_once(&mut self, f: impl FnOnce(A) + Send + 'static) -> CancelHandle {
        self._listen(SharedListener::from_fn_once(f))
    }

    #[inline]
    pub fn unlisten(&mut self, ch: CancelHandle) -> Option<SharedListener<A>> {
        self._unlisten(ch)
    }

    pub fn wait_for(&mut self)->SharedAsyncListener<A>{
        let inner=Arc::new((Mutex::new(None),Mutex::new(None)));
        let al=SharedAsyncListener{
            inner:inner.clone()
        };
        self.listen_once(move|args|{
            *inner.0.lock().unwrap()=Some(args);
            if let Ok(mut g)=inner.1.lock(){
                if let Some(waker)=g.take(){
                    waker.wake();
                }
            }
        });
        al
    }
}

//实现

macro_rules! _impl {
    (cloning) => {
        pub fn emit(&mut self, args: A) {
            for (_, x) in self.listeners.iter_mut() {
                E::exec(x.get().0, args.clone());
            }
            self.listeners.retain(|(_, x)| !x.needs_drop());
        }
    };
    (moving) => {
        pub fn emit(&mut self, args: A) {
            let (c, needs_drop) = self.listeners.front_mut().unwrap().1.get();
            E::exec(c, args);
            if needs_drop {
                self.listeners.pop_front();
            }
        }
    };
}

impl<'a, A: Clone, E: LocalThreadExecutor> EventTarget<Listener<'a, A>, Cloning, E> {
    _impl!(cloning);
}

impl<'a, A, E: LocalThreadExecutor> EventTarget<Listener<'a, A>, Moving, E> {
    _impl!(moving);
}

impl<'a, A: Clone + Send + 'static, E: MultiThreadExecutor>
    EventTarget<SharedListener<A>, Cloning, E>
{
    _impl!(cloning);
}

impl<'a, A: Send + 'static, E: MultiThreadExecutor> EventTarget<SharedListener<A>, Moving, E> {
    _impl!(moving);
}

//提供两个默认模板

impl LocalThreadExecutor for LocalThread {
    #[inline]
    fn exec<A>(f: Callable<'_, A>, args: A) {
        f.call(args);
    }
}

impl MultiThreadExecutor for LocalThread {
    #[inline]
    fn exec<A:Send+'static>(f: SharedCallable<A>, args: A) {
        f.call(args);
    }
}

pub type LocalEvent<'a, A, P, E> = EventTarget<Listener<'a, A>, P, E>;
pub type SharedEvent<A, P, E> = EventTarget<SharedListener<A>, P, E>;

#[test]
fn test1() {
    let k = std::cell::Cell::new(0);
    let mut a = LocalEvent::<u32,Cloning,LocalThread>::new();

    a.listen(|num| {
        k.set(num);
    });

    a.listen_once(|num| {
        k.set(num + 100);
    });

    a.emit(100);
    assert_eq!(k.get(), 200);

    a.emit(100);
    assert_eq!(k.get(), 100);
}

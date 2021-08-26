//#![allow(dead_code)]

use std::{
    cell::{RefCell, RefMut},
    cmp::{Eq, PartialEq},
    collections::{HashMap, VecDeque},
    error::Error,
    fmt,
    marker::PhantomData,
    //ops::{Deref, DerefMut},
    sync::{
        atomic::{AtomicU64, Ordering},
        Mutex, MutexGuard,
    },
};

use private::*;

#[cfg(feature = "multithread")]
#[macro_use]
extern crate lazy_static;

#[cfg(feature = "multithread")]
pub mod multithread;

#[cfg(feature = "multithread")]
pub use multithread::*;

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct CancelHandle(u64);

impl CancelHandle {
    fn new() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        CancelHandle(COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}

mod private {
    pub trait EmitCountLimitation {}
    pub trait ArgsOperation {}
    pub trait ExecuteMethod {}
    pub trait Into2<T> {
        fn into2(self) -> T;
    }
    pub enum Listener<'a, A> {
        Once(Box<dyn FnOnce(A) + 'a>),
        Reusable(Box<dyn FnMut(A) + 'a>),
        Called,
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CanOnlyCallOnceError;

pub struct SingleThreaded;

pub type OnceInner<'a, A, O, M> = ListenerManager<Box<dyn FnOnce(A) + 'a>, Once, O, M>;

pub struct Once;

pub type ReusableInner<'a, A, O, M> = ListenerManager<Listener<'a, A>, Reusable, O, M>;

pub struct Reusable;

pub struct CloneArgs;

pub struct MoveArgs;

pub struct ListenerManager<F, C, O, M> {
    listeners: VecDeque<(CancelHandle, F)>,
    _marker: (PhantomData<C>, PhantomData<O>, PhantomData<M>),
}

//实现多次触发单次事件的错误

impl fmt::Display for CanOnlyCallOnceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "This event can only be emitted once")
    }
}

impl Error for CanOnlyCallOnceError {}

//配置

impl EmitCountLimitation for Reusable {}
impl EmitCountLimitation for Once {}
impl ArgsOperation for CloneArgs {}
impl ArgsOperation for MoveArgs {}
impl ExecuteMethod for SingleThreaded {}

impl<'a, A, F: FnMut(A) + 'a> Into2<Box<dyn FnMut(A) + 'a>> for F {
    fn into2(self) -> Box<dyn FnMut(A) + 'a> {
        Box::new(self)
    }
}

impl<'a, A> Into2<Listener<'a, A>> for Listener<'a, A> {
    fn into2(self) -> Listener<'a, A> {
        self
    }
}

//监听者管理

impl<'a, A> Listener<'a, A> {
    fn from_mut<F: FnMut(A) + 'a>(f: F) -> Self {
        Listener::Reusable(Box::new(f) as Box<dyn FnMut(A)>)
    }

    fn from_once<F: FnOnce(A) + 'a>(f: F) -> Self {
        Listener::Once(Box::new(f) as Box<dyn FnOnce(A)>)
    }

    fn needs_drop(&self) -> bool {
        match self {
            Listener::Called => true,
            _ => false,
        }
    }

    fn call<M: ExecuteMethod>(&mut self, args: A) -> bool {
        match self {
            Listener::Once(_) => {
                let f = std::mem::replace(self, Listener::Called);
                match f {
                    Listener::Once(func) => {
                        M::exec(func, args);
                        true
                    }
                    _ => unreachable!(),
                }
            }
            Listener::Called => true,
            Listener::Reusable(func) => {
                M::exec(func, args);
                false
            }
        }
    }
}

impl<F, C: EmitCountLimitation, O: ArgsOperation, M: ExecuteMethod> ListenerManager<F, C, O, M> {
    #[inline]
    pub fn new() -> Self {
        ListenerManager {
            listeners: VecDeque::new(),
            _marker: Default::default(),
        }
    }

    #[inline]
    pub fn with_capacity(n: usize) -> Self {
        ListenerManager {
            listeners: VecDeque::with_capacity(n),
            _marker: Default::default(),
        }
    }

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

//可复用事件

impl<'a, A, O: ArgsOperation, M: ExecuteMethod> ReusableInner<'a, A, O, M> {
    pub fn listen<F: FnMut(A) + 'a>(&mut self, f: F) -> CancelHandle {
        self._listen(Listener::from_mut(f))
    }

    pub fn listen_once<F: FnOnce(A) + 'a>(&mut self, f: F) -> CancelHandle {
        self._listen(Listener::from_once(f))
    }

    pub fn unlisten(&mut self, ch: CancelHandle) -> Option<Box<dyn FnMut(A) + 'a>> {
        match self._unlisten(ch) {
            Some(Listener::Reusable(func)) => Some(func),
            _ => None,
        }
    }

    pub fn unlisten_once(&mut self, ch: CancelHandle) -> Option<Box<dyn FnOnce(A) + 'a>> {
        match self._unlisten(ch) {
            Some(Listener::Reusable(_func)) => {
                None //TODO
            }
            Some(Listener::Once(func)) => Some(func),
            _ => None,
        }
    }
}

impl<A: Clone, M: ExecuteMethod> ReusableInner<'_, A, CloneArgs, M> {
    pub fn emit(&mut self, args: A) {
        for (_, func) in self.listeners.iter_mut() {
            func.call::<M>(args.clone());
        }
        self.listeners.retain(|x| x.1.needs_drop());
    }
}

impl<A, M: ExecuteMethod> ReusableInner<'_, A, MoveArgs, M> {
    pub fn emit(&mut self, args: A) {
        if let Some(func) = self.listeners.get_mut(0) {
            func.1.call::<M>(args);
            if func.1.needs_drop() {
                self.listeners.pop_front();
            }
        }
    }
}

//单次事件

impl<'a, A, O: ArgsOperation, M: ExecuteMethod> OnceInner<'a, A, O, M> {
    pub fn listen_once<F: FnOnce(A) + 'a>(&mut self, f: F) -> CancelHandle {
        self._listen(Box::new(f))
    }

    pub fn unlisten(&mut self, ch: CancelHandle) -> Option<Box<dyn FnOnce(A) + 'a>> {
        self._unlisten(ch)
    }
}

impl<A: Clone, M: ExecuteMethod> OnceInner<'_, A, CloneArgs, M> {
    pub fn emit(self, args: A) {
        for (_, func) in self.listeners.into_iter() {
            M::exec(func, args.clone())
        }
    }
}

impl<A, M: ExecuteMethod> OnceInner<'_, A, MoveArgs, M> {
    pub fn emit(mut self, args: A) {
        if let Some((_, func)) = self.listeners.pop_front() {
            M::exec(func, args)
        }
    }
}

//执行者

impl SingleThreaded {
    #[inline]
    fn exec<A, F: FnOnce(A)>(f: F, args: A) {
        f(args);
    }
}

//输出

pub type EventReg<'a, A> = ReusableInner<'a, A, CloneArgs, SingleThreaded>;
pub type MoveEventReg<'a, A> = ReusableInner<'a, A, MoveArgs, SingleThreaded>;
pub type OnceEventReg<'a, A> = OnceInner<'a, A, CloneArgs, SingleThreaded>;
pub type OnceMoveEventReg<'a, A> = OnceInner<'a, A, MoveArgs, SingleThreaded>;

#[test]
fn test() {
    let out = std::cell::Cell::new(0);
    let mut a = EventReg::<u32>::new();
    a.listen(|num| out.set(num));
    a.listen(|num| out.set(num + 200));
    a.emit(100);
    assert_eq!(out.get(), 300);
}

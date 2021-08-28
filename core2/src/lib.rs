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

trait ExecCounts{}
trait ExecArgsProcess{}
trait Executor{}

struct Once;
struct Multiple;
impl ExecCounts for Once{}
impl ExecCounts for Multiple{}

struct Move;
struct ToRef;
struct Cloning;
impl ExecArgsProcess for Move{}
impl ExecArgsProcess for ToRef{}
impl ExecArgsProcess for Cloning{}

struct MultiThread;
struct LocalThread;
impl Executor for MultiThreaded{}
impl Executor for SingleThreaded{}

pub struct ListenerManager<F, C, P, E> {
    listeners: VecDeque<(CancelHandle, F)>,
    _marker: (PhantomData<C>, PhantomData<P>, PhantomData<E>),
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct CancelHandle(u64);

trait Into2<T>{
    fn into2(self)->T;
}

impl CancelHandle {
    fn new() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        CancelHandle(COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}

impl<F,C:ExecCounts,P:ExecArgsProcess,E:Executor> ListenerManager<F, C, P, E> {
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

impl<F,P,E> ListenerManager<F,Once,P,E>{
    fn 
}

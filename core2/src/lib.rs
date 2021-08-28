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
        Arc,
    },
};

trait ExecCounts {}
trait ExecArgsProcess<'a, A: 'a> {
    fn exec_list<I, E>(iter: I, args: A)
    where
        I: Iterator<Item = Callable<'a, A>>,
        E: Executor<'a>;
}
trait Executor<'a> {
    fn exec<A: 'a>(f: Callable<'a, A>, args: A);
}
trait Into2<T> {
    fn into2(self) -> T;
}

struct Once;
struct Multiple;
impl ExecCounts for Once {}
impl ExecCounts for Multiple {}

struct Move;
struct Cloning;
//impl ExecArgsProcess for Move {}
//impl ExecArgsProcess for Cloning {}

struct MultiThread;
struct LocalThread;
//impl Executor<'static> for MultiThread{}
//impl<'a> Executor<'a> for LocalThread{}

enum Listener<'a, A> {
    Once(Box<dyn FnOnce(A) + 'a>),
    Multiple(Box<dyn FnMut(A) + 'a>),
    Called,
}

enum SharedListener<A> {
    Once(Box<dyn FnOnce(A) + 'static>),
    Multiple(Arc<dyn Fn(A) + 'static>),
    Called,
}

enum Callable<'a, A> {
    BoxedFnOnce(Box<dyn FnOnce(A) + 'a>),
    RefFnMut(&'a mut (dyn FnMut(A) + 'a)),
}

enum SharedCallable<A: Send> {
    BoxedFnOnce(Box<dyn FnOnce(A) + Send + 'static>),
    ArcFn(Arc<dyn Fn(A) + Send + 'static>),
}

pub struct ListenerManager<F, C, P, E> {
    listeners: VecDeque<(CancelHandle, F)>,
    _marker: (PhantomData<C>, PhantomData<P>, PhantomData<E>),
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
    fn from_fn_mut<F: Fn(A) + 'a>(f: F) -> Self {
        Listener::Multiple(Box::new(f) as Box<dyn FnMut(A)>)
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

    fn call(&'a mut self) -> (Callable<'a, A>, bool) {
        match self {
            Listener::Once(_) => {
                let f = std::mem::replace(self, Listener::Called);
                match f {
                    Listener::Once(func) => (Callable::BoxedFnOnce(func), true),
                    _ => unreachable!(),
                }
            }
            Listener::Called => panic!("This function impls FnOnce and has been called"),
            Listener::Multiple(func) => (Callable::RefFnMut(func), false),
        }
    }
}

impl<A> SharedListener<A> {
    fn from_fn<F: Fn(A) + 'static>(f: F) -> Self {
        SharedListener::Multiple(Arc::new(f))
    }

    fn from_once<F: FnOnce(A) + 'static>(f: F) -> Self {
        SharedListener::Once(Box::new(f))
    }

    fn needs_drop(&self) -> bool {
        match self {
            SharedListener::Called => true,
            _ => false,
        }
    }

    fn call(&mut self) -> (Callable<'static, A>, bool) {
        match self {
            SharedListener::Once(_) => {
                let f = std::mem::replace(self, SharedListener::Called);
                match f {
                    SharedListener::Once(func) => (Callable::BoxedFnOnce(func), true),
                    _ => unreachable!(),
                }
            }
            SharedListener::Called => panic!("This function impls FnOnce and has been called"),
            SharedListener::Multiple(func) => (Callable::ArcFn(func.clone()), false),
        }
    }
}

impl<F, C, P, E> ListenerManager<F, C, P, E> {
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

//执行次数分配

impl<F, P, E> ListenerManager<F, Once, P, E> {}

//参数分配器

impl<'a, A: 'a> ExecArgsProcess<'a, A> for Move {
    fn exec_list<I, E>(mut iter: I, args: A)
    where
        I: Iterator<Item = Callable<'a, A>>,
        E: Executor<'a>,
    {
        if let Some(n) = iter.next() {
            E::exec(n, args);
        }
    }
}

impl<'a, A: Clone + 'a> ExecArgsProcess<'a, A> for Cloning {
    fn exec_list<I, E>(iter: I, args: A)
    where
        I: Iterator<Item = Callable<'a, A>>,
        E: Executor<'a>,
    {
        for n in iter {
            E::exec(n, args.clone());
        }
    }
}

//执行器

impl<'a, A> Callable<'a, A> {
    fn call(self, args: A) {
        match self {
            Callable::BoxedFnOnce(func) => func(args),
            Callable::RefFnMut(func) => func(args),
        }
    }
}

impl<A: Send> SharedCallable<A> {
    fn call(self, args: A) {
        match self {
            //TODO
            Callable::BoxedFnOnce(func) => func(args),
            Callable::RefFnMut(func) => func(args),
        }
    }
}

impl Executor<'static> for MultiThread {
    fn exec<A>(func: Callable<'static, A>, args: A) {
        std::thread::spawn(|| func.call(args));
    }
}

impl<'a> Executor<'a> for LocalThread {
    #[inline]
    fn exec<A>(func: Callable<'a, A>, args: A) {
        func.call(args);
    }
}

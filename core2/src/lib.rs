//#![allow(dead_code)]

use std::{
    cell::{RefCell, RefMut},
    cmp::{Eq, PartialEq},
    collections::{HashMap, VecDeque},
    error::Error,
    fmt,
    marker::PhantomData,
    ops::{Deref, DerefMut},
    sync::{
        atomic::{AtomicU64, Ordering},
        Mutex, MutexGuard,
    },
};

use private::*;

macro_rules! _event {
    ($name:ident,$shared:tt,$once:tt,$move:tt,$lock:ty,$($fnty:tt)*) => {
        pub struct $name<A>(
            _if!{
                if $shared{
                    Mutex<_if!{ if $once {Option<Listeners<dyn $($fnty)*>>} else {Listeners<dyn $($fnty)*>} }>
                }else{
                    RefCell<_if!{ if $once {Option<Listeners<dyn $($fnty)*>>} else {Listeners<dyn $($fnty)*>} }>
                }
            }
        );
        impl<A> $name<A>{
            pub fn new()->Self{
                $name(_if!{
                    if $shared{
                        Mutex::new(_if!{
                            if $once{
                                Some(HashMap::new())
                            }else{
                                HashMap::new()
                            }
                        })
                    }else{
                        RefCell::new(_if!{
                            if $once{
                                Some(HashMap::new())
                            }else{
                                HashMap::new()
                            }
                        })
                    }
                })
            }

            pub fn with_capacity(n:usize)->Self{
                $name(_if!{
                    if $shared{
                        Mutex::new(_if!{
                            if $once{
                                Some(HashMap::with_capacity(n))
                            }else{
                                HashMap::with_capacity(n)
                            }
                        })
                    }else{
                        RefCell::new(_if!{
                            if $once{
                                Some(HashMap::with_capacity(n))
                            }else{
                                HashMap::with_capacity(n)
                            }
                        })
                    }
                })
            }

            pub fn listen<F:$($fnty)*>(&self,func:F)->CancelHandle{
                let ch=CancelHandle::new();
                (_if! {
            if $shared{
                self.0.lock()
            }else{
                self.0.borrow_mut()
            }
        }).insert(ch.clone(),func);
                ch
            }

            pub fn unlisten(&self,ch:CancelHandle)->Option<Box<dyn $($fnty)*>>{
                _fetch_mut!($shared).remove(&ch)
            }
        }
    };
}

macro_rules! _if {
    {
        if 1 {
            $($true:tt)*
        }else{
            $($false:tt)*
        }
    } => {
        $($true)*
    };
    {
        if 0 {
            $($true:tt)*
        }else{
            $($false:tt)*
        }
    } => {
        $($false)*
    };
}

macro_rules! _fetch_mut {
    ($shared:tt) => {
        _if! {
            if $shared{
                self.0.lock()
            }else{
                self.0.borrow_mut()
            }
        }
    };
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct CancelHandle(u64);

impl CancelHandle {
    fn new() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        CancelHandle(COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}

/*_event!(Er, 0, 0, 0, RefCell, FnMut(A));
_event!(Ser, 1, 0, 0, Mutex, FnMut(A) + Send);
_event!(Oer, 0, 1, 0, RefCell, FnOnce(A));
_event!(Mer, 0, 0, 1, RefCell, FnMut(A));
_event!(Soer, 1, 1, 0, Mutex, FnOnce(A) + Send);
_event!(Omer, 0, 1, 1, RefCell, FnOnce(A));
_event!(Smer, 1, 0, 1, Mutex, FnMut(A) + Send);
_event!(Somer, 1, 1, 1, Mutex, FnOnce(A) + Send);*/

/*trait EmitCounts<A, E:ListenerManager<A>, C: ListenerContainer<A,E>> {
    fn _try_emit(&mut self) -> Option<&mut C>;
}

trait ListenerContainer<A,E:ListenerManager<A>> {
    type Target: DerefMut<Target = E>;
    fn _get_mut(self) -> Self::Target;
}

trait ListenerManager<A>{

}*/

mod private {
    pub trait EmitCountLimitation {}
    pub trait ArgsOperation {}
    pub trait From2<T> {
        fn from2(t: T) -> Self;
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CanOnlyCallOnceError;

pub struct Local<T>(RefCell<T>);

pub struct Shared<T>(Mutex<T>);

pub type OnceInner<A, O> = ListenerManager<Box<dyn FnOnce(A)>, Once, O>;

pub struct Once;

pub type ReusableInner<A, O> = ListenerManager<Listener<'static, A>, Reusable, O>;

pub struct Reusable;

pub struct CloneArgs;

pub struct MoveArgs;

pub struct ListenerManager<F, C, O> {
    listeners: VecDeque<(CancelHandle, F)>,
    _marker: (PhantomData<C>, PhantomData<O>),
}

pub enum Listener<'a, A> {
    Once(Box<dyn FnOnce(A) + 'a>),
    Reusable(Box<dyn FnMut(A) + 'a>),
    Called,
}

//实现多次触发单次事件的错误

impl fmt::Display for CanOnlyCallOnceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "This event can only be emitted once")
    }
}

impl Error for CanOnlyCallOnceError {}

//实现单线程/多线程/异步的容器

impl<'a, T> Local<T> {
    #[inline]
    fn new(v: T) -> Self {
        Local(RefCell::new(v))
    }

    #[inline]
    fn get_mut(&'a self) -> RefMut<'a, T> {
        self.0.borrow_mut()
    }
}

impl<'a, T> Shared<T> {
    #[inline]
    fn new(v: T) -> Self {
        Shared(Mutex::new(v))
    }

    #[inline]
    fn get_mut(&'a self) -> MutexGuard<'a, T> {
        self.0.lock().unwrap()
    }
}

//配置

impl EmitCountLimitation for Reusable {}
impl EmitCountLimitation for Once {}
impl ArgsOperation for CloneArgs {}
impl ArgsOperation for MoveArgs {}

impl<'a, A, F: FnMut(A) + 'a> From2<F> for Box<dyn FnMut(A) + 'a> {
    fn from2(f: F) -> Self {
        Box::new(f)
    }
}

impl<'a, A> From2<Listener<'a, A>> for Listener<'a, A> {
    fn from2(t: Listener<'a, A>) -> Self {
        t
    }
}

//监听者管理

impl<'a, A> Listener<'a, A> {
    pub fn from_mut<F: FnMut(A) + 'a>(f: F) -> Self {
        Listener::Reusable(Box::new(f) as Box<dyn FnMut(A)>)
    }

    pub fn from_once<F: FnOnce(A) + 'a>(f: F) -> Self {
        Listener::Once(Box::new(f) as Box<dyn FnOnce(A)>)
    }

    fn needs_drop(&self) -> bool {
        match self {
            Listener::Called => true,
            _ => false,
        }
    }

    fn call(&mut self, args: A) -> bool {
        match self {
            Listener::Once(_) => {
                let f = std::mem::replace(self, Listener::Called);
                match f {
                    Listener::Once(func) => {
                        func(args);
                        true
                    }
                    _ => unreachable!(),
                }
            }
            Listener::Called => true,
            Listener::Reusable(func) => {
                func(args);
                false
            }
        }
    }
}

impl<F, C: EmitCountLimitation, O: ArgsOperation> ListenerManager<F, C, O> {
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

    pub fn listen<B: Into<F>>(&mut self, f: B) -> CancelHandle {
        let ch = CancelHandle::new();
        self.listeners.push_back((ch.clone(), f.into()));
        ch
    }

    #[inline]
    pub fn unlisten(&mut self, ch: CancelHandle) -> Option<F> {
        for (index, (ch_cmp, _)) in self.listeners.iter().enumerate() {
            if *ch_cmp == ch {
                return self.listeners.remove(index).map(|x| x.1);
            }
        }
        None
    }
}

//可复用事件

impl<A: Clone> ReusableInner<A, CloneArgs> {
    pub fn emit(&mut self, args: A) {
        for (_, func) in self.listeners.iter_mut() {
            func.call(args.clone());
        }
        self.listeners.retain(|x| x.1.needs_drop());
    }
}

impl<A> ReusableInner<A, MoveArgs> {
    pub fn emit(&mut self, args: A) {
        if let Some(func) = self.listeners.get_mut(0) {
            func.1.call(args);
            if func.1.needs_drop() {
                self.listeners.pop_front();
            }
        }
    }
}

//单次事件

impl<A: Clone> OnceInner<A, CloneArgs> {
    pub fn emit(self, args: A) {
        for (_, func) in self.listeners.into_iter() {
            func(args.clone())
        }
    }
}

impl<A> OnceInner<A, MoveArgs> {
    pub fn emit(mut self, args: A) {
        if let Some((_, func)) = self.listeners.pop_front() {
            func(args);
        }
    }
}

//输出

pub type EventReg<A> = ReusableInner<A, CloneArgs>;
pub type MoveEventReg<A> = ReusableInner<A, MoveArgs>;
pub type OnceEventReg<A> = OnceInner<A, CloneArgs>;
pub type OnceMoveEventReg<A> = OnceInner<A, MoveArgs>;

fn test() {
    fn foo<F: Into<Box<dyn Fn(u32)>>>(f: F) {}
    impl<'a, F: Fn(u32) + 'a> Into<Box<dyn Fn(u32) + 'a>> for F {
        fn from(f: F) -> Self {
            Box::new(f)
        }
    }
    foo(|int| println!("Input: {}", int));
}

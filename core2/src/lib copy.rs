#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}

use std::{
    cell::{RefCell, RefMut},
    cmp::{Eq, PartialEq},
    collections::hash_map::{self,HashMap},
    error::Error,
    fmt,
    marker::PhantomData,
    //ops::DerefMut,
    sync::{
        atomic::{AtomicU64, Ordering},
        Mutex, MutexGuard,
    },
};

type Listeners<T> = HashMap<CancelHandle, T>;

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

#[derive(Debug, Clone, Copy)]
pub struct CanOnlyCallOnceError;

pub struct Local<T>(RefCell<T>);

pub struct Shared<T>(Mutex<T>);

pub struct Reusable<A>(ListenerManager<A,Listener<Box<dyn FnMut(A)>>>);

pub struct Once<A>(Option<ListenerManager<A,Box<dyn FnMut(A)>>>);

pub struct CloneValue;

pub struct MoveValue;

pub struct ListenerManager<A, F> {
    map: HashMap<CancelHandle, F>,
    _phantom: PhantomData<A>,
}

struct Listener<F>{
    func:F,
    once_info:Option<bool>
}

/*pub struct MoveExecutor<A,F>{
    map:HashMap<CancelHandle,F>,
    insert_order:
}*/

//实现多次触发单次事件的错误

impl fmt::Display for CanOnlyCallOnceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "This event can only be emitted once")
    }
}

impl Error for CanOnlyCallOnceError {}

//实现单次/多次检查

impl<'a,A> Reusable<A> {
    #[inline]
    fn new() -> Self {
        Reusable(ListenerManager::new())
    }

    #[inline]
    fn try_get(&'a mut self) -> Option<hash_map::ValuesMut<'a,CancelHandle,Box<dyn FnMut(A)>>> {
        Some(self.0.get().values_mut())
    }
}

impl<T> Once<T> {
    #[inline]
    fn new() -> Self {
        Once(Some(ListenerManager::new()))
    }

    fn try_get(&mut self) -> {

    }
}

//实现单线程/多线程的容器

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

//实现事件的触发方式

impl CloneValue{
    #[inline]
    fn emit<'a,A,F,I>(exe:I,arg:A)
    where
        A:Clone+'a,
        F:FnOnce(A)+'a,
        I:Iterator<Item=F>
    {
        for f in exe{
            f(arg.clone());
        }
    }
}

impl MoveValue{
    #[inline]
    fn emit<'a,A,F,I>(exe:I,arg:A)
    where
        A:Clone+'a,
        F:FnOnce(A)+'a,
        I:Iterator<Item=F>
    {
        if let Some(func)=exe.next(){
            func(arg);
        }
    }
}

//监听者管理

impl<F> Listener<F>{
    fn new(f:F,is_once:bool)->Self{
        Listener{
            func:f,
            once_info:if is_once {Some(false)} else {None}
        }
    }

    fn get(&mut self)->Option<&mut F>{
        match self.once_info{
            Some(false)=>{
                self.once_info=Some(true);
                Some(&mut self.func)
            },
            Some(true)=>None,
            None=>Some(&mut self.func)
        }
    }
}

impl<A, F> ListenerManager<A, F> {
    #[inline]
    fn new() -> Self {
        ListenerManager {
            map: HashMap::new(),
            _phantom: PhantomData,
        }
    }

    #[inline]
    fn with_capacity(n: usize) -> Self {
        ListenerManager {
            map: HashMap::with_capacity(n),
            _phantom: PhantomData,
        }
    }

    #[inline]
    fn get(&mut self)->&mut HashMap<CancelHandle, F>{
        &mut self.map
    }

    fn listen(&mut self, f: F) -> CancelHandle {
        let ch = CancelHandle::new();
        self.map.insert(ch.clone(), f);
        ch
    }

    #[inline]
    fn unlisten(&mut self, ch: CancelHandle) -> Option<F> {
        self.map.remove(&ch)
    }
}

/*pub struct Er<A>(RefCell<Listeners<dyn FnMut(A)>>);
pub struct Ser<A>(Mutex<Listeners<dyn FnMut(A)>>);
pub struct Oer<A>(RefCell<Option<Listeners<dyn FnOnce(A)>>>);*/
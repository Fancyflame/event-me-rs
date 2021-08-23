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
    collections::HashMap,
    error::Error,
    fmt,
    marker::PhantomData,
    ops::DerefMut,
    sync::{
        atomic::{AtomicU64, Ordering},
        Mutex, MutexGuard,
    },
};

type Listeners<T> = HashMap<CancelHandle, Box<T>>;

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

trait EmitCounts<A, C: ListenerContainer<A>> {
    fn _try_emit(&mut self) -> Option<&mut C>;
}

trait ListenerContainer<A> {
    type Target: DerefMut<Target = A>;
    fn _get_mut(self) -> Self::Target;
}

#[derive(Debug, Clone, Copy)]
pub struct CanOnlyCallOnceError;

struct Reusable<T>(T);

enum Once<T> {
    Callable(T),
    Called(T),
}

impl fmt::Display for CanOnlyCallOnceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "This event can only be emitted once")
    }
}

impl Error for CanOnlyCallOnceError {}

impl<A, C: ListenerContainer<A>> EmitCounts<A, C> for Reusable<C> {
    #[inline]
    fn _try_emit(&mut self) -> Option<&mut C> {
        Some(&mut self.0)
    }
}

impl<A, C: ListenerContainer<A>> EmitCounts<A, C> for Once<C> {
    #[inline]
    fn _try_emit(&mut self) -> Option<&mut C> {
        unsafe {
            match std::ptr::read(self) {
                Once::Callable(v) => {
                    std::ptr::write(self, Once::Called(v));
                    match self {
                        Once::Called(v) => Some(v),
                        _ => unreachable!(),
                    }
                }
                Once::Called(_) => None,
            }
        }
    }
}

impl<'a, A> ListenerContainer<A> for &'a RefCell<A> {
    type Target = RefMut<'a, A>;
    #[inline]
    fn _get_mut(self) -> Self::Target {
        self.borrow_mut()
    }
}

impl<'a, A> ListenerContainer<A> for &'a Mutex<A> {
    type Target = MutexGuard<'a, A>;
    #[inline]
    fn _get_mut(self) -> Self::Target {
        self.lock().unwrap()
    }
}

/*pub struct Er<A>(RefCell<Listeners<dyn FnMut(A)>>);
pub struct Ser<A>(Mutex<Listeners<dyn FnMut(A)>>);
pub struct Oer<A>(RefCell<Option<Listeners<dyn FnOnce(A)>>>);*/

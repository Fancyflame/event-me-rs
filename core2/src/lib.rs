#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}

use std::{
    sync::{
        atomic::{ AtomicU64, Ordering },
        Mutex, MutexGuard
    },
    cell::{ RefCell, Ref },
    collections::HashMap,
    marker::PhantomData
};

type Listeners<T> = HashMap<CancelHandle, Box<T>>;

macro_rules! _event {
    ($name:ident,$shared:tt,$once:tt,$move:tt,$lock:ty,$fnty:path) => {
        pub struct $name(
            _if! $shared{
                Mutex<_if!{ if $once {Option<Listeners<dyn $fnty>>} else {Listeners<dyn $fnty>} }>
            }else{
                RefCell<_if!{ if $once {Option<Listeners<dyn $fnty>>} else {Listeners<dyn $fnty>} }>
            }
        );
        impl<A> $name<A>{
            pub fn new()->Self{
                $name(HashMap::new().into())
            }

            pub fn with_capacity(n:usize)->Self{
                $name(HashMap::with_capacity(n).into())
            }

            pub fn listen<F:$fnty>(&self,func:F)->CancelHandle{
                let ch=CancelHandle::new();
                _fetch_mut!($shared).insert(ch.clone(),func);
                ch
            }

            pub fn unlisten(&self,ch:CancelHandle)->Option<Box<dyn $fnty>>{
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

macro_rules! _fetch_mut{
    ($shared:tt)=>{
        _if!{
            if $shared{
                self.0.lock()
            }else{
                self.0.borrow_mut()
            }
        }
    }
}


#[derive(Clone)]
pub struct CancelHandle(u64);


impl CancelHandle{
    fn new()->Self{
        static COUNTER:AtomicU64=AtomicU64::new(0);
        CancelHandle(COUNTER.fetch_add(1,Ordering::Relaxed))
    }
}


_event!(EventReg,               0, 0, 0, RefCell,    FnMut(A));
_event!(SharedEventReg,         1, 0, 0, Mutex, FnMut(A)+Send);
_event!(OnceEventReg,           0, 1, 0, RefCell, FnOnce(A));
_event!(MoveEventReg,           0, 0, 1, RefCell, FnMut(A));
_event!(SharedOnceEventReg,     1, 1, 0, Mutex, FnOnce(A)+Send);
_event!(OnceMoveEventReg,       0, 1, 1, RefCell,FnOnce(A));
_event!(SharedMoveEventReg,     1, 0, 1, Mutex,FnMut(A)+Send);
_event!(SharedOnceMoveEventReg, 1, 1, 1, Mutex,FnOnce(A)+Send);



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
trait ExecArgsProcess {}
pub trait MultiThreadExecutor {
    fn exec<A:Send+'static>(f: SharedCallable<A>, args: A);
}
pub trait LocalThreadExecutor{
    fn exec<A>(f:Callable<'_,A>,args:A);
}
trait Into2<T> {
    fn into2(self) -> T;
}

/*struct Once;
struct Multiple;
impl ExecCounts for Once {}
impl ExecCounts for Multiple {}*/

pub struct Moving;
pub struct Cloning;
//impl ExecArgsProcess for Moving {}
//impl ExecArgsProcess for Cloning {}

pub struct MultiThread;
pub struct LocalThread;
//impl Executor<'static> for MultiThread{}
//impl<'a> Executor<'a> for LocalThread{}

pub enum Listener<'a, A> {
    Once(Box<dyn FnOnce(A) + 'a>),
    Multiple(Box<dyn FnMut(A) + 'a>),
    Called,
}

pub enum SharedListener<A:Send+'static> {
    Once(Box<dyn FnOnce(A) + Send + 'static>),
    Multiple(Arc<dyn Fn(A) + Send + Sync + 'static>),
    Called,
}

pub enum Callable<'a, A> {
    BoxedFnOnce(Box<dyn FnOnce(A) + 'a>),
    RefFnMut(&'a mut (dyn FnMut(A) + 'a)),
}

pub enum SharedCallable<A: Send+'static> {
    BoxedFnOnce(Box<dyn FnOnce(A) + Send + 'static>),
    ArcFn(Arc<dyn Fn(A) + Send + Sync + 'static>),
}

pub struct ListenerManager<F, P, E> {
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

impl<A:Send> SharedListener<A> {
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

impl<F, P, E> ListenerManager<F, P, E> {
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

impl<'a,A,P,E> ListenerManager<Listener<'a,A>,P,E>{
    #[inline]
    pub fn listen(&mut self,f:impl FnMut(A)+'a)->CancelHandle{
        self._listen(Listener::from_fn_mut(f))
    }

    #[inline]
    pub fn listen_once(&mut self,f:impl FnOnce(A)+'a)->CancelHandle{
        self._listen(Listener::from_fn_once(f))
    }

    #[inline]
    pub fn unlisten(&mut self,ch:CancelHandle)->Option<Listener<'a,A>>{
        self._unlisten(ch)
    }
}

impl<A:Send,P,E:MultiThreadExecutor> ListenerManager<SharedListener<A>,P,E>{
    #[inline]
    pub fn listen(&mut self,f:impl Fn(A)+Send+Sync+'static)->CancelHandle{
        self._listen(SharedListener::from_fn(f))
    }

    #[inline]
    pub fn listen_once(&mut self,f:impl FnOnce(A)+Send+'static)->CancelHandle{
        self._listen(SharedListener::from_fn_once(f))
    }

    #[inline]
    pub fn unlisten(&mut self,ch:CancelHandle)->Option<SharedListener<A>>{
        self._unlisten(ch)
    }
}

//实现

macro_rules! _impl{
    (cloning)=>{
        pub fn emit(&mut self,args:A){
            for (_,x) in self.listeners.iter_mut(){
                E::exec(x.get().0,args.clone());
            }
            self.listeners.retain(|(_,x)|!x.needs_drop());
        }
    };
    (moving)=>{
        pub fn emit(&mut self,args:A){
            let (c,needs_drop)=self.listeners.front_mut().unwrap().1.get();
            E::exec(c,args);
            if needs_drop{
                self.listeners.pop_front();
            }
        }
    }
}

impl<'a,A:Clone,E:LocalThreadExecutor> ListenerManager<Listener<'a,A>,Cloning,E>{
    _impl!(cloning);
}

impl<'a,A,E:LocalThreadExecutor> ListenerManager<Listener<'a,A>,Moving,E>{
    _impl!(moving);
}

impl<'a,A:Clone+Send+'static,E:MultiThreadExecutor> ListenerManager<SharedListener<A>,Cloning,E>{
    _impl!(cloning);
}

impl<'a,A:Send+'static,E:MultiThreadExecutor> ListenerManager<SharedListener<A>,Moving,E>{
    _impl!(moving);
}

//提供两个默认模板

impl LocalThreadExecutor for LocalThread{
    #[inline]
    fn exec<A>(f:Callable<'_,A>,args:A){
        f.call(args);
    }
}

impl MultiThreadExecutor for MultiThread{
    #[inline]
    fn exec<A:Send+'static>(f:SharedCallable<A>,args:A){
        std::thread::spawn(||f.call(args));
    }
}


pub type LocalCloneEvent<'a,A> = ListenerManager<Listener<'a,A>,Cloning,LocalThread>;
pub type LocalMoveEvent<'a,A> = ListenerManager<Listener<'a,A>,Moving,LocalThread>;
pub type SharedCloneEvent<A> = ListenerManager<SharedListener<A>,Cloning,MultiThread>;
pub type SharedMoveEvent<A> = ListenerManager<SharedListener<A>,Moving,MultiThread>;


#[test]
fn test1(){
    let k=std::cell::Cell::new(0);
    let mut a=LocalCloneEvent::<u32,LocalThread>::new();

    a.listen(|num|{
        k.set(num);
    });

    a.listen_once(|num|{
        k.set(num+100);
    });

    a.emit(100);
    assert_eq!(k.get(),200);

    a.emit(100);
    assert_eq!(k.get(),100);
}

#[test]
fn test2(){
    let mut a=SharedMoveEvent::<u32>::new();
    let mutex=std::sync::Arc::new(std::sync::Mutex::new(0));

    let m=mutex.clone();
    a.listen_once(move|num|{
        *m.lock().unwrap()+=1;
        assert_eq!(num,1);
    });

    let m=mutex.clone();
    let ch=a.listen_once(move|num|{
        *m.lock().unwrap()+=2;
        assert_eq!(num,2);
    });

    let m=mutex.clone();
    a.listen(move|num|{
        *m.lock().unwrap()+=num;
        assert!(num==2||num==3);
    });

    a.unlisten(ch);
    a.emit(1);
    a.emit(2);
    a.emit(3);
    std::thread::sleep(std::time::Duration::from_millis(100));
    assert_eq!(*mutex.lock().unwrap(),6);
}

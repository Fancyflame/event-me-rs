#![warn(dead_code)]
use std::{
    ops::{Deref, DerefMut},
    sync::{atomic::*, Arc, Mutex},
};

struct MemExtender<'a, T> {
    next_block_size: usize,
    used_chunks: Vec<Box<[Node<'a, T>]>>,
}

pub struct Container<'a, T>
where
    Self: 'a,
{
    next_free: AtomicPtr<Node<'a, T>>,
    allocater: Mutex<MemExtender<'a, T>>,
}

pub struct LinkedList<'a, T> {
    trunk: &'a Container<'a, T>,
}

pub struct NodeRef<'a, T>(*mut Node<'a, T>);

struct Node<'a, T> {
    ctnr: &'a Container<'a, T>,
    state: Option<T>,
    next: *mut Node<'a, T>,
}

impl<'a, T> Container<'a, T> {
    pub fn new() -> Self {
        Container::<T> {
            next_free: AtomicPtr::new(std::ptr::null_mut()),
            allocater: Mutex::new(MemExtender {
                next_block_size: 4,
                used_chunks: Vec::new(),
            }),
        }
    }

    fn alloc(&'a self) -> *mut Node<'a, T> {
        let mut node: *mut Node<'a, T>;
        let mut old = self.next_free.load(Ordering::Relaxed);
        loop {
            let ptr = match unsafe { old.as_mut() } {
                Some(f) => {
                    //不是null，直接获取下一个节点
                    let n = f.next;
                    node = old;
                    n
                }
                None => {
                    //没空间了
                    match self.allocater.try_lock() {
                        Ok(mut lock) => {
                            let mut block = Vec::<Node<'a, T>>::with_capacity(lock.next_block_size);
                            let offset = block.as_mut_ptr();

                            unsafe {
                                //初始化除了最后一个以外所有元素
                                for x in 0..lock.next_block_size - 1 {
                                    *offset.add(x) = Node::<'a, T> {
                                        ctnr: self,
                                        state: None,
                                        next: offset.add(x + 1),
                                    }
                                }

                                //初始化最后一个
                                *offset.add(lock.next_block_size - 1) = Node::<'a, T> {
                                    ctnr: self,
                                    state: None,
                                    next: std::ptr::null_mut(),
                                };

                                block.set_len(lock.next_block_size);
                                lock.used_chunks.push(block.into_boxed_slice());
                                lock.next_block_size *= 2; //每次添加储存后设置下次储存为上次的2倍
                                node = offset;
                                offset.add(1)
                            }
                        }
                        Err(_) => {
                            let _ = self.allocater.lock().unwrap(); //仅阻塞等待
                            node = self.alloc();
                            continue;
                        }
                    }
                }
            };

            match self.next_free.compare_exchange_weak(
                old,
                ptr,
                Ordering::SeqCst,
                Ordering::Relaxed,
            ) {
                Ok(_) => {
                    unsafe{(*node).next=std::ptr::null_mut();}
                    break node;
                },
                Err(x) => old = x,
            }
        }

    }

    unsafe fn free(&self, f: *mut Node<'a, T>) {
        self.next_free
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |free| {
                (*f).next = free;
                Some(f)
            })
            .unwrap();
    }
}

impl<T> Drop for NodeRef<'_, T> {
    fn drop(&mut self) {
        unsafe {
            (*self.0).ctnr.free(self.0);
        }
    }
}

impl<'a, T> NodeRef<'a, T> {
    #[inline]
    pub fn get_new(from: &'a Container<'a, T>) -> Self {
        from.alloc().into()
    }
}

impl<'a, T> From<*mut Node<'a, T>> for NodeRef<'a, T> {
    #[inline]
    fn from(f: *mut Node<'a, T>) -> Self {
        NodeRef(f)
    }
}

impl<'a, T> Deref for NodeRef<'a, T> {
    type Target = Option<T>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        unsafe { &(*self.0).state }
    }
}

impl<'a, T> DerefMut for NodeRef<'a, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut (*self.0).state }
    }
}

#[test]
fn it_works() {
    use std::time::Instant;
    let ctnr = Container::<&'static str>::new();
    let a = AtomicUsize::new(0);
    let mut v = std::sync::Mutex::new(());
    let ins1 = Instant::now();
    for _ in 0..10_0000 {
        NodeRef::get_new(&ctnr);
        //v=a.swap(1,Ordering::Relaxed);
        /*
         *
        *a = Some("hello");
        let mut b = NodeRef::new(&ctnr);
        *b = Some("world");
        let c = NodeRef::new(&ctnr);
        println!("{} {}", a.unwrap(), b.unwrap());
        drop(a);
        */
    }
    println!("{:?}", ins1.elapsed());
    //format!("{}",v);

    let lock = ctnr.allocater.lock().unwrap();
    println!("{},{}", lock.used_chunks.len(), lock.next_block_size);
    drop(lock);

    let mut v = Box::new("mi");
    let ins2 = Instant::now();
    for _ in 0..10_0000 {
        v=Box::new("kilikili");
    }
    format!("{}",v);
    println!("{:?}", ins2.elapsed());
}

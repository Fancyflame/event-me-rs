use std::{
    sync::{
        Mutex
    },
    task::Waker,
};

static LISTENERS_LIST: Mutex<ListenersList> = Mutex::new(
    ListenersList{
        wakers:Vec::new(),
        free_node:NULL_INDEX
    }
);

const NULL_INDEX: usize = 0;


struct ListenersList{
    wakers:Vec<(Option<Waker>, usize)>,
    free_node:usize
}


pub struct List{
    start:usize
}


impl List{
    pub fn request_new() -> Self{
        let mut mutex=LISTENERS_LIST.lock();
        let w=&mut mutex.wakers;
        let mut index=mutex.free_node;

        if index == NULL_INDEX {
            //未初始化
            w.reserve(16);
            w.push((None, NULL_INDEX));
            for x in 2..15 {
                w.push((None, x));
            }
            w.push((None, NULL_INDEX));
            index=1;
        }

        let item = w[index];
        assert_eq!(item.0, None);

        //将free_node指向下一个空闲节点
        if item.1 == NULL_INDEX {
            //新增位置
            w.push((None,NULL_INDEX));
            mutex.free_node=w.len()-1;
        }else{
            mutex.free_node=item.1;
        }

        NodePointer{
            target:index
        }
    }
}


impl Drop for NodePointer{
    fn drop(&mut self){
        assert_ne!(self.target, NULL_INDEX);

        let mut mutex=LISTENERS_LIST.lock();
        mutex.waker[self.target]

    }
}

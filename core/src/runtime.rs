use std::{sync::atomic::*, task::Waker};

static mut LISTENERS_LIST: ListenersList = ListenersList {
    wakers: Vec::new(),
    free_node: NULL_INDEX,
};

const NULL_INDEX: usize = 0;

struct ListenersList {
    wakers: Vec<Node>,
    free_node: usize,
}

struct Node {
    pub content: Option<Waker>,
    next_index: usize,
}

pub struct List {
    start_index: usize,
    end_index: usize,
}

impl Node {
    fn new() -> Self {
        Node {
            content: None,
            next_index: NULL_INDEX,
        }
    }

    #[inline]
    fn take(&mut self) -> Option<Waker> {
        self.content.take()
    }

    #[inline]
    fn connect_to(&mut self, next: usize) {
        self.next_index = next;
    }
}

impl List {
    pub const fn new() -> Self {
        List {
            start_index: NULL_INDEX,
            end_index: NULL_INDEX,
        }
    }

    pub fn push(&mut self, w: Waker) {
        let new = Self::request_node();
    }

    fn request_node() -> usize {
        let ListenersList {
            ref mut wakers,
            ref mut free_node,
        } = unsafe { &mut LISTENERS_LIST };

        if wakers.len() == 0 {
            //未初始化
            wakers.reserve(16);
            wakers.push(Node::new());
            for x in 2..15 {
                wakers.push(Node {
                    content: None,
                    next_index: x,
                });
            }
            wakers.push(Node::new());
            *free_node = 1;
        }

        if *free_node != NULL_INDEX {
            let output = *free_node;
            let item = &mut wakers[output];
            item.content = None;
            //将free_node指向下一个空闲节点
            *free_node = item.next_index;
            output
        } else {
            wakers.push(Node::new());
            wakers.len() - 1
        }
    }
}

impl Drop for List {
    fn drop(&mut self) {
        unsafe {
            let fr = LISTENERS_LIST.free_node;
            LISTENERS_LIST.free_node = self.start_index;
            LISTENERS_LIST.wakers[self.end_index].next_index = fr;
        }
    }
}

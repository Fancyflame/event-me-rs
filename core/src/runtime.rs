use std::{
    sync::atomic::{AtomicUsize, Ordering},
    task::Waker,
};

static mut LISTENERS_LIST: Vec<(Option<Waker>, usize)> = Vec::new();

static FREE_POS_INDEX: AtomicUsize = AtomicUsize::new(NULL_INDEX); //0代表null

const NULL_INDEX: usize = 0;

pub fn alloc() -> usize {
    let mut ind = FREE_POS_INDEX.load(Ordering::AcqRel);

    if ind == NULL_INDEX {
        unsafe {
            LISTENERS_LIST.reserve(16);
            LISTENERS_LIST.push((None, NULL_INDEX));
            for x in 2..15 {
                LISTENERS_LIST.push((None, x));
            }
            LISTENERS_LIST.push((None, NULL_INDEX));
        }
        ind = 1;
    }

    let item = LISTENERS_LIST[ind];
    assert_eq!(item.0, None);

    if item.1 == NULL_INDEX {
        //TODO
    }

    FREE_POS_INDEX.store(item.1, Ordering::AcqRel);
    ind
}

pub fn free(ind: usize) {
    assert_ne!(ind, NULL_INDEX);

    let mut free_pos = FREE_POS_INDEX.load(Ordering::AcqRel);
    assert_ne!(free_pos, NULL_INDEX);
    unsafe {
        let item = LISTENERS_LIST[ind];
        item.0 = None;
        item.1 = free_pos; //指针指向上一个空位
    }
    FREE_POS_INDEX.store(ind, Ordering::AcqRel);
}

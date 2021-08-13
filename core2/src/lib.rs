#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}

macro_rules! __event {
    ($name:ident,$local:literal,$once:tt,$move:literal) => {
        pub struct $name<A>(Vec<Box<__if! {if $once {dyn FnOnce(A)} else {dyn Fn(A)}}>>);
    };
}

macro_rules! __if {
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

__event!(EventReg, 0, 0, 0);
__event!(LocalEventReg, 1, 0, 0);
__event!(OnceEventReg, 0, 1, 0);
__event!(MoveEventReg, 0, 0, 1);
__event!(LocalOnceEventReg, 1, 1, 0);
__event!(OnceMoveEventReg, 0, 1, 1);
__event!(LocalMoveEventReg, 1, 0, 1);
__event!(LocalOnceMoveEventReg, 1, 1, 1);

macro_rules! foo {
    ($opt:tt) => {
        bar!($opt);
    };
}

macro_rules! bar {
    (0) => {};
    (1) => {};
}

foo!(0);

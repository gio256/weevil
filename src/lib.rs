#[cfg(test)]
mod loom {
    use std::rc::Rc;

    use loom::sync::atomic::AtomicUsize;
    use loom::sync::atomic::Ordering::{Acquire, Release, Relaxed};
    use loom::sync::{Arc, Mutex};
    use loom::thread;

    #[test]
    #[should_panic]
    fn test_deadlock() {
        loom::model(|| {
            let a = Rc::new(Mutex::new(1));
            let b = Rc::new(Mutex::new(2));

            let t0 = {
                let a = a.clone();
                let b = b.clone();
                thread::spawn(move || {
                    let a = a.lock().unwrap();
                    let b = b.lock().unwrap();
                    assert_eq!(*a + *b, 3);
                })
            };
            let t1 = thread::spawn(move || {
                let b = b.lock().unwrap();
                let a = a.lock().unwrap();
                assert_eq!(*a + *b, 3);
            });
            t0.join().unwrap();
            t1.join().unwrap();
        });
    }

    #[test]
    #[should_panic]
    fn test_inc() {
        loom::model(|| {
            let val = Arc::new(AtomicUsize::new(0));
            let threads: Vec<_> = (0..2)
                .map(|_| {
                    let val = val.clone();
                    thread::spawn(move || {
                        let cur = val.load(Acquire);
                        val.store(cur + 1, Release);
                    })
                })
                .collect();
            for t in threads {
                t.join().unwrap();
            }
            assert_eq!(val.load(Relaxed), 2);
        });
    }

    #[test]
    #[ignore]
    #[should_panic]
    fn test_relaxed() {
        loom::model(|| {
            let a = Arc::new(AtomicUsize::new(0));
            let b = Arc::new(AtomicUsize::new(0));
            let t0 = {
                let a = a.clone();
                let b = b.clone();
                thread::spawn(move || {
                    let res = b.load(Relaxed);
                    a.store(1, Relaxed);
                    res
                })
            };
            let t1 = thread::spawn(move || {
                let res = a.load(Relaxed);
                b.store(1, Relaxed);
                res
            });
            let res0 = t0.join().unwrap();
            let res1 = t1.join().unwrap();
            assert!(!(res0 == 1 && res1 == 1));
        });
    }
}

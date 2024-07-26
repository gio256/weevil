#[cfg(test)]
mod loom {
    use std::rc::Rc;

    use loom::sync::{Arc, Mutex};
    use loom::sync::atomic::AtomicUsize;
    use loom::sync::atomic::Ordering;
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
                        let cur = val.load(Ordering::Acquire);
                        val.store(cur + 1, Ordering::Release);
                    })
                })
                .collect();
            for t in threads {
                t.join().unwrap();
            }
            assert_eq!(val.load(Ordering::Relaxed), 2);
        });
    }
}

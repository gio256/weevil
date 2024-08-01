#[cfg(test)]
mod loom {
    use std::rc::Rc;

    use loom::sync::atomic::{AtomicBool, AtomicUsize};
    use loom::sync::atomic::Ordering::{Acquire, Release, Relaxed};
    use loom::sync::{Arc, Mutex};
    use loom::thread;

    #[test]
    fn test_publication() {
        loom::model(|| {
            let flag = Arc::new(AtomicBool::new(false));
            let data = Arc::new(AtomicUsize::new(0));
            let j1 = {
                let flag = flag.clone();
                let data = data.clone();
                thread::spawn(move || {
                    data.store(1, Relaxed);
                    flag.store(true, Release);
                })
            };
            let j2 = thread::spawn(move || {
                if flag.load(Acquire) {
                    assert!(data.load(Relaxed) == 1);
                }
            });
            j1.join().unwrap();
            j2.join().unwrap();
        })
    }

    #[test]
    #[should_panic]
    fn test_deadlock() {
        loom::model(|| {
            let x = Rc::new(Mutex::new(1));
            let y = Rc::new(Mutex::new(2));

            let j1 = {
                let x = x.clone();
                let y = y.clone();
                thread::spawn(move || {
                    let x = x.lock().unwrap();
                    let y = y.lock().unwrap();
                    assert_eq!(*x + *y, 3);
                })
            };
            let j2 = thread::spawn(move || {
                let y = y.lock().unwrap();
                let x = x.lock().unwrap();
                assert_eq!(*x + *y, 3);
            });
            j1.join().unwrap();
            j2.join().unwrap();
        });
    }

    /// This test reflects the weakening of release sequences in C++20:
    /// <http://www.open-std.org/jtc1/sc22/wg21/docs/papers/2018/p0982r1.html>
    #[test]
    #[should_panic]
    fn test_release_sequence() {
        loom::model(|| {
            let flag = Arc::new(AtomicBool::new(false));
            let data = Arc::new(AtomicUsize::new(0));
            let j1 = {
                let flag = flag.clone();
                let data = data.clone();
                thread::spawn(move || {
                    data.store(1, Relaxed);             // A
                    flag.store(true, Release);          // B
                    // The store C breaks the release sequence headed by B,
                    // so D reads from C without being synchronized with B.
                    flag.store(true, Relaxed);          // C
                })
            };
            let j2 = thread::spawn(move || {
                if flag.load(Acquire) {                 // D
                    assert!(data.load(Relaxed) == 1);   // E
                }
            });
            j1.join().unwrap();
            j2.join().unwrap();
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
    #[should_panic]
    fn test_store_buffering() {
        loom::model(|| {
            let x = Arc::new(AtomicUsize::new(0));
            let y = Arc::new(AtomicUsize::new(0));
            let j1 = {
                let (x, y) = (x.clone(), y.clone());
                thread::spawn(move || {
                    x.store(1, Relaxed);
                    y.load(Relaxed)
                })
            };
            let j2 = thread::spawn(move || {
                y.store(1, Relaxed);
                x.load(Relaxed)
            });
            let res0 = j1.join().unwrap();
            let res1 = j2.join().unwrap();
            assert!(!(res0 == 0 && res1 == 0));
        })
    }

    #[test]
    #[ignore]
    #[should_panic]
    fn test_relaxed() {
        loom::model(|| {
            let x = Arc::new(AtomicUsize::new(0));
            let y = Arc::new(AtomicUsize::new(0));
            let j1 = {
                let x = x.clone();
                let y = y.clone();
                thread::spawn(move || {
                    let res = y.load(Relaxed);
                    x.store(1, Relaxed);
                    res
                })
            };
            let j2 = thread::spawn(move || {
                let res = x.load(Relaxed);
                y.store(1, Relaxed);
                res
            });
            let res0 = j1.join().unwrap();
            let res1 = j2.join().unwrap();
            assert!(!(res0 == 1 && res1 == 1));
        });
    }
}

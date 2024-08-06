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
            let res1 = j1.join().unwrap();
            let res2 = j2.join().unwrap();
            assert!(!(res1 == 0 && res2 == 0));
        })
    }

    #[test]
    #[should_panic]
    fn test_message_passing() {
        loom::model(|| {
            let x = Arc::new(AtomicUsize::new(0));
            let y = Arc::new(AtomicUsize::new(0));
            let j1 = {
                let (x, y) = (x.clone(), y.clone());
                thread::spawn(move || {
                    x.store(1, Relaxed);
                    y.store(1, Relaxed);
                })
            };
            let j2 = {
                let (x, y) = (x.clone(), y.clone());
                thread::spawn(move || {
                    let y_read = y.load(Relaxed);
                    let x_read = x.load(Relaxed);
                    (x_read, y_read)
                })
            };
            let j3 = thread::spawn(move || {
                let x_read = x.load(Relaxed);
                let y_read = y.load(Relaxed);
                (x_read, y_read)
            });
            j1.join().unwrap();
            let res2 = j2.join().unwrap();
            let res3 = j3.join().unwrap();
            assert!(!(res2 == (1, 0) && res3 == (0, 1)));
        })
    }

    /// Independent reads of independent writes.
    #[test]
    #[should_panic]
    fn test_iriw() {
        loom::model(|| {
            let x = Arc::new(AtomicUsize::new(0));
            let y = Arc::new(AtomicUsize::new(0));
            let j1 = {
                let x = x.clone();
                thread::spawn(move || {
                    x.store(1, Release);
                })
            };
            let j2 = {
                let y = y.clone();
                thread::spawn(move || {
                    y.store(1, Release);
                })
            };
            let j3 = {
                let (x, y) = (x.clone(), y.clone());
                thread::spawn(move || {
                    let x_read = x.load(Acquire);
                    let y_read = y.load(Acquire);
                    (x_read, y_read)
                })
            };
            let j4 = thread::spawn(move || {
                let y_read = y.load(Acquire);
                let x_read = x.load(Acquire);
                (x_read, y_read)
            });
            j1.join().unwrap();
            j2.join().unwrap();
            let res3 = j3.join().unwrap();
            let res4 = j4.join().unwrap();
            assert!(!(res3 == (1, 0) && res4 == (0, 1)));
        });
    }

    #[test]
    fn test_ok_iriw() {
        loom::model(|| {
            let x = Arc::new(AtomicUsize::new(0));
            let y = Arc::new(AtomicUsize::new(0));
            let j1 = {
                let (x, y) = (x.clone(), y.clone());
                thread::spawn(move || {
                    x.store(1, Release);
                    y.store(1, Release);
                })
            };
            let j2 = {
                let (x, y) = (x.clone(), y.clone());
                thread::spawn(move || {
                    let x_read = x.load(Acquire);
                    let y_read = y.load(Acquire);
                    (x_read, y_read)
                })
            };
            let j3 = thread::spawn(move || {
                let y_read = y.load(Acquire);
                let x_read = x.load(Acquire);
                (x_read, y_read)
            });
            j1.join().unwrap();
            let res2 = j2.join().unwrap();
            let res3 = j3.join().unwrap();
            assert!(!(res2 == (1, 0) && res3 == (0, 1)));
        });
    }

    #[test]
    fn test_write_read_causality() {
        loom::model(|| {
            let x = Arc::new(AtomicUsize::new(0));
            let y = Arc::new(AtomicUsize::new(0));
            let j1 = {
                let x = x.clone();
                thread::spawn(move || {
                    x.store(1, Relaxed);
                })
            };
            let j2 = {
                let (x, y) = (x.clone(), y.clone());
                thread::spawn(move || {
                    let res = x.load(Relaxed);
                    y.store(1, Relaxed);
                    res
                })
            };
            let j3 = thread::spawn(move || {
                let y_load = y.load(Relaxed);
                let x_load = x.load(Relaxed);
                (x_load, y_load)
            });
            j1.join().unwrap();
            let x2 = j2.join().unwrap();
            let (x3, y3) = j3.join().unwrap();
            assert!(!(x2 == 1 && y3 == 1 && x3 == 0));
        });
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
            let res1 = j1.join().unwrap();
            let res2 = j2.join().unwrap();
            assert!(!(res1 == 1 && res2 == 1));
        });
    }
}

use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
    thread, time::Duration,
};

fn lock_test() {
    let mut counter = Arc::new(Mutex::new(0));
    let mut counter_clone = Arc::clone(&counter);
    let t = thread::spawn(move || {
        println!("Enter thread!");
        thread::sleep(Duration::from_secs(1));
        let mut cur = counter_clone.lock().unwrap();
        *cur += 1;
        println!("Leave thread!");
    });
    {
        let mut counter = Arc::clone(&counter);
        let mut cur = counter.lock().unwrap();
        thread::sleep(Duration::from_secs(10));
        *cur += 1;
    }
    println!("Lock released!");
    t.join();
}

fn counter_test() {
    let mut counter = Arc::new(Mutex::new(0));
    let threads = (0..2)
        .map(|_| {
            let mut counter = Arc::clone(&counter);
            thread::spawn(move || {
                let mut value = {
                    let curr = counter.lock().unwrap();
                    *curr
                };
                value += 1;
                *counter.lock().unwrap() = value;
            })
        })
        .collect::<Vec<_>>();

    for thread in threads {
        thread.join().unwrap();
    }
    assert_eq!(*counter.lock().unwrap(), 2);
}

fn counter_test_atomic() {
    let counter = Arc::new(AtomicUsize::new(0));
    let threads = (0..5)
        .map(|_| {
            let counter = Arc::clone(&counter);
            thread::spawn(move || {
                let curr = counter.load(Ordering::SeqCst);
                counter.store(curr + 1, Ordering::SeqCst);
            })
        })
        .collect::<Vec<_>>();

    for thread in threads {
        thread.join().unwrap();
    }
    assert_eq!(counter.load(Ordering::SeqCst), 5);
}

fn interleave_test() {
    let threads = (0..2)
        .map(|it| {
            thread::spawn(move || {
                for i in 0..100 {
                    println!("{}", it);
                    if i == 50 {
                        thread::yield_now()
                    }
                }
            })
        })
        .collect::<Vec<_>>();

    for thread in threads {
        thread.join().unwrap();
    }
}

fn preemption_test() {
    let threads = (0..3)
        .map(|it| {
            thread::spawn(move || {
                for i in 0..10 {
                    println!("{}", it);
                    for i in 0..100000000  {
                    }
                }
            })
        })
        .collect::<Vec<_>>();

    for thread in threads {
        thread.join().unwrap();
    }
}

pub fn main() {
    preemption_test();
}

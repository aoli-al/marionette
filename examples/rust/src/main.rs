use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    thread, io::{self, Write}, time::Duration,
};



use libc::{pthread_mutex_t, pthread_mutex_init, pthread_mutex_lock, pthread_mutex_trylock, pthread_mutex_unlock};
use std::mem;
use std::ptr;
use std::cell::UnsafeCell;
use std::ops::{Deref, DerefMut};

pub struct Mutex<T> {
    mutex: pthread_mutex_t,
    data: UnsafeCell<T>,
}

pub struct MutexGuard<'a, T> {
    mutex: &'a Mutex<T>,
}

impl<T> Deref for MutexGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.mutex.data.get() }
    }
}

impl<T> DerefMut for MutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.mutex.data.get() }
    }
}

impl<T> Drop for MutexGuard<'_, T> {
    fn drop(&mut self) {
        let result = unsafe { pthread_mutex_unlock(&self.mutex.mutex as *const _ as *mut _) };
        if result != 0 {
            panic!("Failed to unlock mutex");
        }
    }
}

unsafe impl<T: Send> Send for Mutex<T> {}
unsafe impl<T: Send> Sync for Mutex<T> {}


impl<T> Mutex<T> {
    /// Creates a new mutex
    pub fn new(data: T) -> Self {
        let mut mutex = unsafe { mem::zeroed() };
        let result = unsafe { pthread_mutex_init(&mut mutex, ptr::null()) };
        if result != 0 {
            panic!("Failed to initialize mutex");
        }

        Mutex {
            mutex,
            data: UnsafeCell::new(data),
        }
    }

    /// Locks the mutex, blocking the current thread until it is able to do so.
    pub fn lock(&self) -> MutexGuard<T> {
        let result = unsafe { pthread_mutex_lock(&self.mutex as *const _ as *mut _) };
        thread::yield_now();
        if result != 0 {
            panic!("Failed to lock mutex");
        }

        MutexGuard { mutex: self }
    }

    /// Tries to lock the mutex. If it is already locked, it will return None.
    pub fn try_lock(&self) -> Option<MutexGuard<T>> {
        let result = unsafe { pthread_mutex_trylock(&self.mutex as *const _ as *mut _) };
        if result == 0 {
            Some(MutexGuard { mutex: self })
        } else {
            None
        }
    }
}

fn counter_test_std_mutex() {
    let counter = Arc::new(std::sync::Mutex::new(0));
    let threads = (0..2)
        .map(|id| {
            let counter = Arc::clone(&counter);
            let t = thread::spawn(move || {
                let mut value = {
                    println!("id: {}", id);
                    io::stdout().flush().unwrap();
                    let curr = counter.lock().unwrap();
                    println!("id: {}, cur: {}", id, *curr);
                    io::stdout().flush().unwrap();
                    *curr
                };
                println!("id: {}, value: {}", id, value);
                io::stdout().flush().unwrap();
                value += 1;
                *counter.lock().unwrap() = value;
            });
            t
        })
        .collect::<Vec<_>>();

    for thread in threads {
        thread.join().unwrap();
    }
    assert_eq!(*counter.lock().unwrap(), 2);
}


fn counter_test() {
    let counter = Arc::new(Mutex::new(0));
    let threads = (0..2)
        .map(|id| {
            let counter: Arc<Mutex<i32>> = Arc::clone(&counter);
            let t = thread::spawn(move || {
                let mut value = {
                    println!("id: {}", id);
                    io::stdout().flush().unwrap();
                    let curr: MutexGuard<'_, i32> = counter.lock();
                    println!("id: {}, cur: {}", id, *curr);
                    io::stdout().flush().unwrap();
                    *curr
                };
                println!("id: {}, value: {}", id, value);
                io::stdout().flush().unwrap();
                value += 1;
                *counter.lock() = value;
            });
            t
        })
        .collect::<Vec<_>>();

    for thread in threads {
        thread.join().unwrap();
    }
    assert_eq!(*counter.lock(), 2);
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
                for _i in 0..10 {
                    println!("{}", it);
                    thread::sleep(Duration::from_secs(1));
                }
            })
        })
        .collect::<Vec<_>>();

    for thread in threads {
        thread.join().unwrap();
    }
}

pub fn main() {
    counter_test();
    // counter_test_std_mutex();
    // preemption_test();
}

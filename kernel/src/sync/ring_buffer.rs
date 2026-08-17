use core::{
    cell::UnsafeCell,
    sync::atomic::{AtomicUsize, Ordering},
};

use alloc::{boxed::Box, vec::Vec};

pub struct AtomicRingBuffer<T, const N: usize> {
    data: [UnsafeCell<T>; N],
    head: AtomicUsize,
    tail: AtomicUsize,
}

impl<T, const N: usize> AtomicRingBuffer<T, N> {
    pub const fn new() -> Self
    where
        T: const Default,
    {
        assert!(N.is_power_of_two(), "the size need to be a power of two");
        const CELL_INIT<K: const Default>: UnsafeCell<K> = UnsafeCell::new(K::default());
        Self {
            data: [CELL_INIT; N],
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
        }
    }

    pub fn len(&self) -> usize {
        self.tail().wrapping_sub(self.head())
    }

    pub fn head(&self) -> usize {
        self.head.load(Ordering::SeqCst)
    }

    pub fn tail(&self) -> usize {
        self.tail.load(Ordering::SeqCst)
    }

    pub fn is_empty(&self) -> bool {
        self.head() == self.tail()
    }

    pub fn is_full(&self) -> bool {
        ((self.head() + 1) & (N - 1)) == self.tail()
    }

    pub fn push(&self, data: T) -> Result<(), &'static str> {
        let head = self.head();
        let tail = self.tail();

        let next = (head + 1) & (N - 1);
        if next == tail {
            return Err("The ring buffer is full");
        }

        unsafe {
            let ptr = self.data[head].get();
            *ptr = data
        }

        self.head.store(next, Ordering::Release);
        Ok(())
    }

    pub fn pop(&self) -> Option<T>
    where
        T: Clone,
    {
        let tail = self.tail.load(Ordering::Relaxed);
        let head = self.head.load(Ordering::Acquire);

        if head == tail {
            return None;
        }

        let elt = unsafe {
            let ptr = self.data[tail].get();
            (*ptr).clone()
        };

        self.tail.store((tail + 1) & (N - 1), Ordering::Release);

        Some(elt)
    }

    pub fn pop_n(&self, n: usize) -> Vec<T>
    where
        T: Clone,
    {
        let len = self.len().min(n);

        let mut v = Vec::new();

        for _ in 0..len {
            let p = self.pop();

            match p {
                Some(p) => v.push(p),
                None => {
                    return v;
                }
            }
        }

        v
    }
}

unsafe impl<T, const N: usize> Send for AtomicRingBuffer<T, N> {}
unsafe impl<T, const N: usize> Sync for AtomicRingBuffer<T, N> {}

pub struct AtomicCircularRingBuffer<const N: usize> {
    data: [UnsafeCell<u8>; N],
    head: AtomicUsize,
    tail: AtomicUsize,
}

unsafe impl<const N: usize> Send for AtomicCircularRingBuffer<N> {}
unsafe impl<const N: usize> Sync for AtomicCircularRingBuffer<N> {}

impl<const N: usize> AtomicCircularRingBuffer<N> {
    pub const fn new() -> Self {
        assert!(N.is_power_of_two(), "N need to be a power of 2");
        const CELL_INIT: UnsafeCell<u8> = UnsafeCell::new(0);

        Self {
            data: [CELL_INIT; N],
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
        }
    }

    pub fn is_empty(&self) -> bool {
        let head = self.head.load(Ordering::SeqCst);
        let tail = self.tail.load(Ordering::SeqCst);

        head == tail
    }

    pub fn push(&self, byte: u8) {
        let head = self.head.load(Ordering::SeqCst);
        let tail = self.tail.load(Ordering::SeqCst);

        unsafe {
            let cell_ptr = self.data[head].get();
            *cell_ptr = byte
        }

        let next_head = (head + 1) & (N - 1);

        if next_head == tail {
            let next_tail = (tail + 1) & (N - 1);
            let _ =
                self.tail
                    .compare_exchange(tail, next_tail, Ordering::Release, Ordering::Relaxed);
        }

        self.head.store(next_head, Ordering::Release);
    }

    pub fn pop(&self) -> Option<u8> {
        let mut tail = self.tail.load(Ordering::Relaxed);
        loop {
            let head = self.head.load(Ordering::Relaxed);
            if head == tail {
                return None;
            }

            let byte = unsafe { *self.data[tail].get() };

            let next_tail = (tail + 1) & (N - 1);

            match self
                .tail
                .compare_exchange(tail, next_tail, Ordering::Release, Ordering::Acquire)
            {
                Ok(_) => return Some(byte),
                Err(actual_tail) => {
                    tail = actual_tail;
                }
            }
        }
    }

    pub fn pop_last(&self) {
        let head = self.head.load(Ordering::SeqCst);
        let tail = self.tail.load(Ordering::SeqCst);

        if head != tail {
            let prev_head = (head.wrapping_sub(1)) & (N - 1);
            self.head.store(prev_head, Ordering::SeqCst);
        }
    }

    pub fn len(&self) -> usize {
        let head = self.head.load(Ordering::Relaxed);
        let tail = self.tail.load(Ordering::Relaxed);

        (head.wrapping_sub(tail)) & (N - 1)
    }

    pub fn pop_n(&self, n: usize) -> Vec<u8> {
        let n = n.min(self.len());

        let mut vec = Vec::new();

        for _ in 0..n {
            let res = self.pop();
            match res {
                Some(v) => vec.push(v),
                None => {
                    return vec;
                }
            }
        }

        vec
    }
}

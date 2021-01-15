//! Single value multiple producer multiple consumer update channel.
//! Both the receiver and channel can be created with the update_channel
//! and update_channel_with functions.
//!
//! The channels are mostly lock free, multiple readers can access the shared value at the same time.
//! The only way to really block is by calling the [`borrow_locked`]((struct.Receiver.html#method.borrow_locked))
//! method and holding onto the RwLockReadGuard
//! or by having a lot of writes happen at the same time. <br /> <br />
//! The update channel can practically be seen as a single value which can be updated by an updater
//! and then a receiver can update its own internal value with the
//! [`receive_update`]((struct.Receiver.html#method.receive_update)) method by cloning the new value in its internal buffer.
//! If only a single receiver is used the take_update method is a more efficient alternative to the receive_update method
//!
//! # Example
//!
//! ```
//! use update_channel::channel_with;
//! use std::thread::spawn;
//! 
//! let (mut receiver, updater) = channel_with(0);
//! assert_eq!(*receiver.borrow(), 0);
//! 
//! spawn(move || {
//!     updater.update(2).unwrap(); // shared value is 2
//!     updater.update(12).unwrap(); // shared value is 12
//! })
//! .join().unwrap();
//! 
//! // Shared value is 2 but internal value is 0
//! assert_eq!(*receiver.borrow(), 0);
//! // Update the latest value
//! receiver.recv_update().unwrap();
//! // Shared value is 12 and internal value 12
//! assert_eq!(*receiver.borrow(), 12);
//! ```
//!

mod receiver;
mod updater;

pub use receiver::*;
pub use updater::*;

use std::{
    cell::UnsafeCell,
    sync::{Arc, RwLock},
};

/// Create a channel where the receiver starts with value as internal value
pub fn channel_with<T>(value: T) -> (Receiver<T>, Updater<T>) {
    let shared = Arc::new(RwLock::new(None));
    let weak = Arc::downgrade(&shared);
    let rec = Receiver {
        cell: UnsafeCell::new(value),
        shared,
    };

    let upd = Updater { lock: weak };

    (rec, upd)
}

/// Creates a channel with None as start value
pub fn channel<T>() -> (Receiver<Option<T>>, Updater<Option<T>>) {
    let shared = Arc::new(RwLock::new(None));
    let weak = Arc::downgrade(&shared);
    let rec = Receiver {
        cell: UnsafeCell::new(None),
        shared,
    };

    let upd = Updater { lock: weak };

    (rec, upd)
}

/// Starts the channel with the default value of T
pub fn channel_default<T>() -> (Receiver<T>, Updater<T>)
where
    T: Default,
{
    let shared = Arc::new(RwLock::new(None));
    let weak = Arc::downgrade(&shared);
    let rec = Receiver {
        cell: UnsafeCell::new(Default::default()),
        shared,
    };

    let upd = Updater { lock: weak };

    (rec, upd)
}

#[cfg(test)]
mod test {
    use super::*;
    use std::{
        sync::{Arc, Barrier},
        thread::spawn,
    };

    #[test]
    fn creation_default() {
        let (rec, upd) = channel_default::<i32>();
        assert!(upd.has_receiver());
        assert!(rec.has_updater());
        assert_eq!(rec.borrow().clone(), 0);
    }

    #[test]
    fn take() {
        let (mut rec, upd) = channel_default::<i32>();
        assert!(upd.has_receiver());
        assert!(rec.has_updater());
        assert_eq!(rec.borrow().clone(), 0);
        rec.take_update().unwrap();
        assert_eq!(rec.borrow().clone(), 0);
        upd.update(10).unwrap();
        rec.take_update().unwrap();
        assert_eq!(rec.borrow().clone(), 10);
    }

    #[test]
    fn recv() {
        let (mut rec, upd) = channel_default::<i32>();
        assert!(upd.has_receiver());
        assert!(rec.has_updater());
        assert_eq!(rec.borrow().clone(), 0);
        rec.recv_update().unwrap();
        assert_eq!(rec.borrow().clone(), 0);
        upd.update(10).unwrap();
        rec.recv_update().unwrap();
        assert_eq!(rec.borrow().clone(), 10);
    }

    #[test]
    fn no_upd() {
        let (rec, upd) = channel_with(100);
        assert!(rec.has_updater());
        assert_eq!(rec.borrow(), &100);
        std::mem::drop(upd);
        assert_eq!(rec.borrow(), &100);
        assert!(!rec.has_updater());
    }

    #[test]
    fn no_rec() {
        let (rec, upd) = channel_with(100);
        assert!(upd.has_receiver());
        assert_eq!(rec.borrow(), &100);
        let val = rec.into_inner();
        assert_eq!(val, 100);
        assert!(!upd.has_receiver());
    }

    fn barrier_pair(n: usize) -> (Arc<Barrier>, Arc<Barrier>) {
        let barrier = Barrier::new(n);
        let b = Arc::new(barrier);
        (Arc::clone(&b), b)
    }

    #[test]
    fn clone_recv() {
        let (mut rec, upd) = channel_with(1);
        let mut rec2 = rec.clone();
        assert_eq!(rec.borrow(), &1);
        assert_eq!(rec2.borrow(), rec.borrow());

        *rec.borrow_mut() = 100;

        assert_eq!(rec.borrow(), &100);
        assert_eq!(rec2.borrow(), &1);

        upd.update(2).unwrap();
        assert_eq!(rec.borrow(), &100);
        assert_eq!(rec2.borrow(), &1);

        rec.recv_update_checked().unwrap();
        rec2.take_update().unwrap();

        assert_eq!(rec.borrow(), &2);
        assert_eq!(rec2.borrow(), &2);
    }

    #[test]
    fn two_threads() {
        let (mut rec, upd) = channel_with(1);
        let (b1, b2) = barrier_pair(2);

        let th = spawn(move || {
            b1.wait();
            upd.update(2).unwrap();
            b1.wait();
        });

        assert_eq!(rec.borrow(), &1);
        b2.wait();

        rec.recv_update().unwrap();

        assert_eq!(rec.borrow(), &2);
        b2.wait();

        th.join().unwrap();
    }

    #[test]
    fn three_threads() {
        let (mut rec, upd) = channel_with(1);
        let (b1, b2) = barrier_pair(3);
        let b3 = Arc::clone(&b1);

        let mut rec2 = rec.clone();

        let th = spawn(move || {
            upd.update(2).unwrap();
            b1.wait();
            b1.wait();
        });

        let th2 = spawn(move || {
            assert_eq!(rec2.borrow(), &1);
            b3.wait();
            rec2.take_update().unwrap();
            assert_eq!(rec2.borrow(), &2);
            b3.wait();
        });

        b2.wait();
        assert_eq!(rec.borrow(), &1);
        b2.wait();
        rec.take_update().unwrap();
        assert_eq!(rec.borrow(), &1);

        th.join().unwrap();
        th2.join().unwrap();
    }

    #[test]
    fn simple() {
        let (mut receiver, updater) = channel_with(0);
        assert_eq!(*receiver.borrow(), 0);

        spawn(move || {
            updater.update(2).unwrap(); // shared value is 2
            updater.update(12).unwrap(); // shared value is 12
        })
        .join().unwrap();

        // Shared value is 2 but internal value is 0
        assert_eq!(*receiver.borrow(), 0);
        // Update the latest value
        receiver.recv_update().unwrap();
        // Shared value is 12 and internal value 12
        assert_eq!(*receiver.borrow(), 12);
    }
}

use std::{
    cell::UnsafeCell,
    sync::{Arc, RwLock, RwLockReadGuard},
};

#[derive(Debug)]
pub struct Receiver<T> {
    pub(crate) cell: UnsafeCell<T>,
    pub(crate) shared: Arc<RwLock<Option<T>>>,
}

impl<T> Receiver<T> {
    /// Takes the new updated value and make it unavailable for all other receivers.
    /// Returns the previous value inside the buffer if the value has not been taken by another receiver,
    /// otherwise Returns Ok(None)
    pub fn take_update(&mut self) -> Result<Option<T>, ReceiveError> {
        unsafe {
            self.take_update_unsafe()
        }
    }

    /// Same as [`Receiver::take_update`](struct.Receiver.html#method.take_update) but without borrowing mutably.
    /// This might open new possiblities, but it might result in undefined behavior if there are immutable borrows
    /// created with [`Receiver::borrow`](struct.Receiver.html#method.borrow)
    pub unsafe fn take_update_unsafe(&self) -> Result<Option<T>, ReceiveError> {
        let mut lock = self.shared.write().map_err(|_| ReceiveError)?;

        if let Some(s) = lock.take() {
            let old = std::mem::replace(&mut *self.cell.get(), s);
            Ok(Some(old))
        } else {
            Ok(None)
        }
    }

    /// Get the value saved in the internal buffer without updating as borrow
    pub fn borrow(&self) -> &T {
        unsafe { &*self.cell.get() }
    }

    /// Get a mutable reference to the value in the buffer
    pub fn borrow_mut(&mut self) -> &mut T {
        unsafe { &mut *self.cell.get() }
    }

    /// Unwrap the value contained in the buffer of this receiver
    pub fn into_inner(self) -> T {
        self.cell.into_inner()
    }

    /// Get the latest value (not the value in the buffer) and return it while holding a read lock to it.
    /// It is not recommended to hold on to this lock for long.
    pub fn borrow_locked<'a>(&'a self) -> Result<RwLockReadGuard<'a, Option<T>>, ReceiveError> {
        self.shared.read().map_err(|_| ReceiveError)
    }

    /// Checks if at least one updater exists
    pub fn has_updater(&self) -> bool {
        Arc::weak_count(&self.shared) != 0
    }
}

impl<T> Receiver<T>
where
    T: Clone,
{
    ///Clone the latest updated value into the internal buffer.
    /// Returns the previous value inside the buffer if the value has not been taken by another receiver
    pub fn recv_update(&mut self) -> Result<Option<T>, ReceiveError> {
        unsafe {
            self.recv_update_unsafe()
        }
    }

    /// Same as [`Receiver::recv_update`](struct.Receiver.html#method.recv_update) but without borrowing mutably.
    /// This might open new possiblities, but it might result in undefined behavior if there are immutable borrows
    /// created with [`Receiver::borrow`](struct.Receiver.html#method.borrow)
    pub unsafe fn recv_update_unsafe(&self) -> Result<Option<T>, ReceiveError> {
        let lock = self.shared.read().map_err(|_| ReceiveError)?;
        if let Some(s) = &*lock {
            let old = std::mem::replace(&mut *self.cell.get(), s.clone());
            Ok(Some(old))
        } else {
            Ok(None)
        }
    }

    /// Get the latest value (not the value in the buffer) cloned
    pub fn get_cloned(&self) -> Result<Option<T>, ReceiveError> {
        self.shared
            .read()
            .map_err(|_| ReceiveError)
            .map(|v| v.clone())
    }
}

impl<T> Receiver<T>
where
    T: Clone + PartialEq,
{
    /// Updates the internal buffer by cloning the updated value if it is different from the current held value
    pub fn recv_update_checked(&mut self) -> Result<Option<T>, ReceiveError> {
        unsafe { self.recv_update_checked_unsafe() }
    }

    /// Same as [`Receiver::recv_update_checked`](struct.Receiver.html#method.recv_update_checked) but without borrowing mutably.
    /// This might open new possiblities, but it might result in undefined behavior if there are immutable borrows
    /// created with [`Receiver::borrow`](struct.Receiver.html#method.borrow)
    pub unsafe fn recv_update_checked_unsafe(&self) -> Result<Option<T>, ReceiveError> {
        let lock = self.shared.read().map_err(|_| ReceiveError)?;

        if let Some(value) = &*lock {
            let vptr = self.cell.get();
            if value != &*vptr {
                let old = std::mem::replace(&mut *vptr, value.clone() );
                Ok(Some(old))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    /// Updates the internal buffer by taking the updated value if it is different from the current held value
    pub fn take_update_checked(&mut self) -> Result<Option<T>, ReceiveError> {
        unsafe { self.take_update_checked_unsafe() }
    }

    /// Same as [`Receiver::recv_update_checked`](struct.Receiver.html#method.recv_update_checked) but without borrowing mutably.
    /// This might open new possiblities, but it might result in undefined behavior if there are immutable borrows
    /// created with [`Receiver::borrow`](struct.Receiver.html#method.borrow)
    pub unsafe fn take_update_checked_unsafe(&self) -> Result<Option<T>, ReceiveError> {
        let mut lock = self.shared.write().map_err(|_| ReceiveError)?;

        if let Some(value) = lock.take() {
            let vptr = self.cell.get();
            if value != *vptr {
                let old = std::mem::replace(&mut *vptr, value.clone() );
                Ok(Some(old))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }
}

impl<T> Clone for Receiver<T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        Self {
            shared: Arc::clone(&self.shared),
            cell: UnsafeCell::new(unsafe { (&*self.cell.get()).clone() }),
        }
    }
}

unsafe impl<T> Send for Receiver<T> where T: Clone {}

/// An error that might occur while receiving a value
#[derive(Debug, Clone)]
pub struct ReceiveError;

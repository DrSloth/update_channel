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
    /// Takes the new updated value make it unavailable for all other receivers
    pub fn take_update(&mut self) -> Result<Option<T>, ReceiveError> {
        let mut lock = self.shared.write().map_err(|_| ReceiveError)?;

        unsafe {
            if let Some(s) = lock.take() {
                let old = std::mem::replace(&mut *self.cell.get(), s);
                Ok(Some(old))
            } else {
                Ok(None)
            }
        }
    }

    /// Same as [`Receiver::take_update`](struct.Receiver.html#method.take_update) but without borrowing mutably.
    /// This might open new possiblities, but it might result in undefined behavior if there are immutable borrows
    /// created with
    pub unsafe fn take_update_unsafe(&self) -> Result<(), ReceiveError> {
        let mut lock = self.shared.write().map_err(|_| ReceiveError)?;
        if let Some(s) = lock.take() {
            *self.cell.get() = s;
        }

        Ok(())
    }

    /// Get the value saved in the internal buffer without updating
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

    pub fn borrow_locked<'a>(&'a self) -> Result<RwLockReadGuard<'a, Option<T>>, ReceiveError> {
        self.shared.read().map_err(|_| ReceiveError)
    }

    pub fn has_updater(&self) -> bool {
        Arc::weak_count(&self.shared) != 0
    }
}

impl<T> Receiver<T>
where
    T: Clone,
{
    pub fn recv_update(&mut self) -> Result<(), ReceiveError> {
        let lock = self.shared.read().map_err(|_| ReceiveError)?;

        unsafe {
            if lock.is_some() {
                *self.cell.get() = lock.clone().unwrap();
            }
        }

        Ok(())
    }

    pub unsafe fn recv_update_unsafe(&self) -> Result<(), ReceiveError> {
        let lock = self.shared.read().map_err(|_| ReceiveError)?;
        if lock.is_some() {
            *self.cell.get() = lock.clone().unwrap();
        }

        Ok(())
    }

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
    pub fn recv_update_checked(&mut self) -> Result<(), ReceiveError> {
        let lock = self.shared.read().map_err(|_| ReceiveError)?;

        unsafe {
            if let Some(value) = &*lock {
                let vptr = self.cell.get();
                if value != &*vptr {
                    *vptr = value.clone();
                }
            }
        }

        Ok(())
    }

    pub unsafe fn recv_update_checked_unsafe(&self) -> Result<(), ReceiveError> {
        let lock = self.shared.read().map_err(|_| ReceiveError)?;

        if let Some(value) = &*lock {
            let vptr = self.cell.get();
            if value != &*vptr {
                *vptr = value.clone();
            }
        }

        Ok(())
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

unsafe impl<T> Send for Receiver<T> {}
unsafe impl<T> Sync for Receiver<T> {}

#[derive(Debug, Clone)]
pub struct ReceiveError;


use std::sync::{RwLock, Weak};

#[derive(Debug, Clone)]
pub struct Updater<T> {
    pub(crate) lock: Weak<RwLock<Option<T>>>
}

impl<T> Updater<T> {
    /// Checks if at least one receiver exists
    pub fn has_receiver(&self) -> bool {
        self.lock.upgrade().is_some()
    }

    /// Updates the value currently saved in the channel. A receiver can then call update in order
    /// to check the latest value and write it into its buffer
    pub fn update(&self, value: T) -> Result<(), UpdateError<T>> {
        if let Some(shared) = self.lock.upgrade() {
            if let Ok(mut write) = shared.write() {
                *write = Some(value);
                Ok(())
            } else {
                Err(UpdateError::Poisoned(value))
            }
        } else {
            Err(UpdateError::NoReceiver(value))
        }
    }
}


pub enum UpdateError<T> {
    NoReceiver(T),
    Poisoned(T)
}

impl<T> UpdateError<T> {
    pub fn into_inner(self) -> T {
        match self {
            UpdateError::NoReceiver(v) => v,
            UpdateError::Poisoned(v) => v,
        }
    }

    pub fn inner(&self) -> &T {
        match self {
            UpdateError::NoReceiver(v) => &v,
            UpdateError::Poisoned(v) => &v,
        }
    }
}

unsafe impl<T> Send for Updater<T> {}
unsafe impl<T> Sync for Updater<T> {}

use std::fmt::{Formatter, Debug, Result as FmtResult};

impl<T> Debug for UpdateError<T> where T: Debug {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(f, "UpdateError({:?})", self.inner())
    }
}

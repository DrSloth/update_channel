use std::sync::{RwLock, Weak};

/// The updater half of the update channel.
/// 
/// You can update the shared value with [`Updater::update`](struct.Updater.html#method.update). <br />
/// This doesn't mean that receivers directly hold the new value 
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
    /// to check the latest value and write it into its buffer. If either the RwLock is poisoned, or
    /// no receiver exists an UpdateError holding the value will be returned.
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

/// An error occurred while updating the update channel
pub enum UpdateError<T> {
    /// There is no receiver
    NoReceiver(T),
    /// The RwLock is poisoned
    Poisoned(T)
}

impl<T> UpdateError<T> {
    /// Get contained value the value contained 
    pub fn into_inner(self) -> T {
        match self {
            UpdateError::NoReceiver(v) => v,
            UpdateError::Poisoned(v) => v,
        }
    }

    /// Get a reference to the contained value.
    pub fn inner(&self) -> &T {
        match self {
            UpdateError::NoReceiver(v) => &v,
            UpdateError::Poisoned(v) => &v,
        }
    }
}

unsafe impl<T> Send for Updater<T> {}

use std::fmt::{Formatter, Debug, Result as FmtResult};

impl<T> Debug for UpdateError<T> where T: Debug {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(f, "UpdateError({:?})", self.inner())
    }
}

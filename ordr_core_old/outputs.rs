use core::fmt;
use std::{any::TypeId, collections::HashMap};

use crate::node::Payload;

/// All results from running a job.
pub struct Outputs {
    /// Hashmap of all results.
    outputs: HashMap<TypeId, Payload>,
}

impl Outputs {
    /// Create a new [`Ouputs`].
    pub(crate) fn new(outputs: HashMap<TypeId, Payload>) -> Self {
        Self { outputs }
    }

    /// Get the result for a specific Node.
    ///
    /// Notice that a result may not be there even if the job is fully done, since it may not have
    /// been needed to run the producer for the node to reach the target.
    ///
    /// # Panics
    /// If we somehow got the wrong payload mapped to a type. This should not be possible.
    #[must_use]
    pub fn get<T: 'static>(&self) -> Option<&T> {
        let id = TypeId::of::<T>();
        self.outputs
            .get(&id)
            .map(|payload| payload.downcast_ref().unwrap())
    }

    /// Takes the result for a specific Node out of the struct.
    ///
    /// Notice that a result may not be there even if the job is fully done, since it may not have
    /// been needed to run the producer for the node to reach the target.
    ///
    /// # Panics
    /// If we somehow got the wrong payload mapped to a type. This should not be possible.
    pub fn take<T: 'static>(&mut self) -> Option<T> {
        let id = TypeId::of::<T>();
        self.outputs
            .remove(&id)
            .map(|payload| *payload.downcast().unwrap())
    }
}

impl fmt::Debug for Outputs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Outputs[{}]", self.outputs.len())
    }
}

use std::cell::{Ref, RefCell, RefMut};

use automerge::sync::State;
use magnus::{Error, RString, Ruby};

use crate::errors::error;

/// What a document knows about one peer while syncing with it.
#[magnus::wrap(class = "Mechanomeld::SyncState", free_immediately, size)]
pub struct SyncState {
    inner: RefCell<State>,
}

impl SyncState {
    fn state(&self, ruby: &Ruby) -> Result<Ref<'_, State>, Error> {
        self.inner
            .try_borrow()
            .map_err(|_| error(ruby, "sync state is being modified"))
    }

    pub fn state_mut(&self, ruby: &Ruby) -> Result<RefMut<'_, State>, Error> {
        self.inner
            .try_borrow_mut()
            .map_err(|_| error(ruby, "sync state is already in use"))
    }

    /// `SyncState.new`: a fresh state for a peer this document has not synced with.
    pub fn new() -> Self {
        Self {
            inner: RefCell::new(State::new()),
        }
    }

    /// `SyncState.decode(bytes)`
    pub fn decode(ruby: &Ruby, bytes: RString) -> Result<Self, Error> {
        let data = unsafe { bytes.as_slice() }.to_vec();
        let state = State::decode(&data)
            .map_err(|e| error(ruby, format!("could not decode sync state: {e}")))?;
        Ok(Self {
            inner: RefCell::new(state),
        })
    }

    /// `sync_state.encode`: the state as a binary String, keeping only what is worth
    /// reusing on a later connection with the same peer.
    pub fn encode(ruby: &Ruby, rb_self: &Self) -> Result<RString, Error> {
        Ok(ruby.str_from_slice(&rb_self.state(ruby)?.encode()))
    }
}

use arc_swap::ArcSwap;
use std::sync::Arc;
use streamdeck_lib::prelude::*;

use streamdeck_sc_core::bindings::action_binding::ActionBinding;
use streamdeck_sc_core::bindings::action_bindings::ActionBindings;

pub struct ActionBindingsStore {
    inner: Arc<ArcSwap<ActionBindings>>,
    logger: Arc<dyn ActionLog>,
}

impl Clone for ActionBindingsStore {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
            logger: Arc::clone(&self.logger),
        }
    }
}

impl ActionBindingsStore {
    pub fn new(logger: Arc<dyn ActionLog>) -> Self {
        Self {
            inner: Arc::new(ArcSwap::from_pointee(ActionBindings::default())),
            logger,
        }
    }

    pub fn snapshot(&self) -> Arc<ActionBindings> {
        self.inner.load_full()
    }

    pub fn replace(&self, new_ab: ActionBindings) {
        self.inner.store(Arc::new(new_ab));
    }

    pub fn clear(&self) {
        self.inner.store(Arc::new(ActionBindings::default()));
    }

    pub fn get_binding_by_id(&self, id: &str) -> Option<ActionBinding> {
        let (map, action) = {
            let mut parts = id.splitn(2, '.');
            (parts.next()?, parts.next()?)
        };
        let snap = self.snapshot();
        snap.action_maps
            .get(map)
            .and_then(|m| m.actions.get(action))
            .cloned()
    }
}

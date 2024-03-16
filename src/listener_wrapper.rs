use crate::ulistener::ClonableBoxUListener;
use crate::ulistener::UListener;
use crate::{UMessage, UStatus};
use std::any::{Any, TypeId};
use std::hash::{Hash, Hasher};

/// A wrapper type around UListener that can be used by `up-client-foo-rust` UPClient libraries
/// to ease some common development scenarios
///
/// # Note
///
/// Not necessary for end-user uEs to use. Primarily intended for `up-client-foo-rust` UPClient libraries
///
/// # Rationale
///
/// The wrapper type is implemented such that it can be used in any location you may wish to
/// hold a generic UListener
///
/// Also implements necessary traits to allow hashing, so that you may hold the wrapper type in
/// collections which require that, such as a `HashMap` or `HashSet`
pub struct ListenerWrapper {
    listener: Box<dyn ClonableBoxUListener + 'static>,
    type_id: TypeId,
}

impl Clone for ListenerWrapper {
    fn clone(&self) -> Self {
        let cloned_listener = self.listener.clone_box();

        // Construct a new ListenerWrapper with the cloned listener and the same type_id
        ListenerWrapper {
            listener: cloned_listener,
            type_id: self.type_id,
        }
    }
}

impl ListenerWrapper {
    pub fn new<T>(listener: T) -> Self
    where
        T: ClonableBoxUListener + UListener + 'static,
    {
        let any_listener = Box::new(listener) as Box<dyn ClonableBoxUListener + 'static>;
        Self {
            listener: any_listener,
            type_id: TypeId::of::<T>(),
        }
    }

    pub fn on_receive(&self, received: Result<UMessage, UStatus>) {
        self.listener.on_receive(received)
    }
}

pub trait UListenerTypeTag {
    fn as_any(&self) -> &dyn Any;
}

impl UListenerTypeTag for ListenerWrapper {
    fn as_any(&self) -> &dyn Any {
        &self.listener
    }
}

impl PartialEq for ListenerWrapper {
    fn eq(&self, other: &Self) -> bool {
        self.type_id == other.type_id
    }
}

impl Eq for ListenerWrapper {}

impl Hash for ListenerWrapper {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.type_id.hash(state);
    }
}

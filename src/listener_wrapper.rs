use crate::ulistener::UListener;
use std::any::{Any, TypeId};
use std::hash::{Hash, Hasher};
use std::ops::Deref;

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
    listener: Box<dyn UListener + 'static>,
    type_id: TypeId,
}

impl ListenerWrapper {
    pub fn new<T>(listener: T) -> Self
    where
        T: UListener + 'static,
    {
        let any_listener = Box::new(listener) as Box<dyn UListener + 'static>;
        Self {
            listener: any_listener,
            type_id: TypeId::of::<T>(),
        }
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

impl Deref for ListenerWrapper {
    type Target = Box<dyn UListener + 'static>;

    fn deref(&self) -> &Self::Target {
        &self.listener
    }
}

use std::any::Any;
use std::fmt::Debug;
use std::hash::{Hash, Hasher};
use crate::{UMessage, UStatus};

pub trait UListener: Debug + Any + Send + Sync {
    fn as_any(&self) -> &dyn Any;

    fn on_receive(&self, received: Result<UMessage, UStatus>);
}

impl Hash for dyn UListener {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_any().type_id().hash(state);
    }
}

impl PartialEq for dyn UListener {
    fn eq(&self, other: &Self) -> bool {
        Any::type_id(self) == Any::type_id(other)
    }
}

impl Eq for dyn UListener {}
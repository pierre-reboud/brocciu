use std::cell::RefCell;
use std::hash::{BuildHasher, Hash, Hasher};
use std::rc::Rc;
use crate::mcts::tree::Node;
use std::ops::Deref;

pub struct HashableRcRefCell<T>(Rc<RefCell<T>>);

pub type NodeRef = HashableRcRefCell<Node>;

impl<T: Hash> HashableRcRefCell<T>{
    pub fn new(t: T) -> Self {
        HashableRcRefCell(Rc::new(RefCell::new(t)))
    }
}

impl<T> Clone for HashableRcRefCell<T> {
    fn clone(&self) -> Self {
        HashableRcRefCell(Rc::clone(&self.0))
    }
}

impl<T: PartialEq> PartialEq for HashableRcRefCell<T> {
    fn eq(&self, other: &Self) -> bool {
        *self.0.borrow() == *other.0.borrow()
    }
}

impl<T: Eq> Eq for HashableRcRefCell<T> {}

impl<T: Hash> Hash for HashableRcRefCell<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (*self.0.borrow()).hash(state);
    }
}

impl<T> Deref for HashableRcRefCell<T>{
    type Target = RefCell<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
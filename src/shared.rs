/*!
 * A simple abstraction over the Rc<RefCell<T>> pattern.
 * Shamelessly copied from https://gist.github.com/stevedonovan/7e3a6d8c8921e3eff16c4b11ab82b8d7.
 */

use std::rc::Rc;
use std::cell::{RefCell, Ref, RefMut};
use std::ops::Deref;
use std::fmt;


/**
 * Represents a shared pointer to an object
 * on the heap, with interior mutability.
 */

#[derive(Clone)]
pub struct Shared<T> {
	v: Rc<RefCell<T>>
}

impl <T> Shared<T> {

	/**
	 * Creates a new Shared with the contents provided.
	 */

	pub fn new(t: T) -> Shared<T> {
		Shared { v: Rc::new(RefCell::new(t)) }
	}
}

impl <T> Shared<T> {

	/**
	 * Borrows an immutable reference to the stored object.
	 */

	pub fn borrow(&self) -> Ref<T> {
		self.v.borrow()
	}


	/**
	 * Borrows a mutable reference to the stored object.
	 */

	pub fn borrow_mut(&self) -> RefMut<T> {
		self.v.borrow_mut()
	}


	/**
	 * Borrows a mutable pointer to the stored object.
	 */

	pub fn as_ptr(&self) -> *mut T {
		self.v.as_ptr()
	}


	/**
	 * Replaces the stored object with a new one.
	 */

	pub fn replace(&self, t: T) -> T {
		self.v.replace(t)
	}


	/**
	 * Creates a new pointer to the stored memory.
	 * This operation is inexpensive, and does not clone the underlying object.
	 */

	pub fn clone(&self) -> Shared<T> {
		Shared { v: Rc::clone(&self.v) }
	}
}


impl <T: fmt::Display> fmt::Display for Shared<T> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "{}", self.deref())
	}
}


impl <T: fmt::Debug> fmt::Debug for Shared<T> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "{:?}", self.deref())
	}
}

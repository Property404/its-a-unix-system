use std::{
    collections::VecDeque,
    iter::{Extend, Iterator},
};

/// An iterator that can be extended.
pub struct ExtendableIterator<T: Sized>(VecDeque<T>);

impl<T: Sized> ExtendableIterator<T> {
    /// Construct a new ExtendableIterator.
    pub fn new<U: DoubleEndedIterator<Item = T>>(it: U) -> Self {
        let vec = it.collect();
        Self(vec)
    }

    /// Prepend to iterator.
    pub fn prepend<U>(&mut self, mut iter: U)
    where
        U: DoubleEndedIterator<Item = T>,
    {
        while let Some(item) = iter.next_back() {
            self.0.push_front(item)
        }
    }

    /// Check if iterator is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl<T: Sized> Iterator for ExtendableIterator<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.pop_front()
    }
}
impl<T: Sized> Extend<T> for ExtendableIterator<T> {
    fn extend<U>(&mut self, iter: U)
    where
        U: IntoIterator<Item = T>,
    {
        self.0.extend(iter)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prepend_iterator() {
        let text = "Hi";
        let mut it = ExtendableIterator::new(text.chars());
        it.prepend("Ho".chars());
        assert_eq!(it.next().unwrap(), 'H');
        assert_eq!(it.next().unwrap(), 'o');
        assert_eq!(it.next().unwrap(), 'H');
        assert_eq!(it.next().unwrap(), 'i');
        assert_eq!(it.next(), None);
    }

    #[test]
    fn extend_iterator() {
        let text = "Hi";
        let mut it = ExtendableIterator::new(text.chars());
        it.extend("Ho".chars());
        assert_eq!(it.next().unwrap(), 'H');
        assert_eq!(it.next().unwrap(), 'i');
        assert_eq!(it.next().unwrap(), 'H');
        assert_eq!(it.next().unwrap(), 'o');
        assert_eq!(it.next(), None);
    }
}

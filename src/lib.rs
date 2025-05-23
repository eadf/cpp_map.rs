//           Copyright 2021 Eadf (github.com/eadf)
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! A simple data structure emulating a C++ std::map. Probably not useful for anyone.
//!
//! I needed a data structure that could emulate a C++ std::map, and it's pointer based iterators.
//!
//! More specifically it needs to emulate the insertion position hint functionality as the keys I intend to
//! use are not entirely transitive. i.e., searching for insertion position from the head or tail makes a big difference.
//!
//! I also needed to be able to replace the key of an already inserted item w/o altering the order. - Don't ask.
//! Another gotcha is that insert(key, value) is a NOP if the key already exists, not even the new value will be used.
//!
//! The current implementation uses a double linked std::vec::Vec list, and it only supports sequential search.
//!
use std::cell::RefCell;
use std::cmp::Ordering;
use std::fmt::Debug;
use std::rc::Rc;

/// Indicates that an iterator has passed beyond the limits of the list.
pub const OUT_OF_BOUNDS: usize = usize::MAX;

#[derive(thiserror::Error, Debug)]
pub enum MapError {
    #[error("error: Some error with the linked list")]
    InternalError(String),
    #[error(transparent)]
    BorrowError(#[from] std::cell::BorrowError),
    #[error(transparent)]
    BorrowMutError(#[from] std::cell::BorrowMutError),
}

#[cfg(test)]
mod test {}

#[derive(Clone, Debug)]
struct Node<K, V>
where
    K: Debug,
    V: Debug,
{
    prev_: usize,
    next_: usize,
    key_: K,
    value_: V,
}

/// A double linked min list.
/// The head (top/front) of the list is the first item. Sorted Order::Less than other items.
/// The tail (bottom/back) is the last item of the list. Sorted Order::Greater than other items.
#[derive(Clone, Debug)]
pub struct LinkedList<K, V>
where
    K: Debug,
    V: Debug,
{
    head_: usize,
    tail_: usize,
    nodes_: Vec<Option<Node<K, V>>>,
    id_pool_: Vec<usize>,
}

impl<K, V> Default for LinkedList<K, V>
where
    K: Debug,
    V: Debug,
{
    fn default() -> Self {
        Self {
            head_: OUT_OF_BOUNDS,
            tail_: OUT_OF_BOUNDS,
            nodes_: Vec::default(),
            id_pool_: Vec::default(),
        }
    }
}

#[derive(Clone, Debug)]
/// borrow checker work-around
struct EraseOperation {
    // (node index, next index)
    change_prev_: Option<(usize, usize)>,
    // the node to erase
    erase_: usize,
    // (node index, pre index)
    change_next_: Option<(usize, usize)>,
}

#[allow(dead_code)]
impl<K, V> LinkedList<K, V>
where
    K: Debug + Ord + PartialOrd,
    V: Debug,
{
    /// Constructs a new, empty LinkedList<K,V> with the specified capacity.
    /// The LinkedList will be able to hold exactly capacity elements without reallocating.
    /// If capacity is 0, the list will not allocate.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            head_: OUT_OF_BOUNDS,
            tail_: OUT_OF_BOUNDS,
            nodes_: Vec::with_capacity(capacity),
            id_pool_: Vec::with_capacity(capacity),
        }
    }

    pub fn iter(&self) -> ListIterator<'_, K, V> {
        ListIterator {
            list_: self,
            my_next_: self.head_,
        }
    }

    #[inline(always)]
    /// Returns the number of inserted elements
    pub fn len(&self) -> usize {
        self.nodes_.len() - self.id_pool_.len()
    }

    /// Returns the capacity or the vectors
    pub fn capacity(&self) -> (usize, usize) {
        (self.nodes_.capacity(), self.id_pool_.capacity())
    }

    #[inline(always)]
    /// Returns true if the list is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Clears the list.
    /// Warning: any Pointer object referring to this list will be corrupted.
    pub fn clear(&mut self) {
        self.head_ = OUT_OF_BOUNDS;
        self.tail_ = OUT_OF_BOUNDS;
        self.nodes_.clear();
        self.id_pool_.clear();
    }

    /// Returns the next free index.
    /// This value will be invalid if any insert or remove operation is performed on the list.
    pub fn next_free_index(&self) -> usize {
        if self.id_pool_.is_empty() {
            self.nodes_.len()
        } else {
            // unwrap is safe after !is_empty() check
            *self.id_pool_.last().unwrap()
        }
    }

    #[inline(always)]
    /// Returns the item key at index
    pub fn get_k(&self, index: usize) -> Result<&K, MapError> {
        let rv = self
            .nodes_
            .get(index)
            .ok_or_else(|| MapError::InternalError("error, item not found".to_string()))?
            .as_ref()
            .ok_or_else(|| MapError::InternalError("error, item was not active".to_string()))?;
        Ok(&rv.key_)
    }

    #[inline(always)]
    /// Returns the item value at index
    pub fn get_v(&self, index: usize) -> Result<&V, MapError> {
        let rv = self
            .nodes_
            .get(index)
            .ok_or_else(|| MapError::InternalError("error, item not found".to_string()))?
            .as_ref()
            .ok_or_else(|| MapError::InternalError("error, item was not active".to_string()))?;
        Ok(&rv.value_)
    }

    #[inline(always)]
    /// Returns the item key and value at index
    ///
    /// # Examples
    ///
    /// ```
    /// # use cpp_map::LinkedList;
    /// let mut ll = LinkedList::<i8, i8>::default();
    /// ll.ordered_insert(1,1);
    /// assert!(ll.get(ll.head()).is_ok());
    /// assert_eq!(ll.get(ll.head()).unwrap(), (&1,&1));
    /// ll.ordered_insert(0,0);
    /// assert!(ll.get(ll.head()).is_ok());
    /// assert_eq!(ll.get(ll.head()).unwrap(), (&0,&0));
    /// assert!(ll.get(ll.tail()).is_ok());
    /// assert_eq!(ll.get(ll.tail()).unwrap(), (&1,&1));
    ///
    /// ```
    pub fn get(&self, index: usize) -> Result<(&K, &V), MapError> {
        if index == OUT_OF_BOUNDS {
            return Err(MapError::InternalError(format!(
                "Invalid pointer (moved past start/end). {}:{}",
                file!(),
                line!()
            )));
        }
        let rv = self
            .nodes_
            .get(index)
            .ok_or_else(|| MapError::InternalError("error, item not found".to_string()))?
            .as_ref()
            .ok_or_else(|| MapError::InternalError("error, item was not active".to_string()))?;
        Ok((&rv.key_, &rv.value_))
    }

    #[inline(always)]
    /// Returns the previous key item of item at index
    pub fn get_prev_k(&self, index: usize) -> Result<&K, MapError> {
        let prev = self
            .nodes_
            .get(index)
            .as_ref()
            .ok_or_else(|| MapError::InternalError("error, item not found".to_string()))?
            .as_ref()
            .ok_or_else(|| MapError::InternalError("error, item was None".to_string()))?
            .prev_;

        let node = self
            .nodes_
            .get(prev)
            .ok_or_else(|| MapError::InternalError("error, prev item not found".to_string()))?
            .as_ref();
        Ok(&node
            .ok_or_else(|| MapError::InternalError("error, item was not active".to_string()))?
            .key_)
    }

    /// Add an item at the front of the list
    /// Note that this ignores the order of items, use with care.
    fn push_front_(&mut self, key: K, value: V) -> Result<usize, MapError> {
        let insertion_index = if !self.id_pool_.is_empty() {
            self.id_pool_.pop().unwrap()
        } else {
            self.nodes_.len()
        };
        let new_node = if let Some(ref mut prev_head) = self.nodes_.get_mut(self.head_) {
            if let Some(prev_head) = prev_head {
                //println!("prev_head:{:?}", prev_head);
                // there were a previous head
                let new_node = Node {
                    next_: self.head_,
                    prev_: OUT_OF_BOUNDS,
                    key_: key,
                    value_: value,
                };
                self.head_ = insertion_index;
                prev_head.prev_ = insertion_index;
                new_node
            } else {
                return Err(MapError::InternalError(format!(
                    "Should not happen error™ at {}:{}",
                    file!(),
                    line!()
                )));
            }
        } else {
            // This will be the first element in the list
            //println!("no prev_head:{}", self.head);
            self.head_ = insertion_index;
            self.tail_ = insertion_index;
            Node {
                next_: OUT_OF_BOUNDS,
                prev_: OUT_OF_BOUNDS,
                key_: key,
                value_: value,
            }
        };
        //println!("push_front Pushed {:?} at index:{}", new_node, curr_len);
        Ok(self.replace_or_push_(insertion_index, new_node))
    }

    #[inline(always)]
    /// insert at position or append at back of the list
    /// Note that this ignores the order of items, use with care.
    fn replace_or_push_(&mut self, insertion_index: usize, new_node: Node<K, V>) -> usize {
        if insertion_index == self.nodes_.len() {
            self.nodes_.push(Some(new_node));
        } else {
            // get_mut will never fail
            let _ = self
                .nodes_
                .get_mut(insertion_index)
                .unwrap()
                .replace(new_node);
        }
        insertion_index
    }

    /// insert a new value before the element at index
    /// Note that this ignores the order of items, use with care.
    fn insert_before_(&mut self, index: usize, key: K, value: V) -> Result<usize, MapError> {
        if index == OUT_OF_BOUNDS {
            return self.push_front_(key, value);
        }

        let insertion_index = if !self.id_pool_.is_empty() {
            self.id_pool_.pop().unwrap()
        } else {
            self.nodes_.len()
        };

        let new_node = if let Some(ref mut next_node) = self.nodes_.get_mut(index) {
            if let Some(next_node) = next_node {
                //println!("next_node:{:?}", next_node);
                // there were a previous head
                let new_node = Node {
                    next_: index,
                    prev_: next_node.prev_,
                    key_: key,
                    value_: value,
                };
                next_node.prev_ = insertion_index;
                new_node
            } else {
                return Err(MapError::InternalError(format!(
                    "Should not happen error™ at {}:{}",
                    file!(),
                    line!()
                )));
            }
        } else {
            // This will be the first element in the list
            //println!("no prev_head:{}", self.head);
            self.head_ = insertion_index;
            self.tail_ = insertion_index;
            Node {
                next_: OUT_OF_BOUNDS,
                prev_: OUT_OF_BOUNDS,
                key_: key,
                value_: value,
            }
        };
        let prev_node = new_node.prev_;

        //println!("insert_before Pushed {:?} at index:{}", new_node, curr_len);
        {
            let _i = self.replace_or_push_(insertion_index, new_node);
            #[cfg(feature = "console_debug")]
            assert_eq!(insertion_index, _i);
        };

        if prev_node != OUT_OF_BOUNDS {
            if let Some(prev_node) = self.nodes_.get_mut(prev_node) {
                if let Some(prev_node) = prev_node {
                    prev_node.next_ = insertion_index;
                } else {
                    return Err(MapError::InternalError(format!(
                        "Should not happen error™ at {}:{}",
                        file!(),
                        line!()
                    )));
                }
            } else {
                // this case should have been handled by the initial push_front()
                return Err(MapError::InternalError(format!(
                    "Should not happen error™ at {}:{}",
                    file!(),
                    line!()
                )));
            }
        } else {
            // We just pushed at the first position
            self.head_ = insertion_index;
        }
        //println!("insert_before inserted at {}", insertion_index);
        Ok(insertion_index)
    }

    /// Add an item at the back of the list
    fn push_back_(&mut self, key: K, value: V) -> Result<usize, MapError> {
        let insertion_index = if !self.id_pool_.is_empty() {
            self.id_pool_.pop().unwrap()
        } else {
            self.nodes_.len()
        };
        let new_node = if let Some(prev_tail) = self.nodes_.get_mut(self.tail_) {
            if let Some(prev_tail) = prev_tail {
                //println!("prev_tail:{:?}", prev_tail);
                // there were a previous tail
                let new_node = Node {
                    next_: OUT_OF_BOUNDS,
                    prev_: self.tail_,
                    key_: key,
                    value_: value,
                };
                self.tail_ = insertion_index;
                prev_tail.next_ = insertion_index;
                new_node
            } else {
                return Err(MapError::InternalError(format!(
                    "Should not happen error™ at {}:{}",
                    file!(),
                    line!()
                )));
            }
        } else {
            // This will be the first element in the list
            //println!("no prev_tail:{}", self.tail);
            self.head_ = insertion_index;
            self.tail_ = insertion_index;
            Node {
                next_: OUT_OF_BOUNDS,
                prev_: OUT_OF_BOUNDS,
                key_: key,
                value_: value,
            }
        };
        //println!("push_back Pushed {:?} at index:{}", new_node, insertion_index);
        {
            let _insert_index = self.replace_or_push_(insertion_index, new_node);
            #[cfg(feature = "console_debug")]
            assert_eq!(_insert_index, insertion_index);
        }
        Ok(insertion_index)
    }

    #[inline(always)]
    /// Insert item at position defined by Order (lesser first)
    /// This is the same as 'ordered_insert_pos()' with self.head_ as position hint
    /// Insert item by Order (lesser first) with a position hint.
    ///
    /// Note that insert(key, value) is a NOP if the key already exists, not even the new value will be used.
    ///
    /// # Examples
    ///
    /// ```
    /// # use cpp_map::LinkedList;
    /// let mut ll = LinkedList::<i8, i8>::default();
    /// ll.ordered_insert(1,1);
    /// assert!(ll.get(ll.head()).is_ok());
    /// assert_eq!(ll.get(ll.head()).unwrap(), (&1,&1));
    /// ll.ordered_insert(0,0);
    /// assert!(ll.get(ll.head()).is_ok());
    /// assert_eq!(ll.get(ll.head()).unwrap(), (&0,&0));
    /// ll.ordered_insert(0,100); // <- this is a NOP
    /// assert!(ll.get(ll.head()).is_ok());
    /// assert_eq!(ll.get(ll.head()).unwrap(), (&0,&0));
    /// ```
    pub fn ordered_insert(&mut self, key: K, value: V) -> Result<usize, MapError> {
        self.ordered_insert_pos(key, value, self.head_)
    }

    /// Insert item by Order (lesser first) with a position hint.
    /// Note that insert(key, value) is a NOP if the key already exists, not even the new value will be used.
    /// # Examples
    ///
    /// ```
    /// # use cpp_map::LinkedList;
    /// let mut ll = LinkedList::<i8, i8>::default();
    /// ll.ordered_insert(1,1);
    /// ll.ordered_insert_pos(2,2,0);
    /// assert!(ll.get(ll.head()).is_ok());
    /// assert_eq!(ll.get(ll.head()).unwrap(), (&1,&1));
    /// assert!(ll.get(ll.tail()).is_ok());
    /// assert_eq!(ll.get(ll.tail()).unwrap(), (&2,&2));
    ///
    /// ```
    pub fn ordered_insert_pos(
        &mut self,
        key: K,
        value: V,
        position: usize,
    ) -> Result<usize, MapError> {
        if self.head_ == OUT_OF_BOUNDS {
            // list is empty, ignore position and insert
            return self.push_back_(key, value);
        }
        //println!("insert at position {}, key={:?} head={}", position, key, self.head_);
        let mut insert_before: Option<usize> = None;

        let (mut curr_index, first_node) = match self.nodes_.get(position) {
            Some(Some(first_node)) => (position, first_node),
            _ => (
                self.head_,
                self.nodes_
                    .get(self.head_)
                    .unwrap()
                    .as_ref()
                    .ok_or_else(|| {
                        MapError::InternalError(format!(
                            "head_ item was None {}:{}",
                            file!(),
                            line!()
                        ))
                    })?,
            ),
        };

        let cmp = key.cmp(&first_node.key_);
        //println!("curr_index:{}, first_node.key={:?}, cmp={:?}", curr_index, first_node.key, cmp);

        #[allow(clippy::collapsible_else_if)] // false positive?
        if (cmp == Ordering::Greater) || (cmp == Ordering::Equal) {
            //println!("search down, insert after equals");
            // we are searching down the list, stop at first Less
            while let Some(Some(sample)) = self.nodes_.get(curr_index) {
                // move past Ordering::Equal
                match key.cmp(&sample.key_) {
                    Ordering::Equal => {
                        return Ok(curr_index); // Insert with an already existing key is a 'nop'
                    }
                    Ordering::Less => {
                        insert_before = Some(curr_index);
                        break;
                    }
                    _ => {
                        curr_index = sample.next_;
                    }
                }
            }
        } else {
            if cmp == Ordering::Less {
                insert_before = Some(curr_index);
            }
            //println!("search up, insert after equals. tmp insert_before:{:?}", insert_before);
            // we are searching up the list, stop at first Equal or Greater
            while let Some(Some(sample)) = self.nodes_.get(curr_index) {
                match key.cmp(&sample.key_) {
                    Ordering::Equal => {
                        return Ok(curr_index); // Insert with an already existing key is a 'nop'
                    }
                    Ordering::Less => {
                        insert_before = Some(curr_index);
                        curr_index = sample.prev_;
                        //println!("continue: curr_index:{}", curr_index);
                    }
                    _ => {
                        //println!("break: insert_before:{:?}", insert_before);
                        break;
                    }
                }
            }
        }

        if let Some(insert_before) = insert_before {
            //println!("inserting before {}", insert_before);
            self.insert_before_(insert_before, key, value)
        } else {
            //println!("pushing at the back");
            self.push_back_(key, value)
        }
    }

    /// Returns the first element in the container whose key is not considered to go
    /// before position (i.e., either it is equivalent or goes after).
    /// If 'search_from_head' is true the search will be performed from the head otherwise from the tail.
    /// Returns None if no data is found
    /// # Examples
    ///
    /// ```
    /// # use cpp_map::LinkedList;
    /// let mut ll = LinkedList::<i8, i8>::default();
    /// ll.ordered_insert(1,1);
    /// ll.ordered_insert(2,2);
    /// ll.ordered_insert(3,3);
    /// let lb = ll.get(ll.lower_bound(2).unwrap().unwrap()).unwrap();
    /// assert_eq!(lb, (&2,&2));
    /// let lb = ll.get(ll.lower_bound(0).unwrap().unwrap()).unwrap();
    /// assert_eq!(lb, (&1,&1));
    /// let lb = ll.get(ll.lower_bound(1).unwrap().unwrap()).unwrap();
    /// assert_eq!(lb, (&1,&1));
    /// let lb = ll.get(ll.lower_bound(3).unwrap().unwrap()).unwrap();
    /// assert_eq!(lb, (&3,&3));
    /// assert!( ll.lower_bound(4).unwrap().is_none());
    /// ```
    pub fn lower_bound(&self, key: K) -> Result<Option<usize>, MapError> {
        #[cfg(feature = "console_debug")]
        {
            let mut iter = self.iter();
            let mut flips = 0_usize;
            let mut last_cmp = iter.next().map(|(first, _)| key.cmp(first));

            for (node, _) in iter {
                let cmp = Some(key.cmp(node));
                if cmp != last_cmp {
                    last_cmp = cmp;
                    flips += 1;
                }
            }
            if flips > 1 {
                println!("\nkey={:?}", key);
                for (n, _) in self.iter() {
                    println!("key.cmp({:?})=={:?}-{:?}", n, n.cmp(&key), key.cmp(n));
                }
            }
        }

        // sequential search from the rear
        if self.tail_ == OUT_OF_BOUNDS {
            return Ok(None);
        }
        let mut last_match: Option<usize> = None;
        let mut curr_index = self.tail_;
        while let Some(Some(sample)) = self.nodes_.get(curr_index) {
            if key.cmp(&sample.key_) != Ordering::Greater {
                //println!("ignoring :{:?} ", sample.key);
                last_match = Some(curr_index);
                curr_index = sample.prev_;
            } else {
                return Ok(last_match);
            }
        }
        Ok(last_match)
    }

    #[inline(always)]
    /// Pop the head item
    ///
    /// # Examples
    /// ```
    /// # use cpp_map::LinkedList;
    /// let mut ll = LinkedList::<i8, i8>::default();
    /// let _ = ll.ordered_insert(1, 0); // 0
    /// let _ = ll.ordered_insert(2, 1); // 1
    /// assert_eq!(ll.pop_front().unwrap().unwrap(), (1_i8,0_i8));
    /// assert_eq!(ll.pop_front().unwrap().unwrap(), (2_i8,1_i8));
    /// ```
    pub fn pop_front(&mut self) -> Result<Option<(K, V)>, MapError> {
        self.remove_(self.head_)
    }

    #[inline(always)]
    /// Pop the tail item
    ///
    /// # Examples
    /// ```
    /// # use cpp_map::LinkedList;
    /// let mut ll = LinkedList::<i8, i8>::default();
    /// let _ = ll.ordered_insert(1, 0); // 0
    /// let _ = ll.ordered_insert(2, 1); // 1
    /// assert_eq!(ll.pop_back().unwrap().unwrap(), (2_i8,1_i8));
    /// assert_eq!(ll.pop_back().unwrap().unwrap(), (1_i8,0_i8));
    /// ```
    pub fn pop_back(&mut self) -> Result<Option<(K, V)>, MapError> {
        self.remove_(self.tail_)
    }

    #[inline(always)]
    /// Peek the head key
    ///
    /// # Examples
    /// ```
    /// # use cpp_map::LinkedList;
    /// let mut ll = LinkedList::<i8, i8>::default();
    /// let _ = ll.ordered_insert(1, 0); // 0
    /// let _ = ll.ordered_insert(2, 1); // 1
    /// assert_eq!(ll.peek_front_k().unwrap(), &1_i8);
    /// ```
    pub fn peek_front_k(&self) -> Option<&K> {
        match self.nodes_.get(self.head_) {
            Some(Some(node)) => Some(&node.key_),
            _ => None,
        }
    }

    #[inline(always)]
    /// Peek the tail key
    ///
    /// # Examples
    /// ```
    /// # use cpp_map::LinkedList;
    /// let mut ll = LinkedList::<i8, i8>::default();
    /// let _ = ll.ordered_insert(1, 0); // 0
    /// let _ = ll.ordered_insert(2, 1); // 1
    /// assert_eq!(ll.peek_back_k().unwrap(), &2_i8);
    /// ```
    pub fn peek_back_k(&self) -> Option<&K> {
        match self.nodes_.get(self.tail_) {
            Some(Some(node)) => Some(&node.key_),
            _ => None,
        }
    }

    #[inline(always)]
    /// Return the tail index
    pub fn tail(&self) -> usize {
        self.tail_
    }

    #[inline(always)]
    /// Return the head index
    pub fn head(&self) -> usize {
        self.head_
    }

    #[inline(always)]
    /// Remove the item at index, return item value if found
    fn remove_(&mut self, index: usize) -> Result<Option<(K, V)>, MapError> {
        let rv = self.remove__(index)?;
        Ok(Some(rv.1))
    }

    /// Disconnect and remove the item at index, return item value if found
    fn remove__(&mut self, index: usize) -> Result<(usize, (K, V), usize), MapError> {
        if self.head_ == OUT_OF_BOUNDS {
            return Err(MapError::InternalError(format!(
                "Could not find element to remove {}:{}",
                file!(),
                line!()
            )));
        }
        //println!("remove {} before:{:?}", index, self);
        let rv = if self.head_ != OUT_OF_BOUNDS {
            // list was not empty
            let operation = if let Some(node) = self.nodes_.get(index) {
                let mut operation = EraseOperation {
                    change_prev_: None,
                    erase_: index,
                    change_next_: None,
                };
                if let Some(node) = node {
                    // Check node next
                    if let Some(next) = self.nodes_.get(node.next_) {
                        if next.is_some() {
                            // node had a next
                            operation.change_next_ = Some((node.next_, node.prev_));
                        } else {
                            return Err(MapError::InternalError(format!(
                                "Should not happen error™ at {}:{}",
                                file!(),
                                line!()
                            )));
                        }
                    }

                    // Check prev node
                    if let Some(prev) = self.nodes_.get(node.prev_) {
                        if prev.is_some() {
                            // node had a prev
                            operation.change_prev_ = Some((node.prev_, node.next_));
                        } else {
                            return Err(MapError::InternalError(format!(
                                "Should not happen error™ at {}:{}",
                                file!(),
                                line!()
                            )));
                        }
                    }
                    Some(operation)
                } else {
                    return Err(MapError::InternalError(format!(
                        "Should not happen error™ at {}:{}",
                        file!(),
                        line!()
                    )));
                }
            } else {
                // index was not found, todo: report error?
                None
            };
            if let Some(operation) = operation {
                Some(self.erase_node_(operation)?)
            } else {
                None
            }
        } else {
            // list was empty
            None
        };
        rv.ok_or_else(|| {
            MapError::InternalError(format!(
                "Could not find element to remove {}:{}",
                file!(),
                line!()
            ))
        })
    }

    /// do the actual erase now that we know how to do it (work around for the borrow checker).
    fn erase_node_(
        &mut self,
        operation: EraseOperation,
    ) -> Result<(usize, (K, V), usize), MapError> {
        //println!("erase_operation {:?}", operation);
        match (operation.change_prev_, operation.change_next_) {
            (Some((prev_i, new_next)), Some((next_i, new_prev))) => {
                #[cfg(feature = "console_debug")]
                {
                    assert_eq!(new_next, next_i);
                    assert_eq!(prev_i, new_prev);
                }
                match self.nodes_.get_mut(prev_i) {
                    Some(Some(node)) => {
                        node.next_ = new_next;
                    }
                    _ => {
                        return Err(MapError::InternalError(format!(
                            "Should not happen error™ at {}:{}",
                            file!(),
                            line!()
                        )));
                    }
                };
                match self.nodes_.get_mut(next_i) {
                    Some(Some(node)) => {
                        node.prev_ = new_prev;
                    }
                    _ => {
                        return Err(MapError::InternalError(format!(
                            "Should not happen error™ at {}:{}",
                            file!(),
                            line!()
                        )));
                    }
                };
            }
            (None, Some((new_head, new_head_prev))) => match self.nodes_.get_mut(new_head) {
                Some(Some(node)) => {
                    node.prev_ = new_head_prev;
                    self.head_ = new_head;
                }
                _ => {
                    return Err(MapError::InternalError(format!(
                        "Should not happen error™ at {}:{}",
                        file!(),
                        line!()
                    )));
                }
            },
            (Some((new_tail, new_tail_next)), None) => match self.nodes_.get_mut(new_tail) {
                Some(Some(node)) => {
                    node.next_ = new_tail_next;
                    self.tail_ = new_tail;
                }
                _ => {
                    return Err(MapError::InternalError(format!(
                        "Should not happen error™ at {}:{}",
                        file!(),
                        line!()
                    )));
                }
            },
            (None, None) => {
                self.head_ = OUT_OF_BOUNDS;
                self.tail_ = OUT_OF_BOUNDS
            }
        }
        match self.nodes_.get_mut(operation.erase_) {
            Some(old_head) => {
                // Replace the node with None
                if let Some(old_head) = old_head.take() {
                    self.id_pool_.push(operation.erase_);
                    return Ok((
                        old_head.prev_,
                        (old_head.key_, old_head.value_),
                        old_head.next_,
                    ));
                }

                Err(MapError::InternalError(format!(
                    "Should not happen error™ at {}:{}",
                    file!(),
                    line!()
                )))
            }
            _ => Err(MapError::InternalError(format!(
                "Should not happen error™, element to erase not found {} at {}:{}",
                operation.erase_,
                file!(),
                line!()
            ))),
        }
    }
}

#[derive(Clone, Debug)]
/// A double ended iterator
pub struct ListIterator<'a, K: 'a, V: 'a>
where
    K: Debug,
    V: Debug,
{
    list_: &'a LinkedList<K, V>,
    my_next_: usize,
}

impl<'a, K: 'a, V: 'a> std::iter::Iterator for ListIterator<'a, K, V>
where
    K: Debug,
    V: Debug,
{
    type Item = (&'a K, &'a V);

    #[inline]
    /// Step the iterator forward one step
    fn next(&mut self) -> Option<(&'a K, &'a V)> {
        if self.my_next_ == OUT_OF_BOUNDS {
            return None;
        }
        //println!("Returning value at index:{}", self.my_next);
        if let Some(node) = self.list_.nodes_.get(self.my_next_)? {
            if self.my_next_ == self.list_.tail_ {
                self.my_next_ = OUT_OF_BOUNDS;
            } else {
                self.my_next_ = node.next_
            }
            Some((&node.key_, &node.value_))
        } else {
            self.my_next_ = OUT_OF_BOUNDS;
            None
        }
    }
}

impl<'a, K: 'a, V: 'a> DoubleEndedIterator for ListIterator<'a, K, V>
where
    K: Debug,
    V: Debug,
{
    #[inline]
    /// Step the iterator backward one step
    fn next_back(&mut self) -> Option<(&'a K, &'a V)> {
        if let Some(node) = self.list_.nodes_.get(self.my_next_)? {
            if self.my_next_ == self.list_.tail_ {
                self.my_next_ = OUT_OF_BOUNDS;
            } else {
                self.my_next_ = node.prev_
            }
            Some((&node.key_, &node.value_))
        } else {
            self.my_next_ = OUT_OF_BOUNDS;
            None
        }
    }
}

/// An effort to emulate a C++ std::map iterator in Rust.
/// It will have functionality like:
/// prev(), next(), get(), erase(), lower_bound(), replace_key()
pub struct PIterator<K, V>
where
    K: Debug,
    V: Debug,
{
    current: usize,
    list: Rc<RefCell<LinkedList<K, V>>>,
}

#[allow(dead_code)]
impl<K, V> PIterator<K, V>
where
    K: Clone + Debug + Unpin + Ord + PartialOrd,
    V: Clone + Debug + Unpin,
{
    /// Initiates the pointer with a list, set current to the head of the list.
    pub fn new(list: Rc<RefCell<LinkedList<K, V>>>) -> Result<Self, MapError> {
        let head = list.try_borrow()?.head_;
        Ok(Self {
            current: head,
            list,
        })
    }

    /// Initiates the pointer with a list, set index.
    pub fn new_2(list: Rc<RefCell<LinkedList<K, V>>>, current: usize) -> Self {
        Self { current, list }
    }

    #[inline(always)]
    /// Returns a clone of the key at current position
    pub fn get_k(&self) -> Result<K, MapError> {
        Ok(self.list.try_borrow()?.get(self.current)?.0.clone())
    }

    #[inline(always)]
    /// Returns a clone of the value at current position
    pub fn get_v(&self) -> Result<V, MapError> {
        Ok(self.list.try_borrow()?.get(self.current)?.1.clone())
    }

    #[allow(clippy::should_implement_trait)]
    #[inline(always)]
    /// Move to the next element.
    /// Note that this is NOT a Rust iterator next() method.
    /// Always check validity of the iterator with is_ok() after next()
    // todo: change the return value to Result<bool, MapError>
    pub fn next(&mut self) -> Result<(), MapError> {
        let list_borrow = self.list.try_borrow()?;
        match list_borrow.nodes_.get(self.current) {
            Some(Some(node)) => self.current = node.next_,
            // Some(None) nodes should be inaccessible
            Some(None) => {
                return Err(MapError::InternalError(format!(
                    "next() failed at index:{}. {}:{}",
                    self.current,
                    file!(),
                    line!()
                )));
            }
            None => self.current = OUT_OF_BOUNDS,
        }
        Ok(())
    }

    #[inline(always)]
    /// Move to the previous element
    /// Always check validity of the iterator with is_ok() after prev()
    // todo: change the return value to Result<bool, MapError>
    pub fn prev(&mut self) -> Result<(), MapError> {
        let list_borrow = self.list.try_borrow()?;
        match list_borrow.nodes_.get(self.current) {
            Some(Some(node)) => self.current = node.prev_,
            // Some(None) nodes should be inaccessible
            Some(None) => {
                return Err(MapError::InternalError(format!(
                    "prev() failed at index:{}. {}:{}",
                    self.current,
                    file!(),
                    line!()
                )));
            }
            None => self.current = OUT_OF_BOUNDS,
        }
        Ok(())
    }

    #[inline(always)]
    /// Move to the first element
    pub fn move_to_head(&mut self) -> Result<(), MapError> {
        self.current = self.list.try_borrow()?.head_;
        Ok(())
    }

    #[inline(always)]
    /// Move to the last element
    pub fn move_to_tail(&mut self) -> Result<(), MapError> {
        self.current = self.list.try_borrow()?.tail_;
        Ok(())
    }

    #[inline(always)]
    /// Return true if pointer has *NOT* moved past beginning or end of the list
    pub fn is_ok(&self) -> Result<bool, MapError> {
        Ok(self.current != OUT_OF_BOUNDS
            && matches!(
                self.list.try_borrow()?.nodes_.get(self.current),
                Some(Some(_))
            ))
    }

    #[inline(always)]
    /// Return true if pointer is at head position or if the list is empty
    pub fn is_at_head(&self) -> Result<bool, MapError> {
        Ok(self.current == self.list.try_borrow()?.head_)
    }

    #[inline(always)]
    /// Return true if pointer is at tail position or if the list is empty
    pub fn is_at_tail(&self) -> Result<bool, MapError> {
        Ok(self.current == self.list.try_borrow()?.tail_)
    }

    #[inline(always)]
    /// Replace current key. This will destroy the internal order of element if you
    /// replace an element with something out of order.
    pub fn replace_key(&mut self, key: K) -> Result<(), MapError> {
        let mut list = std::pin::Pin::new(self.list.try_borrow_mut()?);
        if let Some(Some(node)) = list.nodes_.get_mut(self.current) {
            node.key_ = key;
        }
        Ok(())
    }

    #[inline(always)]
    /// returns current index
    pub fn current(&self) -> usize {
        self.current
    }

    #[inline(always)]
    /// Remove the current element and return it. Move current to the old prev value if exist.
    /// Else pick old next index.
    /// Note: make sure that there are no other Pointer objects at this position.
    pub fn remove_current(&mut self) -> Result<(K, V), MapError> {
        let rv = self.list.try_borrow_mut()?.remove__(self.current)?;
        if rv.0 != OUT_OF_BOUNDS {
            self.current = rv.0;
        } else {
            self.current = rv.2;
        }
        Ok(rv.1)
    }

    #[inline(always)]
    /// Returns a new Pointer positioned at the lower bound item.
    /// Lower bound item is the first element in the container whose key is not considered to go
    /// before position (i.e., either it is equivalent or goes after).
    /// Returns a Pointer where is_ok() returns false if no data is found
    pub fn lower_bound(list: Rc<RefCell<LinkedList<K, V>>>, key: K) -> Result<Self, MapError> {
        let position = list.try_borrow()?.lower_bound(key)?;
        if let Some(position) = position {
            Ok(Self {
                list,
                current: position,
            })
        } else {
            // Return a Pointer that is out of bounds
            Ok(Self {
                list,
                current: OUT_OF_BOUNDS,
            })
        }
    }
}

impl<K, V> Debug for PIterator<K, V>
where
    K: Debug + Unpin + Ord + PartialOrd,
    V: Debug + Unpin,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "PIterator({})", self.current)
    }
}

impl<K, V> Clone for PIterator<K, V>
where
    K: Debug + Unpin + Ord + PartialOrd,
    V: Debug + Unpin,
{
    fn clone(&self) -> Self {
        Self {
            current: self.current,
            list: Rc::clone(&self.list),
        }
    }
}

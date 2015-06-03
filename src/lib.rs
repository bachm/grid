#![feature(alloc, core)]

extern crate alloc;

use self::alloc::heap;
use std::mem;
use std::ptr;
use std::iter::Iterator;
use std::marker::PhantomData;
use std::slice;
use std::ops::{Index, IndexMut};

/// A dynamically allocated two-dimensional array with fixed size.
pub struct DynArray2<T> {
    ptr: *mut T,
    width: u16,
    height: u16
}

impl<T: Clone> DynArray2<T> {
    /// Constructs an array with the given width and height by cloning `element`.
    /// Will panic if width, height or size of T are zero, or allocation fails.
    pub fn new(width: u16, height: u16, element: T) -> DynArray2<T> {
        let ptr = DynArray2::init(width, height, element).expect("DynArray2::new called with invalid input.");
        DynArray2 { ptr: ptr, width: width, height: height }
    }
    
    /// Constructs an array with the given width and height by cloning `element`.
    /// Will return None if width, height or size of T are zero, or allocation fails.
    pub fn new_checked(width: u16, height: u16, element: T) -> Option<DynArray2<T>> {
        DynArray2::init(width, height, element).map(|ptr| DynArray2 { ptr: ptr, width: width, height: height })
    }
    
    fn init(width: u16, height: u16, element: T) -> Option<*mut T> {
        if mem::size_of::<T>() > 0 && width > 0 && height > 0 {
            let count = width as usize * height as usize;
            let bytes = count * mem::size_of::<T>();
            let ptr = unsafe { heap::allocate(bytes, mem::align_of::<T>()) as *mut T };
            if ptr.is_null() { return None }
            for i in 0..count as isize {
                unsafe { ptr::write(ptr.offset(i), element.clone()); };
            }
            Some(ptr)
        } else {
            None
        }
    }
}

impl<T> DynArray2<T> {    
    pub fn iter<'a>(&'a self) -> Iter<'a, T> {
        Iter { ptr: self.ptr, end: self.end(), marker: PhantomData }
    }
    
    pub fn iter_mut<'a>(&'a mut self) -> IterMut<'a, T> {
        IterMut { ptr: self.ptr, end: self.end_mut(), marker: PhantomData }
    }
    
    pub fn rows<'a>(&'a self) -> Rows<'a, T> {
        Rows { ptr: self.ptr, end: self.end(), len: self.width as usize, marker: PhantomData }
    }
    
    pub fn rows_mut<'a>(&'a mut self) -> RowsMut<'a, T> {
        RowsMut { ptr: self.ptr, end: self.end_mut(), len: self.width as usize, marker: PhantomData }
    }
    
    pub fn get(&self, x: u16, y: u16) -> Option<&T> {
        if x < self.width && y < self.height {
            unsafe { self.ptr.offset(x as isize + y as isize * self.width as isize).as_ref() }
        } else {
            None
        }
    }
    
    pub fn get_mut(&mut self, x: u16, y: u16) -> Option<&mut T> {
        if x < self.width && y < self.height {
            unsafe { self.ptr.offset(x as isize + y as isize * self.width as isize).as_mut() }
        } else {
            None
        }
    }
    
    pub fn width(&self) -> u16 {
        self.width
    }
    
    pub fn height(&self) -> u16 {
        self.height
    }
    
    #[inline]
    fn end(&self) -> *const T {
        unsafe { self.ptr.offset(self.width as isize * self.height as isize) }
    }
    
    #[inline]
    fn end_mut(&mut self) -> *mut T {
        unsafe { self.ptr.offset(self.width as isize * self.height as isize) }
    }
}

impl<T> Drop for DynArray2<T> {
    fn drop(&mut self) {
        unsafe {
            for e in self.iter_mut() { 
                ptr::read(e);
            }
            let bytes = self.width as usize * self.height as usize * mem::size_of::<T>();
            heap::deallocate(self.ptr as *mut u8, bytes, mem::align_of::<T>());
        }
    }
}

pub struct Iter<'a, T: 'a> {
    ptr: *const T,
    end: *const T,
    marker: PhantomData<&'a T>
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;
    fn next(&mut self) -> Option<&'a T> {
        if self.ptr == self.end { 
            None
        } else {
            let reference: &T = unsafe { mem::transmute(self.ptr) };
            self.ptr = unsafe { self.ptr.offset(1) };
            Some(reference)
        }
    }
}

pub struct IterMut<'a, T: 'a> {
    ptr: *mut T,
    end: *mut T,
    marker: PhantomData<&'a mut T>
}

impl<'a, T> Iterator for IterMut<'a, T> {
    type Item = &'a mut T;
    fn next(&mut self) -> Option<&'a mut T> {
        if self.ptr == self.end { 
            None
        } else {
            let reference: &mut T = unsafe { mem::transmute(self.ptr) };
            self.ptr = unsafe { self.ptr.offset(1) };
            Some(reference)
        }
    }
}

pub struct Rows<'a, T: 'a> {
    ptr: *const T,
    end: *const T,
    len: usize,
    marker: PhantomData<&'a T>
}

impl<'a, T> Iterator for Rows<'a, T> {
    type Item = &'a [T];
    fn next(&mut self) -> Option<&'a [T]> {
        if self.ptr == self.end { 
            None
        } else {
            let slice = unsafe { slice::from_raw_parts(self.ptr, self.len) };
            self.ptr = unsafe { self.ptr.offset(self.len as isize) };
            Some(slice)
        }
    }
}

pub struct RowsMut<'a, T: 'a> {
    ptr: *mut T,
    end: *mut T,
    len: usize,
    marker: PhantomData<&'a mut T>
}

impl<'a, T> Iterator for RowsMut<'a, T> {
    type Item = &'a mut [T];
    fn next(&mut self) -> Option<&'a mut [T]> {
        if self.ptr == self.end { 
            None
        } else {
            let slice = unsafe { slice::from_raw_parts_mut(self.ptr, self.len) };
            self.ptr = unsafe { self.ptr.offset(self.len as isize) };
            Some(slice)
        }
    }
}

const INDEX_ERROR_MSG: &'static str = "Attempted to index DynArray2 with invalid input."; 

impl<T> Index<(u16, u16)> for DynArray2<T> {
    type Output = T;
    
    fn index(&self, (x, y): (u16, u16)) -> &T {
        if x < self.width && y < self.height {
            unsafe {
                mem::transmute(self.ptr.offset(x as isize + y as isize * self.width as isize))
            }
        } else {
            panic!(INDEX_ERROR_MSG)
        }
    }
}

impl<T> IndexMut<(u16, u16)> for DynArray2<T> {    
    fn index_mut(&mut self, (x, y): (u16, u16)) -> &mut T {
        if x < self.width && y < self.height {
            unsafe {
                mem::transmute(self.ptr.offset(x as isize + y as isize * self.width as isize))
            }
        } else {
            panic!(INDEX_ERROR_MSG)
        }
    }
}


#[cfg(test)]
#[allow(unused_variables)]
mod test {    
    extern crate core;
    use super::DynArray2;
    use self::core::array::FixedSizeArray;
    use std::cell::Cell;
    
    #[test]
    fn get_get_mut() {
        let mut array = DynArray2::new(2, 2, 0u8);
        assert_eq!(array.get(0, 0), Some(&0u8));
        assert_eq!(array.get(0, 1), Some(&0u8));
        assert_eq!(array.get(1, 0), Some(&0u8));
        assert_eq!(array.get(1, 1), Some(&0u8));
        assert_eq!(array.get(2, 0), None);
        *array.get_mut(1, 0).unwrap() = 1u8;
        *array.get_mut(0, 1).unwrap() = 2u8;
        *array.get_mut(1, 1).unwrap() = 3u8;
        assert_eq!(array.get(0, 0), Some(&0u8));
        assert_eq!(array.get(1, 0), Some(&1u8));
        assert_eq!(array.get(0, 1), Some(&2u8));
        assert_eq!(array.get(1, 1), Some(&3u8));
        assert_eq!(array.get(2, 0), None);
    }
    
    #[test]
    fn iter() {
        let mut array = DynArray2::new(2, 2, 0u8);
        *array.get_mut(1, 0).unwrap() = 1;
        *array.get_mut(0, 1).unwrap() = 2;
        *array.get_mut(1, 1).unwrap() = 3;
        let mut iter = array.iter();
        assert_eq!(iter.next(), Some(&0));
        assert_eq!(iter.next(), Some(&1));
        assert_eq!(iter.next(), Some(&2));
        assert_eq!(iter.next(), Some(&3));
        assert_eq!(iter.next(), None);
    }
        
    #[test]
    fn iter_mut() {
        let mut array = DynArray2::new(2, 2, 0u8);
        *array.get_mut(1, 0).unwrap() = 1;
        *array.get_mut(0, 1).unwrap() = 2;
        *array.get_mut(1, 1).unwrap() = 3;
        let mut iter = array.iter_mut();
        assert_eq!(iter.next(), Some(&mut 0));
        assert_eq!(iter.next(), Some(&mut 1));
        assert_eq!(iter.next(), Some(&mut 2));
        assert_eq!(iter.next(), Some(&mut 3));
        assert_eq!(iter.next(), None);
    }
    
    #[test]
    fn rows() {
        let mut array = DynArray2::new(2, 2, 'A');
        *array.get_mut(1, 0).unwrap() = 'B';
        *array.get_mut(0, 1).unwrap() = 'C';
        *array.get_mut(1, 1).unwrap() = 'D';
        let mut iter = array.rows();
        assert_eq!(iter.next(), Some(['A', 'B'].as_slice()));
        assert_eq!(iter.next(), Some(['C', 'D'].as_slice()));
        assert_eq!(iter.next(), None);
    }
    
    #[test]
    fn rows_mut() {
        let mut array = DynArray2::new(2, 2, 'A');
        *array.get_mut(1, 0).unwrap() = 'B';
        *array.get_mut(0, 1).unwrap() = 'C';
        *array.get_mut(1, 1).unwrap() = 'D';
        let mut iter = array.rows_mut();
        assert_eq!(iter.next(), Some(['A', 'B'].as_mut_slice()));
        assert_eq!(iter.next(), Some(['C', 'D'].as_mut_slice()));
        assert_eq!(iter.next(), None);
    }
    
    #[test]
    fn dealloc() {
        #[derive(Clone)]
        struct Foo<'a>(&'a Cell<u8>);
        impl<'a> Drop for Foo<'a> {
            fn drop(&mut self) {
            let Foo(ref cell) = *self;
            cell.set(cell.get() + 1);
            }
        }
        let count = Cell::new(0u8);
        {
            let array = DynArray2::new(2, 2, Foo(&count));
        }
        // Four allocations plus the Foo passed to DynArray2::new.
        assert_eq!(count.get(), 5);
    }
    
    #[test]
    #[should_panic]
    fn phantom_type() {
        #[derive(Copy, Clone)]
        struct Foo;
        let array = DynArray2::new(3, 3, Foo);
    }
    
    #[test]
    #[should_panic]
    fn bad_width() {
        let array = DynArray2::new(0, 3, 0u8);
    }
    
    #[test]
    #[should_panic]
    fn bad_height() {
        let array = DynArray2::new(3, 0, 0u8);
    }
    
    #[test]
    fn indexing() {
        let mut array = DynArray2::new(2, 2, 0u8);
        array[(1, 0)] = 23;
        array[(0, 1)] = 45;
        assert_eq!(array[(1, 1)], 0u8);
    }
    
    #[test]
    #[should_panic]
    fn bad_indexing() {
        let array = DynArray2::new(2, 2, 0u8);
        let x = array[(3, 1)];
    }
}
#![feature(alloc, core)]

extern crate alloc;

use self::alloc::heap;

use std::mem;
use std::ptr;
use std::iter::Iterator;
use std::marker::PhantomData;
use std::slice;

/// A two-dimensional array with whose dimensions are fixed at construction.
pub struct Grid<T> {
    ptr: *mut T,
    width: u16,
    height: u16,
}

impl<T: Clone> Grid<T> {
    /// Constructs a grid with the given width and height by cloning `element`.
    /// Returns None if width, height or size of T are zero.
    pub fn new(width: u16, height: u16, element: T) -> Option<Grid<T>> {
        if mem::size_of::<T>() > 0 && width > 0 && height > 0 {
            let count = width as usize * height as usize;
            let bytes = count * mem::size_of::<T>();
            let ptr = unsafe { heap::allocate(bytes, mem::align_of::<T>()) as *mut T };
            for i in 0..count as isize {
                unsafe { ptr::write(ptr.offset(i), element.clone()); };
            }
            Some(Grid { ptr: ptr, width: width, height: height })
        } else {
            None
        }
    }
}

impl<T> Grid<T> {    
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

impl<T> Drop for Grid<T> {
    fn drop(&mut self) {
        unsafe {
           for e in self.iter_mut() { ptr::read(e); }
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

#[cfg(test)]
mod test {
    extern crate core;
    use super::Grid;
    use self::core::array::FixedSizeArray;
    use std::cell::Cell;
    
    #[test]
    fn get_get_mut() {
        let mut grid = Grid::new(2, 2, 0u8).unwrap();
        assert_eq!(grid.get(0, 0), Some(&0u8));
        assert_eq!(grid.get(0, 1), Some(&0u8));
        assert_eq!(grid.get(1, 0), Some(&0u8));
        assert_eq!(grid.get(1, 1), Some(&0u8));
        assert_eq!(grid.get(2, 0), None);
        *grid.get_mut(1, 0).unwrap() = 1u8;
        *grid.get_mut(0, 1).unwrap() = 2u8;
        *grid.get_mut(1, 1).unwrap() = 3u8;
        assert_eq!(grid.get(0, 0), Some(&0u8));
        assert_eq!(grid.get(1, 0), Some(&1u8));
        assert_eq!(grid.get(0, 1), Some(&2u8));
        assert_eq!(grid.get(1, 1), Some(&3u8));
        assert_eq!(grid.get(2, 0), None);
    }
    
    #[test]
    fn iter() {
        let mut grid = Grid::new(2, 2, 0u8).unwrap();
        *grid.get_mut(1, 0).unwrap() = 1;
        *grid.get_mut(0, 1).unwrap() = 2;
        *grid.get_mut(1, 1).unwrap() = 3;
        let mut iter = grid.iter();
        assert_eq!(iter.next(), Some(&0));
        assert_eq!(iter.next(), Some(&1));
        assert_eq!(iter.next(), Some(&2));
        assert_eq!(iter.next(), Some(&3));
        assert_eq!(iter.next(), None);
    }
        
    #[test]
    fn iter_mut() {
        let mut grid = Grid::new(2, 2, 0u8).unwrap();
        *grid.get_mut(1, 0).unwrap() = 1;
        *grid.get_mut(0, 1).unwrap() = 2;
        *grid.get_mut(1, 1).unwrap() = 3;
        let mut iter = grid.iter_mut();
        assert_eq!(iter.next(), Some(&mut 0));
        assert_eq!(iter.next(), Some(&mut 1));
        assert_eq!(iter.next(), Some(&mut 2));
        assert_eq!(iter.next(), Some(&mut 3));
        assert_eq!(iter.next(), None);
    }
    
    #[test]
    fn rows() {
        let mut grid = Grid::new(2, 2, 'A').unwrap();
        *grid.get_mut(1, 0).unwrap() = 'B';
        *grid.get_mut(0, 1).unwrap() = 'C';
        *grid.get_mut(1, 1).unwrap() = 'D';
        let mut iter = grid.rows();
        assert_eq!(iter.next(), Some(['A', 'B'].as_slice()));
        assert_eq!(iter.next(), Some(['C', 'D'].as_slice()));
        assert_eq!(iter.next(), None);
    }
    
    #[test]
    fn rows_mut() {
        let mut grid = Grid::new(2, 2, 'A').unwrap();
        *grid.get_mut(1, 0).unwrap() = 'B';
        *grid.get_mut(0, 1).unwrap() = 'C';
        *grid.get_mut(1, 1).unwrap() = 'D';
        let mut iter = grid.rows_mut();
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
            let grid = Grid::new(2, 2, Foo(&count));
        }
        // Four allocations plus the Foo passed to Grid::new.
        assert_eq!(count.get(), 5);
    }
    
    #[test]
    fn new_with_bad_input() {
        #[derive(Copy, Clone)]
        struct Foo;
        assert!(Grid::new(3, 0, 1u8).is_none());
        assert!(Grid::new(0, 3, 1u8).is_none());
        assert!(Grid::new(3, 3, Foo).is_none());
    }
}
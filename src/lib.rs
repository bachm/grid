#![feature(alloc, core)]

extern crate alloc;
extern crate rustc_serialize;

use self::alloc::heap;
use self::rustc_serialize::{Decodable, Encodable, Decoder, Encoder};
use std::mem;
use std::ptr;
use std::iter::Iterator;
use std::marker::PhantomData;
use std::slice;
use std::ops::{Index, IndexMut};
use std::fmt;
use std::cmp::Ordering;

/// A dynamically allocated, two-dimensional array with fixed size.
/// Elements are stored in row-major order.
/// Width and height are guaranteed to be greater than zero.
pub struct DynArray2<T> {
    width: u16,
    height: u16,
    ptr: *mut T
}

impl<T: Clone> DynArray2<T> {
    /// Constructs an array from a width and height by cloning `element`.
    /// Will panic if width, height, or size of T are zero, or allocation fails.
    pub fn new(width: u16, height: u16, element: T) -> DynArray2<T> {
        let ptr = DynArray2::alloc(width, height, element).expect("DynArray2::new called with invalid input.");
        DynArray2 { width: width, height: height, ptr: ptr }
    }
    
    /// Constructs an array from a width and height by cloning `element`.
    /// Will return `None` if width, height, or size of T are zero, or allocation fails.
    pub fn new_checked(width: u16, height: u16, element: T) -> Option<DynArray2<T>> {
        DynArray2::alloc(width, height, element).map(|ptr| DynArray2 { width: width, height: height, ptr: ptr })
    }
    
    fn alloc(width: u16, height: u16, element: T) -> Option<*mut T> {
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
    /// Returns an iterator over the elements of the array.
    pub fn iter<'a>(&'a self) -> Iter<'a, T> {
        Iter { ptr: self.ptr, end: self.end(), marker: PhantomData }
    }
    
    /// Returns a mutable iterator over the elements of the array.
    pub fn iter_mut<'a>(&'a mut self) -> IterMut<'a, T> {
        IterMut { ptr: self.ptr, end: self.end_mut(), marker: PhantomData }
    }
    
    /// Returns an iterator over the rows of the array. Rows are represented as slice.
    pub fn rows<'a>(&'a self) -> Rows<'a, T> {
        Rows { ptr: self.ptr, end: self.end(), len: self.width as usize, marker: PhantomData }
    }
    
    /// Returns a mutable iterator over the rows of the array. Rows are represented as slice.
    pub fn rows_mut<'a>(&'a mut self) -> RowsMut<'a, T> {
        RowsMut { ptr: self.ptr, end: self.end_mut(), len: self.width as usize, marker: PhantomData }
    }
    
    /// Returns a reference to the element at the given position, or `None` if the position is invalid.
    pub fn get(&self, x: u16, y: u16) -> Option<&T> {
        if x < self.width && y < self.height {
            unsafe { self.ptr.offset(x as isize + y as isize * self.width as isize).as_ref() }
        } else {
            None
        }
    }
    
    /// Returns a mutable reference to the element at the given position, or `None` if the position is invalid.
    pub fn get_mut(&mut self, x: u16, y: u16) -> Option<&mut T> {
        if x < self.width && y < self.height {
            unsafe { self.ptr.offset(x as isize + y as isize * self.width as isize).as_mut() }
        } else {
            None
        }
    }
    
    /// Returns an iterator over the rows of a rectangular section of the array.
    /// Parts of the section that exceed the array bounds will be skipped.
    pub fn view<'a>(&'a self, x: u16, y: u16, width: u16, height: u16) -> View<'a, T> {
        let (slice_len, rows_left, offset) = self.view_helper(x, y, width, height);
        View {
            slice_ptr: unsafe { self.ptr.offset(offset) }, 
            slice_len: slice_len, 
            row_width: self.width as isize,
            rows_left: rows_left,
            marker: PhantomData
        }
    }
    
    /// Returns a mutable iterator over the rows of a rectangular section of the array.
    /// Parts of the section that exceed the array bounds will be skipped.
    pub fn view_mut<'a>(&'a mut self, x: u16, y: u16, width: u16, height: u16) -> ViewMut<'a, T> {
        let (slice_len, rows_left, offset) = self.view_helper(x, y, width, height);
        ViewMut {
            slice_ptr: unsafe { self.ptr.offset(offset) },
            slice_len: slice_len,
            row_width: self.width as isize,
            rows_left: rows_left,
            marker: PhantomData
        }
    }
    
    /// Returns the width of the array.
    pub fn width(&self) -> u16 {
        self.width
    }
    
    /// Returns the height of the array.
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
    
    #[inline]
    fn as_slice(&self) -> &[T] {
        unsafe { slice::from_raw_parts(self.ptr, self.width as usize * self.height as usize) }
    }
    
    #[inline]
    fn view_helper(&self, x: u16, y: u16, width: u16, height: u16) -> (usize, u16, isize) {
        let slice_len = if x + width < self.width { width } else { self.width - x } as usize;
        let rows_left = if slice_len == 0 { 0 } 
                   else if y + height < self.height { height } 
                   else { self.height - y };
        let offset = x as isize + y as isize * self.width as isize;
        (slice_len, rows_left, offset)
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

/// An iterator over the elements of the array.
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

/// A mutable iterator over the elements of the array.
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

/// An iterator over the rows of the array.
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

/// A mutable iterator over the rows of the array.
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


/// An iterator over the rows of a rectangular section of the array.
pub struct View<'a, T: 'a> {
    slice_ptr: *const T,
    slice_len: usize,
    row_width: isize,
    rows_left: u16,
    marker: PhantomData<&'a T>
}

impl<'a, T> Iterator for View<'a, T> {
    type Item = &'a [T];
    
    fn next(&mut self) -> Option<&'a [T]> {
        if self.rows_left > 0 { 
            self.rows_left -= 1;
            let slice = unsafe { slice::from_raw_parts_mut(self.slice_ptr as *mut T, self.slice_len) };
            self.slice_ptr = unsafe { self.slice_ptr.offset(self.row_width) };
            Some(slice)
        } else {
            None
        }
    }
}

/// A mutable iterator over the rows of a rectangular section of the array.
pub struct ViewMut<'a, T: 'a> {
    slice_ptr: *mut T,
    slice_len: usize,
    row_width: isize,
    rows_left: u16,
    marker: PhantomData<&'a mut T>
}

impl<'a, T> Iterator for ViewMut<'a, T> {
    type Item = &'a mut [T];
    
    fn next(&mut self) -> Option<&'a mut [T]> {
        if self.rows_left > 0 { 
            self.rows_left -= 1;
            let slice = unsafe { slice::from_raw_parts_mut(self.slice_ptr, self.slice_len) };
            self.slice_ptr = unsafe { self.slice_ptr.offset(self.row_width) };
            Some(slice)
        } else {
            None
        }
    }
}


const INDEX_ERROR_MSG: &'static str = "Attempted to index DynArray2 with invalid input."; 

impl<T> Index<(u16, u16)> for DynArray2<T> {
    type Output = T;
    
    fn index(&self, (x, y): (u16, u16)) -> &T {
        if x < self.width && y < self.height {
            unsafe { mem::transmute(self.ptr.offset(x as isize + y as isize * self.width as isize)) }
        } else {
            panic!(INDEX_ERROR_MSG)
        }
    }
}

impl<T> IndexMut<(u16, u16)> for DynArray2<T> {    
    fn index_mut(&mut self, (x, y): (u16, u16)) -> &mut T {
        if x < self.width && y < self.height {
            unsafe { mem::transmute(self.ptr.offset(x as isize + y as isize * self.width as isize)) }
        } else {
            panic!(INDEX_ERROR_MSG)
        }
    }
}

impl<T: Decodable> Decodable for DynArray2<T> {
    fn decode<D: Decoder>(d: &mut D) -> Result<DynArray2<T>, D::Error> {
        d.read_struct("DynArray2", 3, |d| {
            let width = try!(d.read_struct_field("width", 0, |d| d.read_u16()));
            let height = try!(d.read_struct_field("height", 1, |d| d.read_u16()));
            let ptr = try!(d.read_struct_field("data", 2, |d| {
                d.read_seq(|d, len| {
                    let bytes = len * mem::size_of::<T>();
                    let ptr = unsafe { heap::allocate(bytes, mem::align_of::<T>()) as *mut T };
                    for i in 0..len {
                        match d.read_seq_elt(i, |d| Decodable::decode(d)) {
                            Ok(e) => unsafe { ptr::write(ptr.offset(i as isize), e) },
                            Err(e) => {
                                for j in 0..i { 
                                     unsafe { ptr::read(ptr.offset(j as isize)); }
                                }
                                unsafe { heap::deallocate(ptr as *mut u8, bytes, mem::align_of::<T>()); }
                                return Err(e);
                            }
                        }
                    }
                    Ok(ptr)
                })
            }));
            Ok(DynArray2 { width: width, height: height, ptr: ptr })
        })
    }
}

impl<T: Encodable> Encodable for DynArray2<T> {
    fn encode<S: Encoder>(&self, s: &mut S) -> Result<(), S::Error> {
        s.emit_struct("DynArray2", 3, |s| {
            try!(s.emit_struct_field("width", 0, |s| {
                s.emit_u16(self.width)
            }));
            try!(s.emit_struct_field("height", 1, |s| {
                s.emit_u16(self.height)
            }));
            s.emit_struct_field("data", 2, |s| {
                let len = self.width as usize * self.height as usize;
                s.emit_seq(len, |s| {
                    for (i, element) in self.iter().enumerate() {
                        try!(s.emit_seq_elt(i, |s| element.encode(s)))
                    }
                    Ok(())
                })
            })
        })
    }
}

impl<T: fmt::Debug> fmt::Debug for DynArray2<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // This can probably be improved.
        try!(write!(f, "["));
        for row in self.rows().take(1) {
            let ref mut builder = f.debug_list();
            try!(row.iter().fold(builder, |b, e| b.entry(e)).finish())
        }
        for row in self.rows().skip(1) {
            try!(write!(f, ", "));
            let ref mut builder = f.debug_list();
            try!(row.iter().fold(builder, |b, e| b.entry(e)).finish())
        }
        write!(f, "]")
    }
}

impl<T: PartialEq> PartialEq for DynArray2<T> {
    fn eq(&self, rhs: &DynArray2<T>) -> bool {
        self.width.eq(&rhs.width) &&
        self.height.eq(&rhs.height) &&
        self.as_slice().eq(rhs.as_slice())
    }
}

impl<T: Eq> Eq for DynArray2<T> {}

impl<T: PartialOrd> PartialOrd for DynArray2<T> {
    fn partial_cmp(&self, rhs: &DynArray2<T>) -> Option<Ordering> {
        match self.width.partial_cmp(&rhs.width) {
            Some(Ordering::Equal) => match self.height.partial_cmp(&rhs.height) {
                Some(Ordering::Equal) => self.as_slice().partial_cmp(rhs.as_slice()),
                other => other
            },
            other => other
        }
    }
}

impl<T: Ord> Ord for DynArray2<T> {
   fn cmp(&self, rhs: &DynArray2<T>) -> Ordering {
        match self.width.cmp(&rhs.width) {
            Ordering::Equal => match self.height.cmp(&rhs.height) {
                Ordering::Equal => self.as_slice().cmp(rhs.as_slice()),
                other => other
            },
            other => other
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
    
    #[test]
    fn view() {
        // Array:
        // [0, 1, 2]
        // [3, 4, 5]
        // [6, 7, 8]
        let mut grid = DynArray2::new(3, 3, 0u8);
        for y in 0..3 {
            for x in 0..3 {
                grid[(x, y)] = (x + y * 3) as u8;
            }
        }
        
        // Target within the array:
        // [*, *, 2]
        // [*, *, 5]
        // [6, 7, 8]
        let mut iter = grid.view(0, 0, 2, 2);
        assert_eq!(iter.next().unwrap(), [0, 1]);
        assert_eq!(iter.next().unwrap(), [3, 4]);
        assert_eq!(iter.next(), None);
        
        // Target within the array #2:
        // [0, 1, 2]
        // [*, *, *]
        // [*, *, *]
        let mut iter = grid.view(0, 1, 3, 3);
        assert_eq!(iter.next().unwrap(), [3, 4, 5]);
        assert_eq!(iter.next().unwrap(), [6, 7, 8]);
        assert_eq!(iter.next(), None);
        
        // Target partially outside the array:
        // [0, 1, 2]
        // [3, *, *] *  *
        // [6, *, *] *  *
        //     *  *  *  *
        //     *  *  *  * 
        let mut iter = grid.view(1, 1, 4, 4);
        assert_eq!(iter.next().unwrap(), [4, 5]);
        assert_eq!(iter.next().unwrap(), [7, 8]);
        assert_eq!(iter.next(), None);
        
        // Target is the full array:
        // [*, *, *]
        // [*, *, *]
        // [*, *, *]
        let mut iter = grid.view(0, 0, 3, 3);
        assert_eq!(iter.next().unwrap(), [0, 1, 2]);
        assert_eq!(iter.next().unwrap(), [3, 4, 5]);
        assert_eq!(iter.next().unwrap(), [6, 7, 8]);
        assert_eq!(iter.next(), None);
        
        // Target x outside the array.
        // [0, 1, 2] *
        // [3, 4, 5]
        // [6, 7, 8]
        let mut iter = grid.view(3, 0, 1, 1);
        assert_eq!(iter.next(), None);
        
        // Target y outside the array.
        // [0, 1, 2]
        // [3, 4, 5]
        // [6, 7, 8]
        //     *
        let mut iter = grid.view(1, 3, 1, 1);
        assert_eq!(iter.next(), None);
        
        // Target x and y outside the array.
        // [0, 1, 2]
        // [3, 4, 5]
        // [6, 7, 8]
        //           *
        let mut iter = grid.view(3, 3, 1, 1);
        assert_eq!(iter.next(), None);
        
        // Target has no width:
        // [0, 1, 2]
        // [3, 4, 5]
        // [6, 7, 8]
        let mut iter = grid.view(1, 1, 0, 1);
        assert_eq!(iter.next(), None);
        
        // Target has no height:
        // [0, 1, 2]
        // [3, 4, 5]
        // [6, 7, 8]
        let mut iter = grid.view(1, 1, 1, 0);
        assert_eq!(iter.next(), None);
        
        // Target has no width and no height:
        // [0, 1, 2]
        // [3, 4, 5]
        // [6, 7, 8]
        let mut iter = grid.view(1, 1, 0, 0);
        assert_eq!(iter.next(), None);
    }
}
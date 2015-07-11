#![feature(alloc, heap_api, ptr_as_ref, unique)]

extern crate alloc;
extern crate rustc_serialize;

use self::alloc::heap;
use self::rustc_serialize::{Decodable, Encodable, Decoder, Encoder};
use std::mem;
use std::ptr::{self, Unique};
use std::iter::{self, Iterator};
use std::marker::PhantomData;
use std::slice;
use std::ops::{Index, IndexMut};
use std::fmt;
use std::cmp::{self, Ordering};

/// A 2d array whose size is determined at runtime and fixed at construction.
/// Elements are stored in row-major order.
pub struct Array2<T> {
    ptr: Unique<T>,
    width: u32,
    height: u32
}

impl<T: Default> Array2<T> {
    /// Constructs an `Array2<T>` from `width` and `height` by filling it with the default value of `T`.
    pub fn from_default(width: u32, height: u32) -> Array2<T> {
        Array2::from_fn(width, height, || T::default())
    }
}

impl<T: Clone> Array2<T> {
    /// Constructs an `Array2<T>` from `width` and `height` by cloning `element`.
    pub fn from_elem(width: u32, height: u32, element: T) -> Array2<T> {
        Array2::from_fn(width, height, || element.clone())
    }
}

impl<T> Array2<T> {
    /// Constructs an `Array2<T>` from `width` and `height` by repeatedly calling `f`.
    pub fn from_fn<F: FnMut() -> T>(width: u32, height: u32, mut f: F) -> Array2<T> {
        let allocation_required = mem::size_of::<T>() > 0 && width > 0 && height > 0;
        let ptr = if allocation_required {
            let count = width as usize * height as usize;
            let ptr = unsafe { heap::allocate(count * mem::size_of::<T>(), mem::align_of::<T>()) } as *mut T;
            if ptr.is_null() { ::std::process::exit(-9999); }
            for offset in 0..count as isize {
                unsafe { ptr::write(ptr.offset(offset), f()) }
            }
            unsafe { Unique::new(ptr) }
        } else {
            unsafe { Unique::new(heap::EMPTY as *mut T) }
        };
        Array2 { ptr: ptr, width: width, height: height }
    }
    
    /// Constructs an `Array2<T>` from `width` and `height` by repeatedly calling `f` and passing
    /// the x and y coordinates of each element to it.
    pub fn from_fn_with_points<F: FnMut(u32, u32) -> T>(width: u32, height: u32, mut f: F) -> Array2<T> {
        let mut iter = (0..height).flat_map(|y| iter::repeat(y).zip((0..width)));
        Array2::from_fn(width, height, || {
            let (x, y) = iter.next().unwrap();
            f(x, y)
        })
    }
    
    /// Returns a reference to the element at the given position, or `None` if the position is invalid.
    pub fn get(&self, x: u32, y: u32) -> Option<&T> {
        if x < self.width && y < self.height {
            unsafe { self.ptr.offset(x as isize + y as isize * self.width as isize).as_ref() }
        } else {
            None
        }
    }
    
    /// Returns a mutable reference to the element at the given position, or `None` if the position is invalid.
    pub fn get_mut(&mut self, x: u32, y: u32) -> Option<&mut T> {
        if x < self.width && y < self.height {
            unsafe { self.ptr.offset(x as isize + y as isize * self.width as isize).as_mut() }
        } else {
            None
        }
    }

    /// Returns an iterator over the elements of the array.
    pub fn iter<'a>(&'a self) -> Items<'a, T> {
        Items { ptr: *self.ptr, end: self.end(), marker: PhantomData }
    }
    
    /// Returns a mutable iterator over the elements of the array.
    pub fn iter_mut<'a>(&'a mut self) -> ItemsMut<'a, T> {
        ItemsMut { ptr: *self.ptr, end: self.end(), marker: PhantomData }
    }
    
    /// Returns an iterator over the rows of the array. Rows are represented as slice.
    pub fn rows<'a>(&'a self) -> Rows<'a, T> {
        Rows { ptr: *self.ptr, end: self.end(), len: self.width as usize, marker: PhantomData }
    }
    
    /// Returns a mutable iterator over the rows of the array. Rows are represented as slice.
    pub fn rows_mut<'a>(&'a mut self) -> RowsMut<'a, T> {
        RowsMut { ptr: *self.ptr, end: self.end(), len: self.width as usize, marker: PhantomData }
    }
    
    /// Returns an iterator over the rows of a rectangular section of the array.
    /// Parts of the section that exceed the array bounds will be skipped.
    pub fn view<'a>(&'a self, x: u32, y: u32, width: u32, height: u32) -> View<'a, T> {
        let (ptr, end, slice_len, array_width) = self.view_components(x, y, width, height);
        View {
            ptr: ptr,
            end: end,
            slice_len: slice_len,
            array_width: array_width,
            marker: PhantomData
        }
    }
    
    /// Returns a mutable iterator over the rows of a rectangular section of the array.
    /// Parts of the section that exceed the array bounds will be skipped.
    pub fn view_mut<'a>(&'a mut self, x: u32, y: u32, width: u32, height: u32) -> ViewMut<'a, T> {
        let (ptr, end, slice_len, array_width) = self.view_components(x, y, width, height);
        ViewMut {
            ptr: ptr,
            end: end,
            slice_len: slice_len,
            array_width: array_width,
            marker: PhantomData
        }
    }
    
    /// Returns the width of the array.
    pub fn width(&self) -> u32 {
        self.width
    }
    
    /// Returns the height of the array.
    pub fn height(&self) -> u32 {
        self.height
    }
    
    /// Returns a slice over all elements in the array. 
    pub fn as_slice(&self) -> &[T] {
        unsafe { slice::from_raw_parts(*self.ptr, self.width as usize * self.height as usize) }
    }
    
    /// Returns a mutable slice over all elements in the array.
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        unsafe { slice::from_raw_parts_mut(*self.ptr, self.width as usize * self.height as usize) }
    }

    #[inline]
    fn end(&self) -> *mut T {
        if mem::size_of::<T>() > 0 {
            unsafe { self.ptr.offset(self.width as isize * self.height as isize) }
        } else {
            (*self.ptr as usize + self.width as usize * self.height as usize) as *mut T
        }
    }
    
    #[inline]
    fn view_components(&self, x: u32, y: u32, mut width: u32, mut height: u32) -> (*mut T, *mut T, usize, isize) {
        let input_is_valid = x < self.width && y < self.height && width > 0 && height > 0;
        let (ptr, end) = if input_is_valid {
            width = cmp::min(width, self.width - x);
            height = cmp::min(height, self.height - y);
            if mem::size_of::<T>() > 0 {
                let ptr_offset = x as isize + y as isize * self.width as isize;
                let ptr = unsafe { self.ptr.offset(ptr_offset) };
                let end_offset = height as isize * self.width as isize;
                let end = unsafe { ptr.offset(end_offset) };
                (ptr, end)
            } else {
                (*self.ptr, (*self.ptr as usize + height as usize) as *mut T)
            }
        } else {
            (*self.ptr, *self.ptr)
        };
        (ptr, end, width as usize, self.width as isize)
    }
}

impl<T> Drop for Array2<T> {
    fn drop(&mut self) {
        let deallocation_required = *self.ptr != heap::EMPTY as *mut T;
        if deallocation_required {
            for e in self.iter() { 
                unsafe { ptr::read(e); }
            }
            let bytes = self.width as usize * self.height as usize * mem::size_of::<T>();
            unsafe { heap::deallocate(*self.ptr as *mut u8, bytes, mem::align_of::<T>()); }
        }
    }
}
    
/// An iterator over the elements of the array.
pub struct Items<'a, T: 'a> {
    ptr: *const T,
    end: *const T,
    marker: PhantomData<&'a T>
}

impl<'a, T> Iterator for Items<'a, T> {
    type Item = &'a T;
    
    fn next(&mut self) -> Option<Self::Item> {
        if self.ptr < self.end {
            let r = unsafe { &*self.ptr };
            if mem::size_of::<T>() > 0 {
                self.ptr = unsafe { self.ptr.offset(1) };
            } else {
                self.ptr = (self.ptr as usize + 1) as *const T;
            }
            Some(r)
        } else {
            None
        }
    }
}

/// A mutable iterator over the elements of the array.
pub struct ItemsMut<'a, T: 'a> {
    ptr: *mut T,
    end: *mut T,
    marker: PhantomData<&'a mut T>
}

impl<'a, T> Iterator for ItemsMut<'a, T> {
    type Item = &'a mut T;
    
    fn next(&mut self) -> Option<Self::Item> {
        if self.ptr < self.end {
            let r = unsafe { &mut *self.ptr };
            if mem::size_of::<T>() > 0 {
                self.ptr = unsafe { self.ptr.offset(1) };
            } else {
                self.ptr = (self.ptr as usize + 1) as *mut T;
            }
            Some(r)
        } else {
            None
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
    
    fn next(&mut self) -> Option<Self::Item> {
        if self.ptr < self.end {
            let slice = unsafe { slice::from_raw_parts(self.ptr, self.len) };
            if mem::size_of::<T>() > 0 {
                self.ptr = unsafe { self.ptr.offset(self.len as isize) };
            } else {
                self.ptr = (self.ptr as usize + self.len) as *mut T;
            }
            Some(slice)
        } else {
            None
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
    
    fn next(&mut self) -> Option<Self::Item> {
        if self.ptr < self.end {
            let slice = unsafe { slice::from_raw_parts_mut(self.ptr, self.len) };
            if mem::size_of::<T>() > 0 {
                self.ptr = unsafe { self.ptr.offset(self.len as isize) };
            } else {
                self.ptr = (self.ptr as usize + self.len) as *mut T;
            }
            Some(slice)
        } else {
            None
        }
    }
}


/// An iterator over the rows of a rectangular section of the array.
pub struct View<'a, T: 'a> {
    ptr: *const T,
    end: *const T,
    slice_len: usize,
    array_width: isize,
    marker: PhantomData<&'a T>
}

impl<'a, T> Iterator for View<'a, T> {
    type Item = &'a [T];
    
    fn next(&mut self) -> Option<Self::Item> {
        if self.ptr < self.end {
            let slice = unsafe { slice::from_raw_parts(self.ptr, self.slice_len) };
            if mem::size_of::<T>() > 0 {
                self.ptr = unsafe { self.ptr.offset(self.array_width) };
            } else {
                self.ptr = (self.ptr as usize + 1) as *const T;
            }
            Some(slice)
        } else {
            None
        }
    }
}

/// A mutable iterator over the rows of a rectangular section of the array.
pub struct ViewMut<'a, T: 'a> {
    ptr: *mut T,
    end: *mut T,
    slice_len: usize,
    array_width: isize,
    marker: PhantomData<&'a mut T>
}

impl<'a, T> Iterator for ViewMut<'a, T> {
    type Item = &'a mut [T];
    
    fn next(&mut self) -> Option<Self::Item> {
        if self.ptr < self.end { 
            let slice = unsafe { slice::from_raw_parts_mut(self.ptr, self.slice_len) };
            if mem::size_of::<T>() > 0 {
                self.ptr = unsafe { self.ptr.offset(self.array_width) };
            } else {
                self.ptr = (self.ptr as usize + 1) as *mut T;
            }
            Some(slice)
        } else {
            None
        }
    }
}

pub trait Point2 {
    fn x(&self) -> u32;
    fn y(&self) -> u32;
}

impl Point2 for (u32, u32) {
    fn x(&self) -> u32 { self.0 }
    fn y(&self) -> u32 { self.1 }
}

impl Point2 for [u32; 2] {
    fn x(&self) -> u32 { self[0] }
    fn y(&self) -> u32 { self[1] }
}

impl<P: Point2, T> Index<P> for Array2<T> {
    type Output = T;
    
    fn index(&self, point: P) -> &Self::Output {
        let x = point.x();
        let y = point.y();
        if x < self.width && y < self.height {
            unsafe { &*self.ptr.offset(x as isize + y as isize * self.width as isize) }
        } else {
            panic!("Array2 index out of bounds")
        }
    }
}

impl<P: Point2, T> IndexMut<P> for Array2<T> {
    fn index_mut(&mut self, point: P) -> &mut Self::Output {
        let x = point.x();
        let y = point.y();
        if x < self.width && y < self.height {
            unsafe { &mut *self.ptr.offset(x as isize + y as isize * self.width as isize) }
        } else {
            panic!("Array2 index out of bounds")
        }
    }
}

impl<T: Decodable> Decodable for Array2<T> {
    fn decode<D: Decoder>(d: &mut D) -> Result<Array2<T>, D::Error> {
        d.read_struct("Array2", 3, |d| {
            let width = try!(d.read_struct_field("width", 0, |d| d.read_u32()));
            let height = try!(d.read_struct_field("height", 1, |d| d.read_u32()));
            let ptr = try!(d.read_struct_field("data", 2, |d| {
                d.read_seq(|d, len| {
                    let bytes = len * mem::size_of::<T>();
                    let ptr = unsafe { Unique::new(heap::allocate(bytes, mem::align_of::<T>()) as *mut T) };
                    for i in 0..len {
                        match d.read_seq_elt(i, |d| Decodable::decode(d)) {
                            Ok(e) => unsafe { ptr::write(ptr.offset(i as isize), e) },
                            Err(e) => {
                                for j in 0..i { 
                                     unsafe { ptr::read(ptr.offset(j as isize)); }
                                }
                                unsafe { heap::deallocate(*ptr as *mut u8, bytes, mem::align_of::<T>()); }
                                return Err(e);
                            }
                        }
                    }
                    Ok(ptr)
                })
            }));
            Ok(Array2 { width: width, height: height, ptr: ptr })
        })
    }
}

impl<T: Encodable> Encodable for Array2<T> {
    fn encode<S: Encoder>(&self, s: &mut S) -> Result<(), S::Error> {
        s.emit_struct("Array2", 3, |s| {
            try!(s.emit_struct_field("width", 0, |s| {
                s.emit_u32(self.width)
            }));
            try!(s.emit_struct_field("height", 1, |s| {
                s.emit_u32(self.height)
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

impl<T: fmt::Debug> fmt::Debug for Array2<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.rows().fold(&mut f.debug_list(), |b, e| b.entry(&e)).finish()
    }
}

impl<T: PartialEq> PartialEq for Array2<T> {
    fn eq(&self, rhs: &Array2<T>) -> bool {
        self.width.eq(&rhs.width) &&
        self.height.eq(&rhs.height) &&
        self.as_slice().eq(rhs.as_slice())
    }
}

impl<T: Eq> Eq for Array2<T> {}

impl<T: PartialOrd> PartialOrd for Array2<T> {
    fn partial_cmp(&self, rhs: &Array2<T>) -> Option<Ordering> {
        match self.width.partial_cmp(&rhs.width) {
            Some(Ordering::Equal) => match self.height.partial_cmp(&rhs.height) {
                Some(Ordering::Equal) => self.as_slice().partial_cmp(rhs.as_slice()),
                other => other
            },
            other => other
        }
    }
}

impl<T: Ord> Ord for Array2<T> {
   fn cmp(&self, rhs: &Array2<T>) -> Ordering {
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
mod test {
    use super::Array2;
    
    #[derive(Copy, Clone, PartialEq, Debug)]
    struct ZeroSizedType;
   
    fn standard_array() -> Array2<u8> {
        let mut count = 0;
        Array2::from_fn(2, 2, || {
            count += 1;
            count - 1
        })
    }
    
    fn zero_width_array() -> Array2<u8> { Array2::from_elem(0, 2, 0u8) }
    
    fn zero_height_array() -> Array2<u8> { Array2::from_elem(2, 0, 0u8) }
    
    fn zst_array() -> Array2<ZeroSizedType> { Array2::from_elem(2, 2, ZeroSizedType) }
    
    #[test] 
    #[allow(unused_variables)]
    fn construction() {
        let array = standard_array();
        let array = zero_width_array();
        let array = zero_height_array();
        let array = zst_array;
    }
    
    #[test]
    fn get() {
        let array = standard_array();
        assert_eq!(array.get(1, 0), Some(&1));
        assert_eq!(array.get(0, 1), Some(&2));
        assert_eq!(array.get(1, 1), Some(&3));
        assert_eq!(array.get(2, 0), None);
        assert_eq!(array.get(0, 2), None);

        assert_eq!(zero_width_array().get(1, 1), None);
        assert_eq!(zero_height_array().get(1, 1), None);
        
        let array = zst_array();
        assert_eq!(array.get(0, 0), Some(&ZeroSizedType));
        assert_eq!(array.get(1, 1), Some(&ZeroSizedType));
        assert_eq!(array.get(2, 2), None);
    }
    
    #[test]
    fn get_mut() {
        let mut array = standard_array();
        *array.get_mut(1, 0).unwrap() = 10;
        assert_eq!(array.get_mut(1, 0), Some(&mut 10));
        assert_eq!(array.get_mut(0, 1), Some(&mut 2));
        assert_eq!(array.get_mut(1, 1), Some(&mut 3));
        assert_eq!(array.get_mut(2, 0), None);
        assert_eq!(array.get_mut(0, 2), None);

        assert_eq!(zero_width_array().get_mut(1, 1), None);
        assert_eq!(zero_height_array().get_mut(1, 1), None);
        
        let mut array = zst_array();
        *array.get_mut(1, 1).unwrap() = ZeroSizedType;
        assert_eq!(array.get_mut(1, 1), Some(&mut ZeroSizedType));
    }
    
    #[test]
    fn iter() {
        let array = standard_array();
        let mut iter = array.iter();
        assert_eq!(iter.next(), Some(&0));
        assert_eq!(iter.next(), Some(&1));
        assert_eq!(iter.next(), Some(&2));
        assert_eq!(iter.next(), Some(&3));
        assert_eq!(iter.next(), None);
        
        for array in [zero_width_array(), zero_height_array()].iter() {
            assert_eq!(array.iter().next(), None);
        }
        
        let array = zst_array();
        let mut iter = array.iter();
        for _ in 0..4 {
            assert_eq!(*iter.next().unwrap(), ZeroSizedType);
        }
        assert_eq!(iter.next(), None);
    }
        
    #[test]
    fn iter_mut() {
        let mut array = standard_array();
        let mut iter = array.iter_mut();
        assert_eq!(iter.next(), Some(&mut 0));
        assert_eq!(iter.next(), Some(&mut 1));
        assert_eq!(iter.next(), Some(&mut 2));
        assert_eq!(iter.next(), Some(&mut 3));
        assert_eq!(iter.next(), None);
        
        for array in [zero_width_array(), zero_height_array()].iter_mut() {
            assert_eq!(array.iter_mut().next(), None);
        }
        
        let mut array = zst_array();
        let mut iter = array.iter_mut();
        for _ in 0..4 {
            assert_eq!(*iter.next().unwrap(), ZeroSizedType);
        }
        assert_eq!(iter.next(), None);
    }
    
    #[test]
    fn rows() {
        let array = standard_array();
        let mut iter = array.rows();
        assert_eq!(iter.next(), Some(&[0, 1][..]));
        assert_eq!(iter.next(), Some(&[2, 3][..]));
        assert_eq!(iter.next(), None);
        
        for array in [zero_width_array(), zero_height_array()].iter_mut() {
            assert_eq!(array.rows().next(), None);
        }
        
        let array = zst_array();
        let mut iter = array.rows();
        assert_eq!(iter.next(), Some(&[ZeroSizedType, ZeroSizedType][..]));
        assert_eq!(iter.next(), Some(&[ZeroSizedType, ZeroSizedType][..]));
        assert_eq!(iter.next(), None);
    }
    
    #[test]
    fn rows_mut() {
        let mut array = standard_array();
        let mut iter = array.rows_mut();
        assert_eq!(iter.next(), Some(&mut [0, 1][..]));
        assert_eq!(iter.next(), Some(&mut [2, 3][..]));
        assert_eq!(iter.next(), None);
        
        for array in [zero_width_array(), zero_height_array()].iter_mut() {
            assert_eq!(array.rows_mut().next(), None);
        }
        
        let mut array = zst_array();
        let mut iter = array.rows_mut();
        assert_eq!(iter.next(), Some(&mut [ZeroSizedType, ZeroSizedType][..]));
        assert_eq!(iter.next(), Some(&mut [ZeroSizedType, ZeroSizedType][..]));
        assert_eq!(iter.next(), None);
    }
    
    #[test]
    fn index() {
        let array = standard_array();
        assert_eq!(array[(1, 1)], 3u8);
    }
    
    #[test]
    fn index_mut() {
        let mut array = standard_array();
        array[(1, 0)] = 23;
        assert_eq!(array[(1, 1)], 3u8);
    }
    
    #[test]
    #[should_panic]
    #[allow(unused_variables)]
    fn index_panic() {
        let array = standard_array();
        let x = array[(3, 1)];
    }
    
    #[test]
    #[should_panic]
    fn index_mut_panic() {
        let mut array = standard_array();
        array[(3, 1)] += 1;
    }
    
    #[test]
    fn slicing() {
        let array = standard_array();
        assert_eq!(&array.as_slice()[3], &3);
        assert_eq!(&array.as_slice()[..], &[0, 1, 2, 3][..]);
        
        let array = zero_width_array();
        assert_eq!(&array.as_slice()[..], &[]);
        
        let array = zero_height_array();
        assert_eq!(&array.as_slice()[..], &[]);
        
        let array = zst_array();
        assert_eq!(&array.as_slice()[1..3], &[ZeroSizedType, ZeroSizedType][..]);
        
    }
    
    #[test]
    fn view() {
        // Array:
        // [0, 1]
        // [2, 3]
        let array = standard_array();
        
        // Target is full array
        // [*, *]
        // [*, *]
        let mut iter = array.view(0, 0, 2, 2);
        assert_eq!(iter.next(), Some(&[0, 1][..]));
        assert_eq!(iter.next(), Some(&[2, 3][..]));
        assert_eq!(iter.next(), None);
        
        // Target is a subsection
        // [0, *]
        // [2, *]
        let mut iter = array.view(1, 0, 1, 2);
        assert_eq!(iter.next(), Some(&[1][..]));
        assert_eq!(iter.next(), Some(&[3][..]));
        assert_eq!(iter.next(), None);
        
        // Target partially outside the array #1:
        // [0, 1]
        // [*, *] *
        //  *  *  *
        let mut iter = array.view(0, 1, 3, 2);
        assert_eq!(iter.next(), Some(&[2, 3][..]));
        assert_eq!(iter.next(), None);
        
        // Target partially outside the array #2:
        // [0, *] *
        // [2, *] *
        //     *  * 
        let mut iter = array.view(1, 0, 2, 3);
        assert_eq!(iter.next(), Some(&[1][..]));
        assert_eq!(iter.next(), Some(&[3][..]));
        assert_eq!(iter.next(), None);
        
        // Target x outside the array.
        // [0, 1] *
        // [2, 3]
        let mut iter = array.view(2, 0, 1, 1);
        assert_eq!(iter.next(), None);
        
        // Target y outside the array.
        // [0, 1]
        // [2, 3]
        //  *
        let mut iter = array.view(0, 2, 1, 1);
        assert_eq!(iter.next(), None);
        
        // Target x and y outside the array.
        // [0, 1]
        // [2, 3]
        //       *
        let mut iter = array.view(2, 2, 1, 1);
        assert_eq!(iter.next(), None);
        
        // Target has no width:
        // [0, 1]
        // [2, 3]
        let mut iter = array.view(1, 1, 0, 1);
        assert_eq!(iter.next(), None);
        
        // Target has no height:
        // [0, 1]
        // [2, 3]
        let mut iter = array.view(1, 1, 1, 0);
        assert_eq!(iter.next(), None);
        
        // Target has no width and no height:
        // [0, 1]
        // [2, 3]
        let mut iter = array.view(1, 1, 0, 0);
        assert_eq!(iter.next(), None);
        
        let array = zero_width_array();
        let mut iter = array.view(0, 0, 1, 1);
        assert_eq!(iter.next(), None);
        
        let array = zero_height_array();
        let mut iter = array.view(0, 0, 1, 1);
        assert_eq!(iter.next(), None);
        
        let array = zst_array();
        let mut iter = array.view(0, 0, 3, 3);
        assert_eq!(iter.next(), Some(&[ZeroSizedType, ZeroSizedType][..]));
        assert_eq!(iter.next(), Some(&[ZeroSizedType, ZeroSizedType][..]));
        assert_eq!(iter.next(), None);
    }
    
    #[test]
    fn view_mut() {
        let mut array = Array2::from_elem(4, 4, 0u8);
        for row in array.view_mut(2, 0, 2, 4) {
            for element in row.iter_mut() {
                *element += 1;
            }
        }
        for row in array.view_mut(0, 2, 4, 2) {
            for element in row.iter_mut() {
                *element += 1;
            }
        }
        let mut iter = array.view_mut(0, 0, 4, 4);
        assert_eq!(iter.next(), Some(&mut [0, 0, 1, 1][..]));
        assert_eq!(iter.next(), Some(&mut [0, 0, 1, 1][..]));
        assert_eq!(iter.next(), Some(&mut [1, 1, 2, 2][..]));
        assert_eq!(iter.next(), Some(&mut [1, 1, 2, 2][..]));
        assert_eq!(iter.next(), None);
    
        // Array:
        // [0, 1]
        // [2, 3]
        let mut array = standard_array();
        
        // Target is full array
        // [*, *]
        // [*, *]
        {
            let mut iter = array.view_mut(0, 0, 2, 2);
            assert_eq!(iter.next(), Some(&mut [0, 1][..]));
            assert_eq!(iter.next(), Some(&mut [2, 3][..]));
            assert_eq!(iter.next(), None);
        } 
        // Target is a subsection
        // [0, *]
        // [2, *]
        {
            let mut iter = array.view_mut(1, 0, 1, 2);
            assert_eq!(iter.next(), Some(&mut [1][..]));
            assert_eq!(iter.next(), Some(&mut [3][..]));
            assert_eq!(iter.next(), None);
        }
        // Target partially outside the array #1:
        // [0, 1]
        // [*, *] *
        //  *  *  *
        {
            let mut iter = array.view_mut(0, 1, 3, 2);
            assert_eq!(iter.next(), Some(&mut [2, 3][..]));
            assert_eq!(iter.next(), None);
        }
        // Target partially outside the array #2:
        // [0, *] *
        // [2, *] *
        //     *  *
        { 
            let mut iter = array.view_mut(1, 0, 2, 3);
            assert_eq!(iter.next(), Some(&mut [1][..]));
            assert_eq!(iter.next(), Some(&mut [3][..]));
            assert_eq!(iter.next(), None);
        }
        // Target x outside the array.
        // [0, 1] *
        // [2, 3]
        {
            let mut iter = array.view_mut(2, 0, 1, 1);
            assert_eq!(iter.next(), None);
        }
        // Target y outside the array.
        // [0, 1]
        // [2, 3]
        //  *
        {
            let mut iter = array.view_mut(0, 2, 1, 1);
            assert_eq!(iter.next(), None);
        }
        // Target x and y outside the array.
        // [0, 1]
        // [2, 3]
        //       *
        {
            let mut iter = array.view_mut(2, 2, 1, 1);
            assert_eq!(iter.next(), None);
        }
        // Target has no width:
        // [0, 1]
        // [2, 3]
        {
            let mut iter = array.view_mut(1, 1, 0, 1);
            assert_eq!(iter.next(), None);
        }
        // Target has no height:
        // [0, 1]
        // [2, 3]
        {
            let mut iter = array.view_mut(1, 1, 1, 0);
            assert_eq!(iter.next(), None);
        }
        // Target has no width and no height:
        // [0, 1]
        // [2, 3]
        {
            let mut iter = array.view_mut(1, 1, 0, 0);
            assert_eq!(iter.next(), None);
        }
        
        let mut array = zero_width_array();
        let mut iter = array.view_mut(0, 0, 1, 1);
        assert_eq!(iter.next(), None);
        
        let mut array = zero_height_array();
        let mut iter = array.view_mut(0, 0, 1, 1);
        assert_eq!(iter.next(), None);
        
        let mut array = zst_array();
        let mut iter = array.view_mut(0, 0, 3, 3);
        assert_eq!(iter.next(), Some(&mut [ZeroSizedType, ZeroSizedType][..]));
        assert_eq!(iter.next(), Some(&mut [ZeroSizedType, ZeroSizedType][..]));
        assert_eq!(iter.next(), None);
    }
}

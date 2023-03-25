use crate::scalar::{Scalar, ScalarType};
#[cfg(target_arch = "spirv")]
use core::arch::asm;
#[cfg(not(target_arch = "spirv"))]
use core::marker::PhantomData;
use core::ops::Index;
#[cfg(target_arch = "spirv")]
use spirv_std::arch::IndexUnchecked;

#[cfg(target_arch = "spirv")]
fn debug_index_out_of_bounds(index: usize, len: usize) {
    unsafe {
        spirv_std::macros::debug_printfln!(
            "index out of bounds: the len is %u but the index is %u",
            len as u32,
            index as u32,
        );
    }
}

pub trait UnsafeIndex<Idx> {
    type Output;
    unsafe fn unsafe_index(&self, index: Idx) -> &Self::Output;
    unsafe fn unsafe_index_mut(&self, index: Idx) -> &mut Self::Output;
}

#[cfg(target_arch = "spirv")]
trait IndexUncheckedMutExt<T> {
    unsafe fn index_unchecked_mut_ext(&self, index: usize) -> &mut T;
}

/*
#[cfg(target_arch = "spirv")]
impl<T> IndexUncheckedMutExt<T> for [T] {
    unsafe fn index_unchecked_mut_ext(&self, index: usize) -> &mut T {
        // https://docs.rs/spirv-std/0.5.0/src/spirv_std/arch.rs.html#237-248
        unsafe {
            asm!(
                "%slice_ptr = OpLoad _ {slice_ptr_ptr}",
                "%data_ptr = OpCompositeExtract _ %slice_ptr 0",
                "%val_ptr = OpAccessChain _ %data_ptr {index}",
                "OpReturnValue %val_ptr",
                slice_ptr_ptr = in(reg) &self,
                index = in(reg) index,
                options(noreturn)
            )
        }
    }
}
*/

#[cfg(target_arch = "spirv")]
impl<T, const N: usize> IndexUncheckedMutExt<T> for [T; N] {
    unsafe fn index_unchecked_mut_ext(&self, index: usize) -> &mut T {
        // https://github.com/EmbarkStudios/rust-gpu/blob/main/crates/spirv-std/src/arch.rs
        unsafe {
            asm!(
                "%val_ptr = OpAccessChain _ {array_ptr} {index}",
                "OpReturnValue %val_ptr",
                array_ptr = in(reg) self,
                index = in(reg) index,
                options(noreturn)
            )
        }
    }
}

mod sealed {
    pub trait Sealed {}
}
use sealed::Sealed;

pub trait DataBase: Sealed {
    type Elem: Scalar;
    fn len(&self) -> usize;
}

pub trait Data: DataBase + Index<usize, Output = Self::Elem> {}
pub trait UnsafeData: DataBase + UnsafeIndex<usize, Output = Self::Elem> {}

#[allow(unused)]
pub struct SliceRepr<'a, T> {
    #[cfg(not(target_arch = "spirv"))]
    inner: &'a [T],
    #[cfg(target_arch = "spirv")]
    inner: &'a [T; 1],
    #[cfg(target_arch = "spirv")]
    offset: usize,
    #[cfg(target_arch = "spirv")]
    len: usize,
}

impl<T> Sealed for SliceRepr<'_, T> {}

impl<T: Scalar> DataBase for SliceRepr<'_, T> {
    type Elem = T;
    #[cfg(not(target_arch = "spirv"))]
    fn len(&self) -> usize {
        self.inner.len()
    }
    #[cfg(target_arch = "spirv")]
    fn len(&self) -> usize {
        self.len
    }
}

impl<T: Scalar> Index<usize> for SliceRepr<'_, T> {
    type Output = T;
    #[cfg(not(target_arch = "spirv"))]
    fn index(&self, index: usize) -> &Self::Output {
        self.inner.index(index)
    }
    #[cfg(target_arch = "spirv")]
    fn index(&self, index: usize) -> &Self::Output {
        if index < self.len {
            unsafe { self.inner.index_unchecked(self.offset + index) }
        } else {
            debug_index_out_of_bounds(index, self.len);
            panic!();
        }
    }
}

impl<T: Scalar> Data for SliceRepr<'_, T> {}

pub struct UnsafeSliceRepr<'a, T> {
    #[cfg(not(target_arch = "spirv"))]
    ptr: *mut T,
    #[cfg(target_arch = "spirv")]
    #[allow(unused)]
    inner: &'a mut [T; 1],
    #[cfg(target_arch = "spirv")]
    #[allow(unused)]
    offset: usize,
    len: usize,
    #[cfg(not(target_arch = "spirv"))]
    _m: PhantomData<&'a ()>,
}

impl<T> Sealed for UnsafeSliceRepr<'_, T> {}

impl<T: Scalar> DataBase for UnsafeSliceRepr<'_, T> {
    type Elem = T;
    fn len(&self) -> usize {
        self.len
    }
}

impl<T: Scalar> UnsafeIndex<usize> for UnsafeSliceRepr<'_, T> {
    type Output = T;
    #[cfg(not(target_arch = "spirv"))]
    unsafe fn unsafe_index(&self, index: usize) -> &Self::Output {
        if index < self.len {
            unsafe { &*self.ptr.add(index) }
        } else {
            panic!(
                "index out of bounds: the len is {index} but the index is {len}",
                len = self.len
            );
        }
    }
    #[cfg(target_arch = "spirv")]
    unsafe fn unsafe_index(&self, index: usize) -> &Self::Output {
        if index < self.len {
            unsafe { self.inner.index_unchecked(self.offset + index) }
        } else {
            debug_index_out_of_bounds(index, self.len);
            panic!();
        }
    }
    #[cfg(not(target_arch = "spirv"))]
    unsafe fn unsafe_index_mut(&self, index: usize) -> &mut Self::Output {
        if index < self.len {
            unsafe { &mut *self.ptr.add(index) }
        } else {
            panic!(
                "index out of bounds: the len is {index} but the index is {len}",
                len = self.len
            );
        }
    }
    #[cfg(target_arch = "spirv")]
    unsafe fn unsafe_index_mut(&self, index: usize) -> &mut Self::Output {
        if index < self.len {
            unsafe { self.inner.index_unchecked_mut_ext(self.offset + index) }
        } else {
            debug_index_out_of_bounds(index, self.len);
            panic!();
        }
    }
}

impl<T: Scalar> UnsafeData for UnsafeSliceRepr<'_, T> {}

pub struct BufferBase<S> {
    data: S,
}

pub type Slice<'a, T> = BufferBase<SliceRepr<'a, T>>;
pub type UnsafeSlice<'a, T> = BufferBase<UnsafeSliceRepr<'a, T>>;

impl<S: DataBase> BufferBase<S> {
    pub fn len(&self) -> usize {
        self.data.len()
    }
    pub fn scalar_type(&self) -> ScalarType {
        S::Elem::scalar_type()
    }
}

impl<S: Data> Index<usize> for BufferBase<S> {
    type Output = S::Elem;
    fn index(&self, index: usize) -> &Self::Output {
        self.data.index(index)
    }
}

impl<S: UnsafeData> UnsafeIndex<usize> for BufferBase<S> {
    type Output = S::Elem;
    unsafe fn unsafe_index(&self, index: usize) -> &Self::Output {
        unsafe { self.data.unsafe_index(index) }
    }
    unsafe fn unsafe_index_mut(&self, index: usize) -> &mut Self::Output {
        unsafe { self.data.unsafe_index_mut(index) }
    }
}

impl<'a, T: Scalar> Slice<'a, T> {
    #[cfg(target_arch = "spirv")]
    pub unsafe fn from_raw_parts(inner: &'a [T; 1], offset: usize, len: usize) -> Self {
        let data = SliceRepr { inner, offset, len };
        Self { data }
    }
}

impl<'a, T: Scalar> UnsafeSlice<'a, T> {
    #[cfg(target_arch = "spirv")]
    pub unsafe fn from_unsafe_raw_parts(inner: &'a mut [T; 1], offset: usize, len: usize) -> Self {
        let data = UnsafeSliceRepr { inner, offset, len };
        Self { data }
    }
}

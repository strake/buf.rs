#![no_std]

extern crate containers;
extern crate io;
extern crate loca;

use containers::collections::{RawVec, Vec};
use core::cmp;
use loca::Alloc;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Read<T, R, A: Alloc> {
    r: R,
    k: usize,
    buf: Vec<T, A>,
}

impl<T, R, A: Alloc + Default> Read<T, R, A> {
    #[inline]
    pub fn with_capacity(r: R, cap: usize) -> Option<Self> {
        Vec::with_capacity(cap).map(|buf| Read { r, buf, k: 0 })
    }
}

impl<T, R, A: Alloc> Read<T, R, A> {
    #[inline]
    pub fn with_capacity_in(a: A, r: R, cap: usize) -> Option<Self> {
        Vec::with_capacity_in(a, cap).map(|buf| Read { r, buf, k: 0 })
    }

    #[inline]
    pub fn from_raw(r: R, raw: RawVec<T, A>) -> Self {
        Read { r, buf: Vec::from_raw(raw), k: 0 }
    }

    #[inline]
    pub fn as_ref(&self) -> &R { &self.r }

    #[inline]
    pub fn as_mut(&mut self) -> &mut R { &mut self.r }
}

impl<T: Copy, R: ::io::Read<T>, A: Alloc> ::io::Read<T> for Read<T, R, A> {
    type Err = R::Err;
    #[inline]
    fn read(&mut self, buf: &mut [T]) -> Result<usize, Self::Err> {
        if self.k == self.buf.len() {
            self.buf.truncate(0);
            self.k = 0;
            {
                let (m, n_opt) = self.r.size_hint();
                self.buf.reserve(n_opt.unwrap_or(m));
            }
            if buf.len() >= self.buf.capacity() { return self.r.read(buf); }
            match self.r.read_onto_vec(&mut self.buf) {
                Ok(n) if n > 0 => (),
                x => return x,
            }
        }
        let l = cmp::min(self.buf.len() - self.k, buf.len());
        buf[0..l].copy_from_slice(&self.buf[0..l]);
        self.k += l;
        Ok(l)
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
         let l = self.buf.len();
         (l, self.r.size_hint().1.and_then(|n| usize::checked_add(l, n)))
    }
}

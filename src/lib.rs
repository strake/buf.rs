#![no_std]

extern crate containers;
extern crate either;
extern crate io;
extern crate loca;

use containers::collections::{RawVec, Vec};
use core::cmp;
use either::{Either, Left, Right};
use io::EndOfFile;
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

/// Pass-thru
impl<T: Copy, R: ::io::Write<T>, A: Alloc> ::io::Write<T> for Read<T, R, A> {
    type Err = R::Err;
    #[inline]
    fn write(&mut self, buf: &[T]) -> Result<usize, Self::Err> { self.r.write(buf) }
    #[inline]
    fn writev(&mut self, buf: &[&[T]]) -> Result<usize, Self::Err> { self.r.writev(buf) }
    #[inline]
    fn flush(&mut self) -> Result<(), Self::Err> { self.r.flush() }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Write<T, W, A: Alloc> {
    w: W,
    buf: Vec<T, A>,
}

impl<T, W, A: Alloc> Write<T, W, A> {
    #[inline]
    pub fn with_capacity_in(a: A, w: W, cap: usize) -> Option<Self> {
        Vec::with_capacity_in(a, cap).map(|buf| Write { w, buf })
    }

    #[inline]
    pub fn from_raw(w: W, raw: RawVec<T, A>) -> Self {
        Write { w, buf: Vec::from_raw(raw) }
    }

    #[inline]
    pub fn as_ref(&self) -> &W { &self.w }

    #[inline]
    pub fn as_mut(&mut self) -> &mut W { &mut self.w }
}

impl<T: Copy, W: ::io::Write<T>, A: Alloc> Write<T, W, A> {
    #[inline]
    pub fn flush_buffer(&mut self) -> Result<(), Either<<W as ::io::Write<T>>::Err, EndOfFile>> {
        self.flush_buffer_and(&[]).map(|_| ())
    }

    fn flush_buffer_and(&mut self, buf: &[T]) -> Result<usize, Either<<W as ::io::Write<T>>::Err, EndOfFile>> {
        while self.buf.len() > 0 {
            let n = self.write_buffer_and(buf)?;
            if n > 0 { return Ok(n) }
        }
        Ok(0)
    }

    fn write_buffer_and(&mut self, buf: &[T]) -> Result<usize, Either<<W as ::io::Write<T>>::Err, EndOfFile>> {
        let n = self.w.writev(&[&self.buf[..], buf]).map_err(Left)?;
        if 0 == n { return Err(Right(EndOfFile)); }
        let l = self.buf.len();
        self.buf.drain(0..::core::cmp::min(n, l));
        Ok(n.saturating_sub(l))
    }
}

impl<T: Copy, W: ::io::Write<T>, A: Alloc> ::io::Write<T> for Write<T, W, A> {
    type Err = Either<W::Err, EndOfFile>;

    fn flush(&mut self) -> Result<(), Self::Err> {
        self.flush_buffer()?;
        self.w.flush().map_err(Left)
    }

    fn write(&mut self, buf: &[T]) -> Result<usize, Self::Err> {
        loop {
            if self.buf.capacity() - self.buf.len() >= buf.len() {
                self.buf.append_slice(buf);
                return Ok(buf.len());
            }
            let n = self.write_buffer_and(buf)?;
            if n > 0 { return Ok(n); }
        }
    }
}

impl<W: ::io::Write<u8>, A: Alloc> ::core::fmt::Write for Write<u8, W, A> {
    #[inline]
    fn write_str(&mut self, s: &str) -> ::core::fmt::Result {
        use io::Write as _Write;
        self.write(s.as_bytes()).map(|_| ()).map_err(|_| ::core::fmt::Error)
    }
}

/// Pass-thru
impl<T: Copy, W: ::io::Read<T>, A: Alloc> ::io::Read<T> for Write<T, W, A> {
    type Err = W::Err;
    #[inline]
    fn read(&mut self, buf: &mut [T]) -> Result<usize, Self::Err> { self.w.read(buf) }
    #[inline]
    fn readv(&mut self, bufs: &mut [&mut [T]]) -> Result<usize, Self::Err> { self.w.readv(bufs) }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) { self.w.size_hint() }
}

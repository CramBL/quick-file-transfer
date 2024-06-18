use anyhow::Result;
use memmap2::Mmap;
use std::{
    fs::File,
    io::{self, Cursor},
    path::Path,
};

pub struct MemoryMappedReader<'file> {
    _mmap: Mmap, // Hold this or segfault
    cursor: Cursor<&'file [u8]>,
}

impl<'file> MemoryMappedReader<'file> {
    pub fn new(path: &'file Path) -> Result<Self> {
        let file = File::open(path)?;
        let mmap = unsafe { Mmap::map(&file)? };

        #[cfg(linux)]
        mmap.advise(memmap2::Advice::PopulateRead)?;

        let cursor = Cursor::new(unsafe { std::slice::from_raw_parts(mmap.as_ptr(), mmap.len()) });
        Ok(Self {
            _mmap: mmap,
            cursor,
        })
    }
}

impl<'file> io::Read for MemoryMappedReader<'file> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.cursor.read(buf)
    }
}

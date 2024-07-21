use anyhow::{bail, Result};
use memmap2::Mmap;
use std::{fs::File, ops::Range, path::Path};

pub struct MemoryMapWrapper {
    mmap: Mmap, // Hold this or segfault
}

impl MemoryMapWrapper {
    pub fn new(path: &Path) -> Result<Self> {
        let file = File::open(path)?;
        let mmap = unsafe { Mmap::map(&file)? };

        #[cfg(target_os = "linux")]
        mmap.advise(memmap2::Advice::PopulateRead)?;

        Ok(Self { mmap })
    }

    pub fn borrow_full(&self) -> &[u8] {
        &self.mmap[..]
    }

    pub fn borrow_slice(&self, range: Range<usize>) -> Result<&[u8]> {
        if range.start > range.end || range.end > self.mmap.len() {
            bail!("Invalid slice range");
        }
        Ok(&self.mmap[range])
    }

    pub fn flen(&self) -> usize {
        self.mmap.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use temp_dir::TempDir;
    use testresult::TestResult;

    #[test]
    fn test_borrow_slice() -> TestResult {
        let d = TempDir::new()?;
        let c = d.child("file");

        let content = b"Hello, world!";
        fs::write(c.as_path(), content)?;

        let reader = MemoryMapWrapper::new(c.as_path())?;
        let slice = reader.borrow_slice(0..reader.flen())?;

        assert_eq!(slice, content);
        Ok(())
    }

    #[test]
    fn test_borrow_slice_at() -> TestResult {
        let d = TempDir::new()?;
        let c = d.child("file");
        let content = b"Hello, world!";
        fs::write(c.as_path(), content)?;

        let path = c.as_path();
        let reader = MemoryMapWrapper::new(path)?;

        let slice1 = reader.borrow_slice(0..5)?;
        let over_lapping_slice = reader.borrow_slice(4..9)?;
        let slice2 = reader.borrow_slice(7..12)?;
        assert_eq!(slice2, b"world");
        assert_eq!(slice1, b"Hello");
        assert_eq!(over_lapping_slice.len(), 5);

        // Test out of bounds
        assert!(reader.borrow_slice(5..14).is_err());

        Ok(())
    }
}

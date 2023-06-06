use std::{cmp, pin::Pin, task::Poll};

use futures_io::{
    AsyncRead as Read, AsyncSeek as Seek, Error, ErrorKind, Result, SeekFrom,
};
use futures_lite::{io, AsyncReadExt};

use crate::header::Header;

/// Representation of an archive entry.
///
/// `Entry` objects implement the `Read` trait, and can be used to extract the
/// data from this archive entry.  If the underlying reader supports the `Seek`
/// trait, then the `Entry` object supports `Seek` as well.
pub struct Entry<'a, R: 'a + Read + Unpin> {
    pub(crate) header: &'a Header,
    pub(crate) reader: &'a mut R,
    pub(crate) length: u64,
    pub(crate) position: u64,
}

impl<'a, R: 'a + Read + Unpin> Entry<'a, R> {
    /// Returns the header for this archive entry.
    pub fn header(&self) -> &Header {
        self.header
    }
}

impl<'a, R: 'a + Read + Unpin> Read for Entry<'a, R> {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut [u8],
    ) -> Poll<Result<usize>> {
        debug_assert!(self.position <= self.length);
        if self.position == self.length {
            return Poll::Ready(Ok(0));
        }
        let max_len =
            cmp::min(self.length - self.position, buf.len() as u64) as usize;

        let pinned = Pin::new(&mut self.reader);
        let bytes_read = pinned.poll_read(cx, &mut buf[0..max_len]);

        if let Poll::Ready(Ok(bytes_read)) = bytes_read {
            self.position += bytes_read as u64;
        }
        debug_assert!(self.position <= self.length);
        bytes_read
    }
}

impl<'a, R: 'a + Read + Seek + Unpin> Seek for Entry<'a, R> {
    fn poll_seek(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        pos: SeekFrom,
    ) -> Poll<Result<u64>> {
        let delta = match pos {
            SeekFrom::Start(offset) => offset as i64 - self.position as i64,
            SeekFrom::End(offset) => {
                self.length as i64 + offset - self.position as i64
            }
            SeekFrom::Current(delta) => delta,
        };
        let new_position = self.position as i64 + delta;
        if new_position < 0 {
            let msg = format!(
                "Invalid seek to negative position ({})",
                new_position
            );
            return Poll::Ready(Err(Error::new(ErrorKind::InvalidInput, msg)));
        }
        let new_position = new_position as u64;
        if new_position > self.length {
            let msg = format!(
                "Invalid seek to position past end of entry ({} vs. {})",
                new_position, self.length
            );
            return Poll::Ready(Err(Error::new(ErrorKind::InvalidInput, msg)));
        }
        let pinned = Pin::new(&mut self.reader);
        let poll = pinned.poll_seek(cx, SeekFrom::Current(delta));

        if let Poll::Ready(Ok(seek_position)) = poll {
            debug_assert!(seek_position == new_position);
            self.position = new_position
        }
        poll
    }
}

impl<'a, R: 'a + Read + Unpin> Drop for Entry<'a, R> {
    fn drop(&mut self) {
        if self.position < self.length {
            // Consume the rest of the data in this entry.
            let mut remaining = self.reader.take(self.length - self.position);
            std::mem::drop(io::copy(&mut remaining, &mut io::sink()));
        }
    }
}

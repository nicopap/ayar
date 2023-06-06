use futures_io::{
    AsyncRead as Read, AsyncWrite as Write, Error, ErrorKind, Result,
};
use futures_lite::{io, AsyncWriteExt};

use crate::archive::GLOBAL_HEADER;
use crate::header::Header;

/// A structure for building Common or BSD-variant archives (the archive format
/// typically used on e.g. BSD and Mac OS X systems).
///
/// This structure has methods for building up an archive from scratch into any
/// arbitrary writer.
pub struct Builder<W: Write> {
    writer: W,
    started: bool,
}

impl<W: Write + Unpin> Builder<W> {
    /// Create a new archive builder with the underlying writer object as the
    /// destination of all data written.
    pub fn new(writer: W) -> Builder<W> {
        Builder { writer, started: false }
    }

    /// Unwrap this archive builder, returning the underlying writer object.
    pub fn into_inner(self) -> Result<W> {
        Ok(self.writer)
    }

    /// Adds a new entry to this archive.
    pub async fn append<R: Read + Unpin>(
        &mut self,
        header: &Header,
        mut data: R,
    ) -> Result<()> {
        if !self.started {
            self.writer.write_all(GLOBAL_HEADER).await?;
            self.started = true;
        }
        header.write(&mut self.writer).await?;
        let actual_size = io::copy(&mut data, &mut self.writer).await?;
        if actual_size != header.size() {
            let msg = format!(
                "Wrong file size (header.size() = {}, actual size was {actual_size})",
                header.size(),
            );
            return Err(Error::new(ErrorKind::InvalidData, msg));
        }
        if actual_size % 2 != 0 {
            self.writer.write_all(&[b'\n']).await?;
        }
        Ok(())
    }
}

#[cfg(never)]
mod tests {
    use super::{Builder, Header};
    use std::io::{Read, Result};
    use std::str;

    struct SlowReader<'a> {
        current_position: usize,
        buffer: &'a [u8],
    }

    impl<'a> Read for SlowReader<'a> {
        fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
            if self.current_position >= self.buffer.len() {
                return Ok(0);
            }
            buf[0] = self.buffer[self.current_position];
            self.current_position += 1;
            return Ok(1);
        }
    }

    #[test]
    fn build_common_archive() {
        let mut builder = Builder::new(Vec::new());
        let mut header1 = Header::new(b"foo.txt".to_vec(), 7);
        header1.set_mtime(1487552916);
        header1.set_uid(501);
        header1.set_gid(20);
        header1.set_mode(0o100644);
        builder.append(&header1, "foobar\n".as_bytes()).unwrap();
        let header2 = Header::new(b"baz.txt".to_vec(), 4);
        builder.append(&header2, "baz\n".as_bytes()).unwrap();
        let actual = builder.into_inner().unwrap();
        let expected = "\
        !<arch>\n\
        foo.txt         1487552916  501   20    100644  7         `\n\
        foobar\n\n\
        baz.txt         0           0     0     0       4         `\n\
        baz\n";
        assert_eq!(str::from_utf8(&actual).unwrap(), expected);
    }

    #[test]
    fn build_bsd_archive_with_long_filenames() {
        let mut builder = Builder::new(Vec::new());
        let mut header1 = Header::new(b"short".to_vec(), 1);
        header1.set_identifier(b"this_is_a_very_long_filename.txt".to_vec());
        header1.set_mtime(1487552916);
        header1.set_uid(501);
        header1.set_gid(20);
        header1.set_mode(0o100644);
        header1.set_size(7);
        builder.append(&header1, "foobar\n".as_bytes()).unwrap();
        let header2 = Header::new(
            b"and_this_is_another_very_long_filename.txt".to_vec(),
            4,
        );
        builder.append(&header2, "baz\n".as_bytes()).unwrap();
        let actual = builder.into_inner().unwrap();
        let expected = "\
        !<arch>\n\
        #1/32           1487552916  501   20    100644  39        `\n\
        this_is_a_very_long_filename.txtfoobar\n\n\
        #1/44           0           0     0     0       48        `\n\
        and_this_is_another_very_long_filename.txt\x00\x00baz\n";
        assert_eq!(str::from_utf8(&actual).unwrap(), expected);
    }

    #[test]
    fn build_bsd_archive_with_space_in_filename() {
        let mut builder = Builder::new(Vec::new());
        let header = Header::new(b"foo bar".to_vec(), 4);
        builder.append(&header, "baz\n".as_bytes()).unwrap();
        let actual = builder.into_inner().unwrap();
        let expected = "\
        !<arch>\n\
        #1/8            0           0     0     0       12        `\n\
        foo bar\x00baz\n";
        assert_eq!(str::from_utf8(&actual).unwrap(), expected);
    }

    #[test]
    fn build_gnu_archive() {
        let names = vec![b"baz.txt".to_vec(), b"foo.txt".to_vec()];
        let mut builder = GnuBuilder::new(Vec::new(), names);
        let mut header1 = Header::new(b"foo.txt".to_vec(), 7);
        header1.set_mtime(1487552916);
        header1.set_uid(501);
        header1.set_gid(20);
        header1.set_mode(0o100644);
        builder.append(&header1, "foobar\n".as_bytes()).unwrap();
        let header2 = Header::new(b"baz.txt".to_vec(), 4);
        builder.append(&header2, "baz\n".as_bytes()).unwrap();
        let actual = builder.into_inner().unwrap();
        let expected = "\
        !<arch>\n\
        foo.txt/        1487552916  501   20    100644  7         `\n\
        foobar\n\n\
        baz.txt/        0           0     0     0       4         `\n\
        baz\n";
        assert_eq!(str::from_utf8(&actual).unwrap(), expected);
    }

    #[test]
    fn build_gnu_archive_with_long_filenames() {
        let names = vec![
            b"this_is_a_very_long_filename.txt".to_vec(),
            b"and_this_is_another_very_long_filename.txt".to_vec(),
        ];
        let mut builder = GnuBuilder::new(Vec::new(), names);
        let mut header1 = Header::new(b"short".to_vec(), 1);
        header1.set_identifier(b"this_is_a_very_long_filename.txt".to_vec());
        header1.set_mtime(1487552916);
        header1.set_uid(501);
        header1.set_gid(20);
        header1.set_mode(0o100644);
        header1.set_size(7);
        builder.append(&header1, "foobar\n".as_bytes()).unwrap();
        let header2 = Header::new(
            b"and_this_is_another_very_long_filename.txt".to_vec(),
            4,
        );
        builder.append(&header2, "baz\n".as_bytes()).unwrap();
        let actual = builder.into_inner().unwrap();
        let expected = "\
        !<arch>\n\
        //                                              78        `\n\
        this_is_a_very_long_filename.txt/\n\
        and_this_is_another_very_long_filename.txt/\n\
        /0              1487552916  501   20    100644  7         `\n\
        foobar\n\n\
        /34             0           0     0     0       4         `\n\
        baz\n";
        assert_eq!(str::from_utf8(&actual).unwrap(), expected);
    }

    #[test]
    fn build_gnu_archive_with_space_in_filename() {
        let names = vec![b"foo bar".to_vec()];
        let mut builder = GnuBuilder::new(Vec::new(), names);
        let header = Header::new(b"foo bar".to_vec(), 4);
        builder.append(&header, "baz\n".as_bytes()).unwrap();
        let actual = builder.into_inner().unwrap();
        let expected = "\
        !<arch>\n\
        foo bar/        0           0     0     0       4         `\n\
        baz\n";
        assert_eq!(str::from_utf8(&actual).unwrap(), expected);
    }

    #[test]
    #[should_panic(
        expected = "Identifier \\\"bar\\\" was not in the list of \
                               identifiers passed to GnuBuilder::new()"
    )]
    fn build_gnu_archive_with_unexpected_identifier() {
        let names = vec![b"foo".to_vec()];
        let mut builder = GnuBuilder::new(Vec::new(), names);
        let header = Header::new(b"bar".to_vec(), 4);
        builder.append(&header, "baz\n".as_bytes()).unwrap();
    }
}

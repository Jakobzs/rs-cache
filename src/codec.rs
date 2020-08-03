use std::io::{ self, Read };
use std::convert::TryFrom;

use bzip2::{
	read::BzDecoder,
	write::BzEncoder,
};
use libflate::gzip::{
	Decoder,
	Encoder,
};

use crate::error::CompressionError;

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum Compression {
	None,
	Bzip2,
	Gzip
}

impl From<Compression> for u8 {
	#[inline]
	fn from(compression: Compression) -> Self {
		match compression {
			Compression::None => 0,
			Compression::Bzip2 => 1,
			Compression::Gzip => 2,
		}
	}
}

impl TryFrom<u8> for Compression {
	type Error = CompressionError;

	#[inline]
	fn try_from(compression: u8) -> Result<Self, Self::Error> {
		match compression {
			0 => Ok(Self::None),
			1 => Ok(Self::Bzip2),
			2 => Ok(Self::Gzip),
			_ => Err(CompressionError::Unsupported(compression))
		}
	}
}

#[inline]
pub fn encode(compression: Compression, data: &[u8], revision: Option<i16>) -> crate::Result<Vec<u8>> {
	let compressed_data = match compression {
		Compression::None => data.to_owned(),
		Compression::Bzip2 => compress_bzip2(data)?,
		Compression::Gzip => compress_gzip(data)?,
	};

	let mut buffer = Vec::with_capacity(compressed_data.len() + 11);
	buffer.push(compression as u8);
	buffer.extend_from_slice(&u32::to_be_bytes(compressed_data.len() as u32));
	
	if compression != Compression::None {
		buffer.extend_from_slice(&u32::to_be_bytes(data.len() as u32));
	}

	buffer.extend(compressed_data);

	let revision = revision.unwrap_or(-1);
	if revision != -1 {
		buffer.extend_from_slice(&i16::to_be_bytes(revision));
	}

	Ok(buffer)
}

#[inline]
pub fn decode(mut buffer: &[u8]) -> crate::Result<Vec<u8>> {
	let mut buf = [0; 1];
	buffer.read_exact(&mut buf)?;
	let compression = Compression::try_from(buf[0])?;

	let mut buf = [0; 4];
	buffer.read_exact(&mut buf)?;
	let len = u32::from_be_bytes(buf) as usize;

	let (_revision, buffer) = match compression {
		Compression::None => decompress_none(buffer, len)?,
		Compression::Bzip2 => decompress_bzip2(buffer, len)?,
		Compression::Gzip => decompress_gzip(buffer, len)?,
	};

	Ok(buffer)
}

fn compress_bzip2(data: &[u8]) -> io::Result<Vec<u8>> {
	let compressor = Encoder::new(data.to_owned())?;
	compressor.finish().into_result()
}

fn compress_gzip(data: &[u8]) -> io::Result<Vec<u8>> {
	let compressor = BzEncoder::new(data.to_owned(), bzip2::Compression::Default);
	let mut compressed_data = compressor.finish()?;
	compressed_data.drain(0..4);

	Ok(compressed_data)
}

fn decompress_none(mut buffer: &[u8], len: usize) -> crate::Result<(i16, Vec<u8>)> {
	let mut compressed_data = vec![0; len];
	buffer.read_exact(&mut compressed_data)?;

	Ok((read_revision(buffer)?, compressed_data))
}

fn decompress_bzip2(mut buffer: &[u8], len: usize) -> crate::Result<(i16, Vec<u8>)> {
	let mut buf = [0; 4];
	buffer.read_exact(&mut buf)?;
	let decompressed_len = u32::from_be_bytes(buf) as usize;

	let mut compressed_data = vec![0; len - 4];
	buffer.read_exact(&mut compressed_data)?;

	let revision = read_revision(buffer)?;

	compressed_data.insert(0, b'1');
	compressed_data.insert(0, b'h');
	compressed_data.insert(0, b'Z');
	compressed_data.insert(0, b'B');
	let mut decompressor = BzDecoder::new(&compressed_data[..]);
	let mut decompressed_data = vec![0; decompressed_len];
	decompressor.read_exact(&mut decompressed_data)?;

	Ok((revision, decompressed_data))
}

fn decompress_gzip(mut buffer: &[u8], len: usize) -> crate::Result<(i16, Vec<u8>)> {
	let mut buf = [0; 4];
	buffer.read_exact(&mut buf)?;
	let decompressed_len = u32::from_be_bytes(buf) as usize;

	let mut compressed_data = vec![0; len - 4];
	buffer.read_exact(&mut compressed_data)?;

	let revision = read_revision(buffer)?;

	let mut decoder = Decoder::new(&compressed_data[..])?;
	let mut decompressed_data = vec![0; decompressed_len];
	decoder.read_exact(&mut decompressed_data)?;

	Ok((revision, decompressed_data))
}

fn read_revision(mut buffer: &[u8]) -> crate::Result<i16> {
	if let Some(remaining) = buffer.bytes().size_hint().1 {
		if remaining >= 2 {
			let mut rev_buffer = [0; 2];
			buffer.read_exact(&mut rev_buffer)?;
			return Ok(i16::from_be_bytes(rev_buffer))
		}
	}

	Ok(-1)
}
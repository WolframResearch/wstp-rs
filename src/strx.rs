//! Unsized types representing encoded string data.

// Note: This file is designed to be separate from the `wstp` crate. In theory, it could
//       (and perhaps should) be an independent crate.

use std::{
    char::DecodeUtf16Error,
    fmt::{self, Display},
    mem,
};

/// UTF-8 string slice.
///
/// This type supports efficient conversion to and from the [`str`] type. It is provided
/// primarily for consistency with the other string types.
#[derive(Debug)]
#[repr(transparent)]
pub struct Utf8Str([u8]);

/// UTF-16 string slice.
#[derive(Debug)]
#[repr(transparent)]
pub struct Utf16Str([u16]);

/// UTF-32 string slice.
#[derive(Debug)]
#[repr(transparent)]
pub struct Utf32Str([u32]);

/// UCS-2 string slice.
#[derive(Debug)]
#[repr(transparent)]
pub struct Ucs2Str([u16]);

//======================================
// Impls
//======================================

//--------------------------------------
// Utf8
//--------------------------------------

impl Utf8Str {
    /// Convert a string slice to a `Utf8`.
    pub fn from_str(str: &str) -> &Utf8Str {
        const _: () = assert!(mem::size_of::<&[u8]>() == mem::size_of::<&str>());
        const _: () = assert!(mem::align_of::<&[u8]>() == mem::align_of::<&str>());

        // SAFETY: Relies on representation of references to unsized data being the same
        //         between types.
        unsafe { Utf8Str::from_utf8_unchecked(str.as_bytes()) }
    }

    /// Convert a slice of bytes to a `Utf8`.
    pub fn from_utf8(utf8: &[u8]) -> Result<&Utf8Str, ()> {
        let str: &str = std::str::from_utf8(utf8).map_err(|_| ())?;

        Ok(Utf8Str::from_str(str))
    }

    /// Access this data as a `str`.
    ///
    /// This view is possible because the [`str`] type represents a UTF-8 encoded
    /// sequence of bytes, just as `Utf8` does.
    pub fn as_str(&self) -> &str {
        let Utf8Str(slice) = self;

        unsafe { std::str::from_utf8_unchecked(slice) }
    }

    /// Converts a slice of bytes to a `Utf8` without validating that the slice
    /// contains valid UTF-8 encoded data.
    pub unsafe fn from_utf8_unchecked(utf8: &[u8]) -> &Utf8Str {
        const _: () = assert!(mem::size_of::<&Utf8Str>() == mem::size_of::<&[u8]>());
        const _: () = assert!(mem::align_of::<&Utf8Str>() == mem::align_of::<&[u8]>());


        // SAFETY: Relies on representation of references to unsized data being the same
        //         between types.
        std::mem::transmute::<&[u8], &Utf8Str>(utf8)
    }

    /// Access the elements of this UTF-8 string as a slice of `u8` elements.
    pub fn as_slice(&self) -> &[u8] {
        let Utf8Str(slice) = self;
        slice
    }
}

//--------------------------------------
// Utf16
//--------------------------------------

impl Utf16Str {
    /// Convert a slice of [`u16`] to a UTF-16 string slice.
    pub fn from_utf16(utf16: &[u16]) -> Result<&Utf16Str, DecodeUtf16Error> {
        // Verify that `utf16` succcessfully decodes as valid UTF-16.
        for result in char::decode_utf16(utf16.iter().copied()) {
            let _: char = result?;
        }

        Ok(unsafe { Utf16Str::from_utf16_unchecked(utf16) })
    }

    /// Converts a slice of bytes to a [`Utf16Str`] without validating that the slice
    /// contains valid UTF-16 encoded data.
    pub unsafe fn from_utf16_unchecked(utf16: &[u16]) -> &Utf16Str {
        const _: () = assert!(mem::size_of::<&Utf16Str>() == mem::size_of::<&[u16]>());
        const _: () = assert!(mem::align_of::<&Utf16Str>() == mem::align_of::<&[u16]>());

        // SAFETY: Relies on representation of references to unsized data being the same
        //         between types.
        std::mem::transmute::<&[u16], &Utf16Str>(utf16)
    }

    /// Access the elements of this UTF-16 string as a slice of `u16` elements.
    pub fn as_slice(&self) -> &[u16] {
        let Utf16Str(slice) = self;
        slice
    }
}

//--------------------------------------
// Utf32
//--------------------------------------

impl Utf32Str {
    /// Converts a slice of bytes to a [`Utf32Str`] without validating that the slice
    /// contains valid UTF-32 encoded data.
    pub unsafe fn from_utf32_unchecked(utf32: &[u32]) -> &Utf32Str {
        const _: () = assert!(mem::size_of::<&Utf32Str>() == mem::size_of::<&[u32]>());
        const _: () = assert!(mem::align_of::<&Utf32Str>() == mem::align_of::<&[u32]>());

        // SAFETY: Relies on representation of references to unsized data being the same
        //         between types.
        std::mem::transmute::<&[u32], &Utf32Str>(utf32)
    }

    /// Access the elements of this UTF-32 string as a slice of `u32` elements.
    pub fn as_slice(&self) -> &[u32] {
        let Utf32Str(slice) = self;
        slice
    }
}

//======================================
// Display Impls
//======================================

impl Display for Utf8Str {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Display::fmt(self.as_str(), f)
    }
}

impl Display for Utf16Str {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let Utf16Str(slice) = self;

        for char in char::decode_utf16(slice.into_iter().copied()) {
            let char: char = match char {
                Ok(char) => char,
                Err(err) => panic!("Utf16Str could not be decoded: {err}"),
            };
            let () = Display::fmt(&char, f)?;
        }

        Ok(())
    }
}

impl Display for Utf32Str {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let Utf32Str(slice) = self;

        for char_u32 in slice.into_iter().copied() {
            let char: char = match char::from_u32(char_u32) {
                Some(char) => char,
                None => panic!("Utf32Str code point is not a valid `char`: {char_u32}"),
            };
            let () = Display::fmt(&char, f)?;
        }

        Ok(())
    }
}

//------------------
// Display tests
//------------------

#[test]
fn test_utf8_str_display() {
    assert_eq!(
        format!("{}", Utf8Str::from_str("hello 👋")),
        String::from("hello 👋")
    );
}


#[test]
fn test_utf16_str_display() {
    let utf16: Vec<u16> = "hello 👋".encode_utf16().collect();
    let utf16: &Utf16Str = Utf16Str::from_utf16(&utf16).unwrap();

    assert_eq!(format!("{}", utf16), String::from("hello 👋"));
}

#[test]
fn test_utf32_str_display() {
    let utf32: Vec<u32> = "hello 👋"
        .chars()
        .map(|char: char| u32::from(char))
        .collect();
    let utf32: &Utf32Str = unsafe { Utf32Str::from_utf32_unchecked(&utf32) };

    assert_eq!(format!("{}", utf32), String::from("hello 👋"));
}

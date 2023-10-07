use std::convert::TryFrom;
use std::ffi::CString;

use crate::{
    sys::{
        self, WSPutArgCount, WSPutInteger16, WSPutInteger32, WSPutInteger64,
        WSPutInteger8, WSPutReal32, WSPutReal64, WSPutUTF16String, WSPutUTF32String,
        WSPutUTF8String, WSPutUTF8Symbol,
    },
    Error, Link,
};

impl Link {
    /// TODO: Augment this function with a `put_type()` method which takes a
    ///       (non-exhaustive) enum value.
    ///
    /// *WSTP C API Documentation:* [`WSPutType()`](https://reference.wolfram.com/language/ref/c/WSPutType.html)
    pub fn put_raw_type(&mut self, type_: i32) -> Result<(), Error> {
        if unsafe { sys::WSPutType(self.raw_link, type_) } == 0 {
            return Err(self.error_or_unknown());
        }

        Ok(())
    }

    /// *WSTP C API Documentation:* [`WSEndPacket()`](https://reference.wolfram.com/language/ref/c/WSEndPacket.html)
    pub fn end_packet(&mut self) -> Result<(), Error> {
        if unsafe { sys::WSEndPacket(self.raw_link) } == 0 {
            return Err(self.error_or_unknown());
        }

        Ok(())
    }

    //==================================
    // Atoms
    //==================================

    /// *WSTP C API Documentation:* [`WSPutUTF8String()`](https://reference.wolfram.com/language/ref/c/WSPutUTF8String.html)
    pub fn put_str(&mut self, string: &str) -> Result<(), Error> {
        let len = i32::try_from(string.as_bytes().len()).expect("usize overflows i32");
        let ptr = string.as_ptr() as *const u8;

        if unsafe { WSPutUTF8String(self.raw_link, ptr, len) } == 0 {
            return Err(self.error_or_unknown());
        }

        Ok(())
    }

    /// *WSTP C API Documentation:* [`WSPutUTF8Symbol()`](https://reference.wolfram.com/language/ref/c/WSPutUTF8Symbol.html)
    pub fn put_symbol(&mut self, symbol: &str) -> Result<(), Error> {
        // FIXME:
        //     Is this extra allocation necessary?WSPutUTF8Symbol doesn't seem to require
        //     that the data contains a NULL terminator, so we should be able to just
        //     pass a pointer to `symbol`'s data.
        let c_string = CString::new(symbol).unwrap();

        let len = i32::try_from(c_string.as_bytes().len()).expect("usize overflows i32");
        let ptr = c_string.as_ptr() as *const u8;

        if unsafe { WSPutUTF8Symbol(self.raw_link, ptr, len) } == 0 {
            return Err(self.error_or_unknown());
        }

        Ok(())
    }

    //==================================
    // Strings
    //==================================

    /// *WSTP C API Documentation:* [`WSPutUTF8String()`](https://reference.wolfram.com/language/ref/c/WSPutUTF8String.html)
    ///
    /// This function will return a WSTP error if `utf8` is not a valid UTF-8 encoded
    /// string.
    pub fn put_utf8_str(&mut self, utf8: &[u8]) -> Result<(), Error> {
        let len = i32::try_from(utf8.len()).expect("usize overflows i32");

        if unsafe { WSPutUTF8String(self.raw_link, utf8.as_ptr(), len) } == 0 {
            return Err(self.error_or_unknown());
        }
        Ok(())
    }

    /// Put a UTF-16 encoded string.
    ///
    /// This function will return a WSTP error if `utf16` is not a valid UTF-16 encoded
    /// string.
    ///
    /// *WSTP C API Documentation:* [`WSPutUTF16String()`](https://reference.wolfram.com/language/ref/c/WSPutUTF16String.html)
    ///
    pub fn put_utf16_str(&mut self, utf16: &[u16]) -> Result<(), Error> {
        let len = i32::try_from(utf16.len()).expect("usize overflows i32");

        if unsafe { WSPutUTF16String(self.raw_link, utf16.as_ptr(), len) } == 0 {
            return Err(self.error_or_unknown());
        }
        Ok(())
    }

    /// Put a UTF-32 encoded string.
    ///
    /// This function will return a WSTP error if `utf32` is not a valid UTF-32 encoded
    /// string.
    ///
    /// *WSTP C API Documentation:* [`WSPutUTF32String()`](https://reference.wolfram.com/language/ref/c/WSPutUTF32String.html)
    pub fn put_utf32_str(&mut self, utf32: &[u32]) -> Result<(), Error> {
        let len = i32::try_from(utf32.len()).expect("usize overflows i32");

        if unsafe { WSPutUTF32String(self.raw_link, utf32.as_ptr(), len) } == 0 {
            return Err(self.error_or_unknown());
        }
        Ok(())
    }

    //==================================
    // Functions
    //==================================

    /// Begin putting a function onto this link.
    ///
    /// # Examples
    ///
    /// Put the expression `{1, 2, 3}` on the link:
    ///
    /// ```
    /// # use wstp::Link;
    /// # fn test() -> Result<(), wstp::Error> {
    /// let mut link = Link::new_loopback()?;
    ///
    /// link.put_function("System`List", 3)?;
    /// link.put_i64(1)?;
    /// link.put_i64(2)?;
    /// link.put_i64(3)?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// Put the expression `foo["a"]["b"]` on the link:
    ///
    /// ```
    /// # use wstp::Link;
    /// # fn test() -> Result<wolfram_expr::Expr, wstp::Error> {
    /// let mut link = Link::new_loopback()?;
    ///
    /// link.put_function(None, 1)?;
    /// link.put_function("Global`foo", 1)?;
    /// link.put_str("a")?;
    /// link.put_str("b")?;
    /// # link.get_expr()
    /// # }
    ///
    /// # use wolfram_expr::{Expr, Symbol};
    /// # assert_eq!(test().unwrap(), Expr::normal(
    /// #     Expr::normal(Symbol::new("Global`foo"), vec![Expr::string("a")]),
    /// #     vec![Expr::string("b")]
    /// # ))
    /// ```
    pub fn put_function<'h, H: Into<Option<&'h str>>>(
        &mut self,
        head: H,
        count: usize,
    ) -> Result<(), Error> {
        self.put_raw_type(i32::from(sys::WSTKFUNC))?;
        self.put_arg_count(count)?;

        if let Some(head) = head.into() {
            self.put_symbol(head)?;
        }

        Ok(())
    }

    /// *WSTP C API Documentation:* [`WSPutArgCount()`](https://reference.wolfram.com/language/ref/c/WSPutArgCount.html)
    pub fn put_arg_count(&mut self, count: usize) -> Result<(), Error> {
        let count: i32 = i32::try_from(count).map_err(|err| {
            Error::custom(format!(
                "put_arg_count: Error converting usize to i32: {}",
                err
            ))
        })?;

        if unsafe { WSPutArgCount(self.raw_link, count) } == 0 {
            return Err(self.error_or_unknown());
        }

        Ok(())
    }

    //==================================
    // Numerics
    //==================================

    /// *WSTP C API Documentation:* [`WSPutInteger64()`](https://reference.wolfram.com/language/ref/c/WSPutInteger64.html)
    pub fn put_i64(&mut self, value: i64) -> Result<(), Error> {
        if unsafe { WSPutInteger64(self.raw_link, value) } == 0 {
            return Err(self.error_or_unknown());
        }
        Ok(())
    }

    /// *WSTP C API Documentation:* [`WSPutInteger32()`](https://reference.wolfram.com/language/ref/c/WSPutInteger32.html)
    pub fn put_i32(&mut self, value: i32) -> Result<(), Error> {
        if unsafe { WSPutInteger32(self.raw_link, value) } == 0 {
            return Err(self.error_or_unknown());
        }
        Ok(())
    }

    /// *WSTP C API Documentation:* [`WSPutInteger16()`](https://reference.wolfram.com/language/ref/c/WSPutInteger16.html)
    pub fn put_i16(&mut self, value: i16) -> Result<(), Error> {
        // Note: This conversion is necessary due to the declaration of WSPutInteger16,
        //       which takes an int for legacy reasons.
        let value = i32::from(value);

        if unsafe { WSPutInteger16(self.raw_link, value) } == 0 {
            return Err(self.error_or_unknown());
        }
        Ok(())
    }

    /// *WSTP C API Documentation:* [`WSPutInteger8()`](https://reference.wolfram.com/language/ref/c/WSPutInteger8.html)
    pub fn put_u8(&mut self, value: u8) -> Result<(), Error> {
        if unsafe { WSPutInteger8(self.raw_link, value) } == 0 {
            return Err(self.error_or_unknown());
        }
        Ok(())
    }

    /// *WSTP C API Documentation:* [`WSPutReal64()`](https://reference.wolfram.com/language/ref/c/WSPutReal64.html)
    pub fn put_f64(&mut self, value: f64) -> Result<(), Error> {
        if unsafe { WSPutReal64(self.raw_link, value) } == 0 {
            return Err(self.error_or_unknown());
        }
        Ok(())
    }

    /// *WSTP C API Documentation:* [`WSPutReal32()`](https://reference.wolfram.com/language/ref/c/WSPutReal32.html)
    pub fn put_f32(&mut self, value: f32) -> Result<(), Error> {
        // Note: This conversion is necessary due to the declaration of WSPutReal32,
        //       which takes a double for legacy reasons.
        let value = f64::from(value);

        if unsafe { WSPutReal32(self.raw_link, value) } == 0 {
            return Err(self.error_or_unknown());
        }
        Ok(())
    }

    //==================================
    // Integer numeric arrays
    //==================================

    /// Put a multidimensional array of [`i64`].
    ///
    /// # Panics
    ///
    /// This function will panic if the product of `dimensions` is not equal to `data.len()`.
    ///
    /// *WSTP C API Documentation:* [`WSPutInteger64Array()`](https://reference.wolfram.com/language/ref/c/WSPutInteger64Array.html)
    pub fn put_i64_array(
        &mut self,
        data: &[i64],
        dimensions: &[usize],
    ) -> Result<(), Error> {
        assert_eq!(
            data.len(),
            dimensions.iter().product(),
            "data length does not equal product of dimensions"
        );

        let dimensions: Vec<i32> = abi_array_dimensions(dimensions)?;

        let result = unsafe {
            sys::WSPutInteger64Array(
                self.raw_link,
                data.as_ptr(),
                dimensions.as_ptr(),
                std::ptr::null_mut(),
                dimensions.len() as i32,
            )
        };

        if result == 0 {
            return Err(self.error_or_unknown());
        }

        Ok(())
    }

    /// Put a multidimensional array of [`i32`].
    ///
    /// # Panics
    ///
    /// This function will panic if the product of `dimensions` is not equal to `data.len()`.
    ///
    /// *WSTP C API Documentation:* [`WSPutInteger32Array()`](https://reference.wolfram.com/language/ref/c/WSPutInteger32Array.html)
    pub fn put_i32_array(
        &mut self,
        data: &[i32],
        dimensions: &[usize],
    ) -> Result<(), Error> {
        assert_eq!(
            data.len(),
            dimensions.iter().product(),
            "data length does not equal product of dimensions"
        );

        let dimensions: Vec<i32> = abi_array_dimensions(dimensions)?;

        let result = unsafe {
            sys::WSPutInteger32Array(
                self.raw_link,
                data.as_ptr(),
                dimensions.as_ptr(),
                std::ptr::null_mut(),
                dimensions.len() as i32,
            )
        };

        if result == 0 {
            return Err(self.error_or_unknown());
        }

        Ok(())
    }

    /// Put a multidimensional array of [`i16`].
    ///
    /// # Panics
    ///
    /// This function will panic if the product of `dimensions` is not equal to `data.len()`.
    ///
    /// *WSTP C API Documentation:* [`WSPutInteger16Array()`](https://reference.wolfram.com/language/ref/c/WSPutInteger16Array.html)
    pub fn put_i16_array(
        &mut self,
        data: &[i16],
        dimensions: &[usize],
    ) -> Result<(), Error> {
        assert_eq!(
            data.len(),
            dimensions.iter().product(),
            "data length does not equal product of dimensions"
        );

        let dimensions: Vec<i32> = abi_array_dimensions(dimensions)?;

        let result = unsafe {
            sys::WSPutInteger16Array(
                self.raw_link,
                data.as_ptr(),
                dimensions.as_ptr(),
                std::ptr::null_mut(),
                dimensions.len() as i32,
            )
        };

        if result == 0 {
            return Err(self.error_or_unknown());
        }

        Ok(())
    }

    /// *WSTP C API Documentation:* [`WSPutInteger8Array()`](https://reference.wolfram.com/language/ref/c/WSPutInteger8Array.html)
    pub fn put_u8_array(
        &mut self,
        data: &[u8],
        dimensions: &[usize],
    ) -> Result<(), Error> {
        assert_eq!(
            data.len(),
            dimensions.iter().product(),
            "data length does not equal product of dimensions"
        );

        let dimensions: Vec<i32> = abi_array_dimensions(dimensions)?;

        let result = unsafe {
            sys::WSPutInteger8Array(
                self.raw_link,
                data.as_ptr(),
                dimensions.as_ptr(),
                std::ptr::null_mut(),
                dimensions.len() as i32,
            )
        };

        if result == 0 {
            return Err(self.error_or_unknown());
        }

        Ok(())
    }

    //==================================
    // Floating-point numeric arrays
    //==================================

    /// Put a multidimensional array of [`f64`].
    ///
    /// # Panics
    ///
    /// This function will panic if the product of `dimensions` is not equal to `data.len()`.
    ///
    /// *WSTP C API Documentation:* [`WSPutReal64Array()`](https://reference.wolfram.com/language/ref/c/WSPutReal64Array.html)
    pub fn put_f64_array(
        &mut self,
        data: &[f64],
        dimensions: &[usize],
    ) -> Result<(), Error> {
        assert_eq!(
            data.len(),
            dimensions.iter().product(),
            "data length does not equal product of dimensions"
        );

        let dimensions: Vec<i32> = abi_array_dimensions(dimensions)?;

        let result = unsafe {
            sys::WSPutReal64Array(
                self.raw_link,
                data.as_ptr(),
                dimensions.as_ptr(),
                std::ptr::null_mut(),
                dimensions.len() as i32,
            )
        };

        if result == 0 {
            return Err(self.error_or_unknown());
        }

        Ok(())
    }

    /// Put a multidimensional array of [`f32`].
    ///
    /// # Panics
    ///
    /// This function will panic if the product of `dimensions` is not equal to `data.len()`.
    ///
    /// *WSTP C API Documentation:* [`WSPutReal32Array()`](https://reference.wolfram.com/language/ref/c/WSPutReal32Array.html)
    pub fn put_f32_array(
        &mut self,
        data: &[f32],
        dimensions: &[usize],
    ) -> Result<(), Error> {
        assert_eq!(
            data.len(),
            dimensions.iter().product(),
            "data length does not equal product of dimensions"
        );

        let dimensions: Vec<i32> = abi_array_dimensions(dimensions)?;

        let result = unsafe {
            sys::WSPutReal32Array(
                self.raw_link,
                data.as_ptr(),
                dimensions.as_ptr(),
                std::ptr::null_mut(),
                dimensions.len() as i32,
            )
        };

        if result == 0 {
            return Err(self.error_or_unknown());
        }

        Ok(())
    }
}

/// Convert `dimensions` to a `Vec<i32>`, which can further be converted to a
/// *const i32, which is needed when calling the low-level WSTP API functions.
fn abi_array_dimensions(dimensions: &[usize]) -> Result<Vec<i32>, Error> {
    let mut i32_dimensions = Vec::with_capacity(dimensions.len());

    for (index, dim) in dimensions.iter().copied().enumerate() {
        match i32::try_from(dim) {
            Ok(val) => i32_dimensions.push(val),
            Err(err) => {
                // Overflowing the array dimension size should probably never happen in
                // well-behaved code, but if it does happen, there is probably some subtle
                // bug, so we should try to emit an error message that is as specific as
                // possible.
                return Err(Error::custom(format!(
                    "in dimensions list {dimensions:?}, the dimension at index {index} \
                     (value: {dim}) overflows i32: {}; during WSTP array operation.",
                    err
                )));
            },
        }
    }

    Ok(i32_dimensions)
}

use std::convert::TryFrom;
use std::ffi::{CStr, CString};
use std::iter::FromIterator;

use crate::{
    sys::{
        self, WSGetArgCount, WSGetInteger64, WSGetReal64, WSGetUTF8String,
        WSReleaseString, WSReleaseSymbol,
    },
    Error, Link,
};

/// Reference to string data borrowed from a [`Link`].
///
/// `LinkStr` is returned from [`Link::get_string_ref()`] and [`Link::get_symbol_ref()`].
///
/// When [`LinkStr::drop()`] is called, `WSReleaseString()` is used to deallocate the
/// underlying string.
#[derive(Debug)]
pub struct LinkStr<'link> {
    link: &'link Link,
    // Note: See `LinkStr::to_str()` for discussion of the safety reasons we *don't* store
    //       a `&str` field (even though that would have the benefit of paying the UTF-8
    //       validation penalty only once).
    c_string: *const u8,
    byte_length: usize,
    is_symbol: bool,
}

impl Link {
    /// TODO: Augment this function with a `get_type()` method which returns a
    ///       (non-exhaustive) enum value.
    ///
    /// If the returned type is [`WSTKERR`][sys::WSTKERR], an error is returned.
    ///
    /// *WSTP C API Documentation:* [`WSGetType()`](https://reference.wolfram.com/language/ref/c/WSGetType.html)
    pub fn get_raw_type(&mut self) -> Result<i32, Error> {
        let type_ = unsafe { sys::WSGetType(self.raw_link) };

        if type_ == sys::WSTKERR {
            return Err(self.error_or_unknown());
        }

        Ok(type_)
    }

    //==================================
    // Atoms
    //==================================

    // TODO:
    //     Reserving the name `get_str()` in case it's possible in the future to implement
    //     implement a `Link::get_str() -> &str` method. It may be safe to do that if
    //     we either:
    //
    //       * Keep track of all the strings we need to call `WSReleaseString` on, and
    //         then do so in `Link::drop()`.
    //       * Verify that we don't need to explicitly deallocate the string data, because
    //         they will be deallocated when the mempool is freed (presumably during
    //         WSClose()?).

    /// *WSTP C API Documentation:* [`WSGetUTF8String()`](https://reference.wolfram.com/language/ref/c/WSGetUTF8String.html)
    pub fn get_string_ref<'link>(&'link mut self) -> Result<LinkStr<'link>, Error> {
        let mut c_string: *const u8 = std::ptr::null();
        let mut num_bytes: i32 = 0;
        let mut num_chars = 0;

        if unsafe {
            WSGetUTF8String(self.raw_link, &mut c_string, &mut num_bytes, &mut num_chars)
        } == 0
        {
            // NOTE: According to the documentation, we do NOT have to release
            //      `string` if the function returns an error.
            return Err(self.error_or_unknown());
        }

        let num_bytes = usize::try_from(num_bytes).unwrap();

        Ok(LinkStr {
            link: self,
            c_string,
            byte_length: num_bytes,
            // Needed to control whether `WSReleaseString` or `WSReleaseSymbol` is called.
            is_symbol: false,
        })
    }

    /// Convenience wrapper around [`Link::get_string_ref()`].
    pub fn get_string(&mut self) -> Result<String, Error> {
        Ok(self.get_string_ref()?.to_str().to_owned())
    }

    /// *WSTP C API Documentation:* [`WSGetUTF8Symbol()`](https://reference.wolfram.com/language/ref/c/WSGetUTF8Symbol.html)
    pub fn get_symbol_ref<'link>(&'link mut self) -> Result<LinkStr<'link>, Error> {
        let mut c_string: *const u8 = std::ptr::null();
        let mut num_bytes: i32 = 0;
        let mut num_chars = 0;

        if unsafe {
            sys::WSGetUTF8Symbol(
                self.raw_link,
                &mut c_string,
                &mut num_bytes,
                &mut num_chars,
            )
        } == 0
        {
            // NOTE: According to the documentation, we do NOT have to release
            //      `string` if the function returns an error.
            return Err(self.error_or_unknown());
        }

        let num_bytes = usize::try_from(num_bytes).unwrap();

        Ok(LinkStr {
            link: self,
            c_string,
            byte_length: num_bytes,
            // Needed to control whether `WSReleaseString` or `WSReleaseSymbol` is called.
            is_symbol: true,
        })
    }

    //==================================
    // Functions
    //==================================

    /// Check that the incoming expression is a function with head `symbol`.
    ///
    /// If the check succeeds, the number of elements in the incoming expression is
    /// returned. Otherwise, an error is returned.
    ///
    /// # Example
    ///
    /// ```
    /// use wstp::Link;
    ///
    /// #[derive(Debug, PartialEq)]
    /// struct Quantity {
    ///     value: f64,
    ///     unit: String,
    /// }
    ///
    /// fn get_quantity(link: &mut Link) -> Result<Quantity, wstp::Error> {
    ///     // Use test_head() to verify that the incoming expression has the expected
    ///     // head.
    ///     let argc = link.test_head("System`Quantity")?;
    ///
    ///     assert!(argc == 2, "expected Quantity to have 2 arguments");
    ///
    ///     let value = link.get_f64()?;
    ///     let unit = link.get_string()?;
    ///
    ///     Ok(Quantity { value, unit })
    /// }
    ///
    /// let mut link = Link::new_loopback().unwrap();
    /// link.put_function("System`Quantity", 2).unwrap();
    /// link.put_f64(5.0).unwrap();
    /// link.put_str("Seconds").unwrap();
    ///
    /// assert_eq!(
    ///     get_quantity(&mut link),
    ///     Ok(Quantity { value: 5.0, unit: "Seconds".into() })
    /// );
    /// ```
    pub fn test_head(&mut self, symbol: &str) -> Result<usize, Error> {
        let c_string = CString::new(symbol).unwrap();

        self.test_head_cstr(c_string.as_c_str())
    }

    /// Check that the incoming expression is a function with head `symbol`.
    ///
    /// This method is an optimized variant of [`Link::test_head()`].
    pub fn test_head_cstr(&mut self, symbol: &CStr) -> Result<usize, Error> {
        let mut len: std::os::raw::c_int = 0;

        if unsafe { sys::WSTestHead(self.raw_link, symbol.as_ptr(), &mut len) } == 0 {
            return Err(self.error_or_unknown());
        }

        let len = usize::try_from(len).expect("c_int overflows usize");

        Ok(len)
    }

    /// *WSTP C API Documentation:* [`WSGetArgCount()`](https://reference.wolfram.com/language/ref/c/WSGetArgCount.html)
    pub fn get_arg_count(&mut self) -> Result<usize, Error> {
        let mut arg_count = 0;

        if unsafe { WSGetArgCount(self.raw_link, &mut arg_count) } == 0 {
            return Err(self.error_or_unknown());
        }

        let arg_count = usize::try_from(arg_count)
            // This really shouldn't happen on any modern 32/64 bit OS. If this
            // condition *is* reached, it's more likely going to be do to an ABI or
            // numeric environment handling issue.
            .expect("WSTKFUNC argument count could not be converted to usize");

        Ok(arg_count)
    }

    //==================================
    // Numerics
    //==================================

    /// *WSTP C API Documentation:* [`WSGetInteger64()`](https://reference.wolfram.com/language/ref/c/WSGetInteger64.html)
    pub fn get_i64(&mut self) -> Result<i64, Error> {
        let mut int = 0;
        if unsafe { WSGetInteger64(self.raw_link, &mut int) } == 0 {
            return Err(self.error_or_unknown());
        }
        Ok(int)
    }

    /// *WSTP C API Documentation:* [`WSGetReal64()`](https://reference.wolfram.com/language/ref/c/WSGetReal64.html)
    pub fn get_f64(&mut self) -> Result<f64, Error> {
        let mut real: f64 = 0.0;
        if unsafe { WSGetReal64(self.raw_link, &mut real) } == 0 {
            return Err(self.error_or_unknown());
        }
        Ok(real)
    }

    /// Get a multidimensional array of [`i64`].
    ///
    /// # Example
    ///
    /// ```
    /// use wstp::Link;
    ///
    /// let mut link = Link::new_loopback().unwrap();
    ///
    /// link.put_i64_array(&[1, 2, 3, 4], &[2, 2]).unwrap();
    ///
    /// let out = link.get_i64_array().unwrap();
    ///
    /// assert_eq!(out.data().len(), 4);
    /// assert_eq!(out.dimensions(), &[2, 2]);
    /// ```
    ///
    /// *WSTP C API Documentation:* [`WSGetInteger64Array()`](https://reference.wolfram.com/language/ref/c/WSGetInteger64Array.html)
    pub fn get_i64_array(&mut self) -> Result<Array<i64>, Error> {
        let Link { raw_link } = *self;

        let mut data_ptr: *mut i64 = std::ptr::null_mut();
        let mut dims_ptr: *mut i32 = std::ptr::null_mut();
        let mut heads_ptr: *mut *mut std::os::raw::c_char = std::ptr::null_mut();
        let mut depth: i32 = 0;

        let result = unsafe {
            sys::WSGetInteger64Array(
                raw_link,
                &mut data_ptr,
                &mut dims_ptr,
                &mut heads_ptr,
                &mut depth,
            )
        };

        if result == 0 {
            return Err(self.error_or_unknown());
        }

        let depth =
            usize::try_from(depth).expect("WSGetInteger64Array depth overflows usize");

        let dims: &[i32] = unsafe { std::slice::from_raw_parts(dims_ptr, depth) };
        let dims = Vec::from_iter(dims.iter().map(|&val| {
            usize::try_from(val)
                .expect("WSGetInteger64Array dimension size overflows usize")
        }));

        Ok(Array {
            link: self,
            data_ptr,
            release_callback: Box::new(move |link: &Link| unsafe {
                sys::WSReleaseInteger64Array(
                    link.raw_link,
                    data_ptr,
                    dims_ptr,
                    heads_ptr,
                    depth as i32,
                );
            }),
            dimensions: dims,
        })
    }

    /// Get a multidimensional array of [`f64`].
    ///
    /// # Example
    ///
    /// ```
    /// use wstp::Link;
    ///
    /// let mut link = Link::new_loopback().unwrap();
    ///
    /// link.put_f64_array(&[3.141, 1.618, 2.718], &[3]).unwrap();
    ///
    /// let out = link.get_f64_array().unwrap();
    ///
    /// assert_eq!(out.data().len(), 3);
    /// assert_eq!(out.data(), &[3.141, 1.618, 2.718]);
    /// assert_eq!(out.dimensions(), &[3]);
    /// ```
    ///
    /// *WSTP C API Documentation:* [`WSGetReal64Array()`](https://reference.wolfram.com/language/ref/c/WSGetReal64Array.html)
    pub fn get_f64_array(&mut self) -> Result<Array<f64>, Error> {
        let Link { raw_link } = *self;

        let mut data_ptr: *mut f64 = std::ptr::null_mut();
        let mut dims_ptr: *mut i32 = std::ptr::null_mut();
        let mut heads_ptr: *mut *mut std::os::raw::c_char = std::ptr::null_mut();
        let mut depth: i32 = 0;

        let result = unsafe {
            sys::WSGetReal64Array(
                raw_link,
                &mut data_ptr,
                &mut dims_ptr,
                &mut heads_ptr,
                &mut depth,
            )
        };

        if result == 0 {
            return Err(self.error_or_unknown());
        }

        let depth =
            usize::try_from(depth).expect("WSGetInteger64Array depth overflows usize");

        let dims: &[i32] = unsafe { std::slice::from_raw_parts(dims_ptr, depth) };
        let dims = Vec::from_iter(dims.iter().map(|&val| {
            usize::try_from(val)
                .expect("WSGetInteger64Array dimension size overflows usize")
        }));

        Ok(Array {
            link: self,
            data_ptr,
            release_callback: Box::new(move |link: &Link| unsafe {
                sys::WSReleaseReal64Array(
                    link.raw_link,
                    data_ptr,
                    dims_ptr,
                    heads_ptr,
                    depth as i32,
                );
            }),
            dimensions: dims,
        })
    }
}


impl<'link> LinkStr<'link> {
    /// Get the UTF-8 string data.
    ///
    /// # Panics
    ///
    /// This function will panic if the contents of the string are not valid UTF-8.
    pub fn to_str<'s>(&'s self) -> &'s str {
        self.try_to_str().expect("WSTP returned non-UTF-8 string")
    }

    #[allow(missing_docs)]
    pub fn try_to_str<'s>(&'s self) -> Result<&'s str, std::str::Utf8Error> {
        let LinkStr {
            link: _,
            c_string,
            byte_length,
            is_symbol: _,
        } = *self;

        // Safety: Assert this pre-condition of `slice::from_raw_parts()`.
        assert!(byte_length < usize::try_from(isize::MAX).unwrap());

        // SAFETY:
        //     It is important that the lifetime of `bytes` is tied to `self` and NOT to
        //     'link. A `&'link str` could outlive the `LinkStr` object, which would lead
        //     to a a use-after-free bug because the string data is deallocated when
        //     `LinkStr` is dropped.
        let bytes: &'s [u8] =
            unsafe { std::slice::from_raw_parts(c_string, byte_length) };

        // TODO: Optimization: Do we trust WSTP enough to always produce valid UTF-8 to
        //       use `str::from_utf8_unchecked()` here? If a client writes malformed data
        //       with WSPutUTF8String, does WSTP validate it and return an error, or would
        //       it be passed through to unsuspecting us?
        // This function will panic if `c_string` is not valid UTF-8.
        std::str::from_utf8(bytes)
    }
}

impl<'link> Drop for LinkStr<'link> {
    fn drop(&mut self) {
        let LinkStr {
            link,
            c_string,
            byte_length: _,
            is_symbol,
        } = *self;

        let c_string = c_string as *const i8;

        // Deallocate the string data.
        match is_symbol {
            true => unsafe { WSReleaseSymbol(link.raw_link, c_string) },
            false => unsafe { WSReleaseString(link.raw_link, c_string) },
        }
    }
}


/// Reference to a multidimensional rectangular array borrowed from a [`Link`].
///
/// [`Array`] is returned from:
///
/// * [`Link::get_i64_array()`]
/// * [`Link::get_f64_array()`]
pub struct Array<'link, T> {
    link: &'link Link,

    data_ptr: *mut T,
    release_callback: Box<dyn FnMut(&Link)>,

    dimensions: Vec<usize>,
}

impl<'link, T> Array<'link, T> {
    /// Access the elements stored in this [`Array`] as a flat buffer.
    pub fn data<'s>(&'s self) -> &'s [T] {
        let data_len: usize = self.dimensions.iter().product();

        // SAFETY:
        //     It is important that the lifetime of `data` is tied to `self` and NOT to
        //     'link. A `&'link Array` could outlive the `Array` object, which would lead
        //     to a a use-after-free bug because the string data is deallocated when
        //     `Array` is dropped.
        let data: &'s [T] =
            unsafe { std::slice::from_raw_parts(self.data_ptr, data_len) };

        data
    }

    /// Get the number of dimensions in this array.
    pub fn rank(&self) -> usize {
        self.dimensions.len()
    }

    /// Get the dimensions of this array.
    pub fn dimensions(&self) -> &[usize] {
        self.dimensions.as_slice()
    }

    /// Length of the first dimension of this array.
    pub fn length(&self) -> usize {
        self.dimensions[0]
    }
}

impl<'link, T> Drop for Array<'link, T> {
    fn drop(&mut self) {
        let Array {
            link,
            ref mut release_callback,
            data_ptr: _,
            dimensions: _,
        } = *self;

        release_callback(link)
    }
}

use std::convert::TryFrom;

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

        if type_ == sys::WSTKERR as i32 {
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
}


impl<'link> LinkStr<'link> {
    /// Get the UTF-8 string data.
    ///
    /// # Panics
    ///
    /// This function will panic if the contents of the string are not valid UTF-8.
    pub fn to_str<'s>(&'s self) -> &'s str {
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
        std::str::from_utf8(bytes).expect("WSTP returned non-UTF-8 string")
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

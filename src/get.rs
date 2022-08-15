use std::ffi::{CStr, CString};
use std::iter::FromIterator;
use std::{convert::TryFrom, fmt, os::raw::c_char};

use crate::{
    sys::{
        self, WSGetArgCount, WSGetInteger16, WSGetInteger32, WSGetInteger64,
        WSGetInteger8, WSGetReal32, WSGetReal64, WSGetUTF16String, WSGetUTF32String,
        WSGetUTF8String, WSReleaseUTF16String, WSReleaseUTF16Symbol,
        WSReleaseUTF32String, WSReleaseUTF32Symbol, WSReleaseUTF8String,
        WSReleaseUTF8Symbol,
    },
    Error, Link, Utf16Str, Utf32Str, Utf8Str,
};

/// Basic unit of expression data read from a [`Link`].
///
/// [`Link::get_token()`] is used to read the next available token from a [`Link`].
#[allow(missing_docs)]
#[derive(Debug)]
pub enum Token<'link> {
    Integer(i64),
    Real(f64),
    Symbol(LinkStr<'link>),
    String(LinkStr<'link>),

    /// A function expression with `length` elements.
    ///
    /// The next expression is the head of the function, followed by `length` number of
    /// expression elements.
    Function {
        length: usize,
    },
}

/// The type of a token available to read from a [`Link`].
///
/// See also [`Token`].
///
/// See the [`WSGetType()`](https://reference.wolfram.com/language/ref/c/WSGetType.html)
/// documentation for a listing of WSTP token types.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum TokenType {
    /// [`WSTKINT`][sys::WSTKINT]
    Integer,
    /// [`WSTKREAL`][sys::WSTKREAL]
    Real,
    /// [`WSTKSYM`][sys::WSTKSYM]
    Symbol,
    /// [`WSTKSTR`][sys::WSTKSTR]
    String,
    /// [`WSTKFUNC`][sys::WSTKFUNC]
    Function,
}

/// String borrowed from a [`Link`].
///
/// `LinkStr` is returned from:
///
/// * [`Link::get_string_ref()`]
/// * [`Link::get_symbol_ref()`].
///
/// When `LinkStr` is dropped, the string is deallocated by the `Link`.
///
/// # Example
///
/// ```
/// use wstp::{Link, LinkStr};
///
/// let mut link = Link::new_loopback().unwrap();
///
/// link.put_str("hello world").unwrap();
///
/// // Read a string from the link
/// let string: LinkStr = link.get_string_ref().unwrap();
///
/// // Get a `&str` from the `LinkStr`
/// assert_eq!(string.as_str(), "hello world");
/// ```
pub struct LinkStr<'link, T: LinkStrType + ?Sized = str> {
    link: &'link Link,

    /// See [`LinkStr::get()`] for discussion of the safety reasons we *don't* store
    /// a `&[T::Element]` field.
    ptr: *const T::Element,
    length: usize,

    // Needed to control whether `WSReleaseString` or `WSReleaseSymbol` is called.
    is_symbol: bool,
}

pub unsafe trait LinkStrType: fmt::Debug {
    type Element;

    unsafe fn from_slice_unchecked<'s>(slice: &'s [Self::Element]) -> &'s Self;

    unsafe fn release(
        link: &Link,
        ptr: *const Self::Element,
        len: usize,
        is_symbol: bool,
    );
}

//======================================
// Impls
//======================================

impl Link {
    /// Get the type of the next token available to read on this link.
    ///
    /// See also [`Link::get_token()`].
    pub fn get_type(&self) -> Result<TokenType, Error> {
        use wstp_sys::{WSTKFUNC, WSTKINT, WSTKREAL, WSTKSTR, WSTKSYM};

        let type_: i32 = self.get_raw_type()?;

        let token_type = match u8::try_from(type_).unwrap() {
            WSTKINT => TokenType::Integer,
            WSTKREAL => TokenType::Real,
            WSTKSTR => TokenType::String,
            WSTKSYM => TokenType::Symbol,
            WSTKFUNC => TokenType::Function,
            _ => return Err(Error::custom(format!("unknown WSLINK type: {}", type_))),
        };

        Ok(token_type)
    }

    /// Read the next token from this link.
    ///
    /// See also [`Link::get_type()`].
    ///
    /// # Example
    ///
    /// Read the expression `{5, "second", foo}` from a link one [`Token`] at a time:
    ///
    /// ```
    /// use wstp::{Link, Token};
    ///
    /// // Put {5, "second", foo}
    /// let mut link = Link::new_loopback().unwrap();
    /// link.put_function("System`List", 3).unwrap();
    /// link.put_i64(5).unwrap();
    /// link.put_str("second").unwrap();
    /// link.put_symbol("Global`foo").unwrap();
    ///
    /// // Read it back
    /// assert!(matches!(link.get_token().unwrap(), Token::Function { length: 3 }));
    /// assert!(matches!(link.get_token().unwrap(), Token::Symbol(s) if s.as_str() == "System`List"));
    /// assert!(matches!(link.get_token().unwrap(), Token::Integer(5)));
    /// assert!(matches!(link.get_token().unwrap(), Token::String(s) if s.as_str() == "second"));
    /// assert!(matches!(link.get_token().unwrap(), Token::Symbol(s) if s.as_str() == "Global`foo"));
    /// ```
    pub fn get_token(&mut self) -> Result<Token, Error> {
        let token = match self.get_type()? {
            TokenType::Integer => Token::Integer(self.get_i64()?),
            TokenType::Real => Token::Real(self.get_f64()?),
            TokenType::String => Token::String(self.get_string_ref()?),
            TokenType::Symbol => Token::Symbol(self.get_symbol_ref()?),
            TokenType::Function => Token::Function {
                length: self.get_arg_count()?,
            },
        };

        Ok(token)
    }

    /// Get the raw type of the next token available to read on this link.
    ///
    /// If the returned type is [`WSTKERR`][sys::WSTKERR], an error is returned.
    ///
    /// See also [`Link::get_type()`].
    ///
    /// *WSTP C API Documentation:* [`WSGetType()`](https://reference.wolfram.com/language/ref/c/WSGetType.html)
    pub fn get_raw_type(&self) -> Result<i32, Error> {
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
    pub fn get_string_ref<'link>(&'link mut self) -> Result<LinkStr<'link, str>, Error> {
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
            ptr: c_string,
            length: num_bytes,
            is_symbol: false,
        })
    }

    /// Convenience wrapper around [`Link::get_string_ref()`].
    pub fn get_string(&mut self) -> Result<String, Error> {
        Ok(self.get_string_ref()?.get().to_owned())
    }

    /// *WSTP C API Documentation:* [`WSGetUTF8Symbol()`](https://reference.wolfram.com/language/ref/c/WSGetUTF8Symbol.html)
    pub fn get_symbol_ref<'link>(&'link mut self) -> Result<LinkStr<'link, str>, Error> {
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
            ptr: c_string,
            length: num_bytes,
            is_symbol: true,
        })
    }

    //==================================
    // Strings
    //==================================

    /// *WSTP C API Documentation:* [`WSGetUTF8String()`](https://reference.wolfram.com/language/ref/c/WSGetUTF8String.html)
    pub fn get_utf8_str<'link>(
        &'link mut self,
    ) -> Result<LinkStr<'link, Utf8Str>, Error> {
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

            ptr: c_string,
            length: num_bytes,

            is_symbol: false,
        })
    }

    /// *WSTP C API Documentation:* [`WSGetUTF16String()`](https://reference.wolfram.com/language/ref/c/WSGetUTF16String.html)
    pub fn get_utf16_str<'link>(
        &'link mut self,
    ) -> Result<LinkStr<'link, Utf16Str>, Error> {
        let mut c_string: *const u16 = std::ptr::null();
        let mut num_elems: i32 = 0;
        let mut num_chars = 0;

        if unsafe {
            WSGetUTF16String(self.raw_link, &mut c_string, &mut num_elems, &mut num_chars)
        } == 0
        {
            // NOTE: According to the documentation, we do NOT have to release
            //      `string` if the function returns an error.
            return Err(self.error_or_unknown());
        }

        let num_elems = usize::try_from(num_elems).unwrap();

        Ok(LinkStr {
            link: self,

            ptr: c_string,
            length: num_elems,

            is_symbol: false,
        })
    }

    /// *WSTP C API Documentation:* [`WSGetUTF32String()`](https://reference.wolfram.com/language/ref/c/WSGetUTF32String.html)
    pub fn get_utf32_str<'link>(
        &'link mut self,
    ) -> Result<LinkStr<'link, Utf32Str>, Error> {
        let mut c_string: *const u32 = std::ptr::null();
        let mut num_elems: i32 = 0;

        if unsafe { WSGetUTF32String(self.raw_link, &mut c_string, &mut num_elems) } == 0
        {
            // NOTE: According to the documentation, we do NOT have to release
            //      `string` if the function returns an error.
            return Err(self.error_or_unknown());
        }

        let num_elems = usize::try_from(num_elems).unwrap();

        Ok(LinkStr {
            link: self,

            ptr: c_string,
            length: num_elems,

            is_symbol: false,
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

    /// *WSTP C API Documentation:* [`WSGetInteger32()`](https://reference.wolfram.com/language/ref/c/WSGetInteger32.html)
    pub fn get_i32(&mut self) -> Result<i32, Error> {
        let mut int = 0;
        if unsafe { WSGetInteger32(self.raw_link, &mut int) } == 0 {
            return Err(self.error_or_unknown());
        }
        Ok(int)
    }

    /// *WSTP C API Documentation:* [`WSGetInteger16()`](https://reference.wolfram.com/language/ref/c/WSGetInteger16.html)
    pub fn get_i16(&mut self) -> Result<i16, Error> {
        let mut int = 0;
        if unsafe { WSGetInteger16(self.raw_link, &mut int) } == 0 {
            return Err(self.error_or_unknown());
        }
        Ok(int)
    }

    /// *WSTP C API Documentation:* [`WSGetInteger8()`](https://reference.wolfram.com/language/ref/c/WSGetInteger8.html)
    pub fn get_u8(&mut self) -> Result<u8, Error> {
        let mut int = 0;
        if unsafe { WSGetInteger8(self.raw_link, &mut int) } == 0 {
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

    /// *WSTP C API Documentation:* [`WSGetReal32()`](https://reference.wolfram.com/language/ref/c/WSGetReal32.html)
    pub fn get_f32(&mut self) -> Result<f32, Error> {
        let mut real: f32 = 0.0;
        if unsafe { WSGetReal32(self.raw_link, &mut real) } == 0 {
            return Err(self.error_or_unknown());
        }
        Ok(real)
    }

    //==================================
    // Integer numeric arrays
    //==================================

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
        unsafe { self.get_array(sys::WSGetInteger64Array, sys::WSReleaseInteger64Array) }
    }

    /// *WSTP C API Documentation:* [`WSGetInteger32Array()`](https://reference.wolfram.com/language/ref/c/WSGetInteger32Array.html)
    pub fn get_i32_array(&mut self) -> Result<Array<i32>, Error> {
        unsafe { self.get_array(sys::WSGetInteger32Array, sys::WSReleaseInteger32Array) }
    }

    /// *WSTP C API Documentation:* [`WSGetInteger16Array()`](https://reference.wolfram.com/language/ref/c/WSGetInteger16Array.html)
    pub fn get_i16_array(&mut self) -> Result<Array<i16>, Error> {
        unsafe { self.get_array(sys::WSGetInteger16Array, sys::WSReleaseInteger16Array) }
    }

    /// *WSTP C API Documentation:* [`WSGetInteger8Array()`](https://reference.wolfram.com/language/ref/c/WSGetInteger8Array.html)
    pub fn get_u8_array(&mut self) -> Result<Array<u8>, Error> {
        unsafe { self.get_array(sys::WSGetInteger8Array, sys::WSReleaseInteger8Array) }
    }

    //==================================
    // Floating-point numeric arrays
    //==================================

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
        unsafe { self.get_array(sys::WSGetReal64Array, sys::WSReleaseReal64Array) }
    }

    /// *WSTP C API Documentation:* [`WSGetReal32Array()`](https://reference.wolfram.com/language/ref/c/WSGetReal32Array.html)
    pub fn get_f32_array(&mut self) -> Result<Array<f32>, Error> {
        unsafe { self.get_array(sys::WSGetReal32Array, sys::WSReleaseReal32Array) }
    }

    #[allow(non_snake_case)]
    unsafe fn get_array<T: 'static>(
        &mut self,
        WSGetTArray: unsafe extern "C" fn(
            sys::WSLINK,
            *mut *mut T,
            *mut *mut i32,
            *mut *mut *mut c_char,
            *mut i32,
        ) -> i32,
        WSReleaseTArray: unsafe extern "C" fn(
            sys::WSLINK,
            *mut T,
            *mut i32,
            *mut *mut c_char,
            i32,
        ),
    ) -> Result<Array<T>, Error> {
        let Link { raw_link } = *self;

        let mut data_ptr: *mut T = std::ptr::null_mut();
        let mut dims_ptr: *mut i32 = std::ptr::null_mut();
        let mut heads_ptr: *mut *mut c_char = std::ptr::null_mut();
        let mut depth: i32 = 0;

        let result: i32 = {
            WSGetTArray(
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

        let depth: usize =
            usize::try_from(depth).expect("WSGet*Array depth overflows usize");

        let dims: &[i32] = { std::slice::from_raw_parts(dims_ptr, depth) };
        let dims: Vec<usize> = Vec::from_iter(dims.iter().map(|&val| {
            usize::try_from(val)
                .expect("WSGetInteger64Array dimension size overflows usize")
        }));

        Ok(Array {
            link: self,
            data_ptr,
            release_callback: Box::new(move |link: &Link| unsafe {
                WSReleaseTArray(
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

impl<'link, T: LinkStrType + ?Sized> LinkStr<'link, T> {
    /// Get the string contained by this `LinkStr`.
    pub fn get<'this>(&'this self) -> &'this T {
        let LinkStr {
            link: _,
            ptr,
            length,
            is_symbol: _,
        } = *self;

        unsafe {
            // SAFETY:
            //     It is important that the lifetime of `slice` is tied to `self` and NOT
            //     to 'link. A `&'link str` could outlive the `LinkStr` object, which
            //     would lead to a a use-after-free bug because the string data is
            //     deallocated when `LinkStr` is dropped.
            let slice: &'this [T::Element] = std::slice::from_raw_parts(ptr, length);

            // SAFETY:
            //     This depends on the assumption that WSTP always returns correctly
            //     encoded UTF-8/UTF-16/UTF-32/UCS-2. We do not do any validation of
            //     the encoding here.
            //
            // TODO: Do we trust WSTP enough to always produce valid UTF-8 to
            //       use `str::from_utf8_unchecked()` here? If a client writes malformed
            //       data with WSPutUTF8String, does WSTP validate it and return an error,
            //       or would it be passed through to unsuspecting us?
            T::from_slice_unchecked(slice)
        }
    }
}

impl<'link> LinkStr<'link, str> {
    /// Get the UTF-8 string data.
    pub fn as_str<'s>(&'s self) -> &'s str {
        self.get()
    }

    /// Get the UTF-8 string data.
    #[deprecated(note = "Use LinkStr::as_str() instead")]
    pub fn to_str<'s>(&'s self) -> &'s str {
        self.get()
    }
}

impl<'link, T: ?Sized + LinkStrType> Drop for LinkStr<'link, T> {
    fn drop(&mut self) {
        let LinkStr {
            link,
            ptr,
            length,
            is_symbol,
        } = *self;

        let () = unsafe { T::release(link, ptr, length, is_symbol) };
    }
}

//======================================
// LinkStrType impls
//======================================

unsafe impl LinkStrType for str {
    type Element = u8;

    unsafe fn from_slice_unchecked<'s>(slice: &'s [Self::Element]) -> &'s Self {
        let str: &'s str = std::str::from_utf8_unchecked(slice);
        str
    }

    unsafe fn release(
        link: &Link,
        ptr: *const Self::Element,
        len: usize,
        is_symbol: bool,
    ) {
        let len = i32::try_from(len).expect("LinkStr usize length overflows i32");

        // Deallocate the string data.
        match is_symbol {
            true => WSReleaseUTF8Symbol(link.raw_link, ptr, len),
            false => WSReleaseUTF8String(link.raw_link, ptr, len),
        }
    }
}

unsafe impl LinkStrType for Utf8Str {
    type Element = u8;

    // unsafe fn from_raw_parts_unchecked<'s>(ptr: *const u8, len: usize) -> &'s Self {
    unsafe fn from_slice_unchecked<'s>(slice: &'s [Self::Element]) -> &'s Self {
        let str: &'s Utf8Str = Utf8Str::from_utf8_unchecked(slice);
        str
    }

    unsafe fn release(
        link: &Link,
        ptr: *const Self::Element,
        len: usize,
        is_symbol: bool,
    ) {
        let len = i32::try_from(len).expect("LinkStr usize length overflows i32");

        // Deallocate the string data.
        match is_symbol {
            true => WSReleaseUTF8Symbol(link.raw_link, ptr, len),
            false => WSReleaseUTF8String(link.raw_link, ptr, len),
        }
    }
}

unsafe impl LinkStrType for Utf16Str {
    type Element = u16;

    // unsafe fn from_raw_parts_unchecked<'s>(ptr: *const u8, len: usize) -> &'s Self {
    unsafe fn from_slice_unchecked<'s>(slice: &'s [Self::Element]) -> &'s Self {
        let str: &'s Utf16Str = Utf16Str::from_utf16_unchecked(slice);
        str
    }

    unsafe fn release(
        link: &Link,
        ptr: *const Self::Element,
        len: usize,
        is_symbol: bool,
    ) {
        let len = i32::try_from(len).expect("LinkStr usize length overflows i32");

        // Deallocate the string data.
        match is_symbol {
            true => WSReleaseUTF16Symbol(link.raw_link, ptr, len),
            false => WSReleaseUTF16String(link.raw_link, ptr, len),
        }
    }
}

unsafe impl LinkStrType for Utf32Str {
    type Element = u32;

    // unsafe fn from_raw_parts_unchecked<'s>(ptr: *const u8, len: usize) -> &'s Self {
    unsafe fn from_slice_unchecked<'s>(slice: &'s [Self::Element]) -> &'s Self {
        let str: &'s Utf32Str = Utf32Str::from_utf32_unchecked(slice);
        str
    }

    unsafe fn release(
        link: &Link,
        ptr: *const Self::Element,
        len: usize,
        is_symbol: bool,
    ) {
        let len = i32::try_from(len).expect("LinkStr usize length overflows i32");

        // Deallocate the string data.
        match is_symbol {
            true => WSReleaseUTF32Symbol(link.raw_link, ptr, len),
            false => WSReleaseUTF32String(link.raw_link, ptr, len),
        }
    }
}


/// Reference to a multidimensional rectangular array borrowed from a [`Link`].
///
/// [`Array`] is returned from:
///
/// * [`Link::get_i64_array()`]
/// * [`Link::get_i32_array()`]
/// * [`Link::get_i16_array()`]
/// * [`Link::get_u8_array()`]
/// * [`Link::get_f64_array()`]
/// * [`Link::get_f32_array()`]
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

//======================================
// Formatting impls
//======================================

impl<'link, T: LinkStrType + fmt::Debug + ?Sized> fmt::Debug for LinkStr<'link, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let LinkStr {
            link,
            ptr,
            length,
            is_symbol,
        } = self;

        let value = format!("{:?}", self.get());

        f.debug_struct("LinkStr")
            .field("<value>", &value)
            .field("link", link)
            .field("ptr", ptr)
            .field("length", length)
            .field("is_symbol", is_symbol)
            .finish()
    }
}

impl<'link, T> fmt::Debug for Array<'link, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let Array {
            link,
            data_ptr,
            release_callback: _,
            dimensions,
        } = self;

        f.debug_struct("Array")
            .field("link", link)
            .field("dimensions", dimensions)
            .field("data_ptr", data_ptr)
            .finish()
    }
}

use crate::{
    sys::{self, WSLINK},
    Error, Link,
};

use std::collections::HashMap;
use std::sync::Mutex;

struct ForceSend<T>(T);

unsafe impl<T> Send for ForceSend<T> {}

lazy_static::lazy_static! {
    /// Hash map used to store the closure passed to [`Link::wait_with_callback()`].
    ///
    /// This is a workaround for the fact that [WSWaitForLinkActivityWithCallback][sys::WSWaitForLinkActivityWithCallback]
    /// takes a function pointer as an argument, but provides no way to provide a piece of
    /// data to that function pointer. Both pieces of data are required to pass a Rust
    /// closure across the FFI boundry. Instead, we store the closure in this global static
    /// hash map, and look it up inside the callback trampoline function.
    static ref WAIT_CALLBACKS: Mutex<ForceSend<HashMap<WSLINK, *mut std::ffi::c_void>>> = Mutex::new(ForceSend(HashMap::new()));
}

impl Link {
    /// *WSTP C API Documentation:* [`WSWaitForLinkActivity`](https://reference.wolfram.com/language/ref/c/WSWaitForLinkActivity.html)
    pub fn wait(&mut self) -> Result<(), Error> {
        let Link { raw_link } = *self;

        let result: i32 = unsafe { sys::WSWaitForLinkActivity(raw_link) };

        match result as u32 {
            sys::WSWAITSUCCESS => Ok(()),
            sys::WSWAITERROR => Err(self.error_or_unknown()),
            _ => Err(Error::custom(format!(
                "WSWaitForLinkActivity returned unexpected value: {}",
                result
            ))),
        }
    }

    /// Wait for data to become available, periodically calling a callback.
    ///
    /// `true` will be returned if data is available. `false` will be returned if the
    /// callback returns [`Break`][std::ops::ControlFlow::Break].
    ///
    /// # Example
    ///
    /// ```
    /// use wstp::{Link, Protocol};
    ///
    /// let mut listener = Link::listen(Protocol::IntraProcess, "").unwrap();
    ///
    /// let mut counter = 0;
    ///
    /// listener
    ///     .wait_with_callback(|_: &mut Link| {
    ///         use std::ops::ControlFlow;
    ///
    ///         counter += 1;
    ///
    ///         if counter < 5 {
    ///             ControlFlow::Continue(())
    ///         } else {
    ///             ControlFlow::Break(())
    ///         }
    ///     })
    ///     .unwrap();
    /// ```
    ///
    /// # User data fields
    ///
    /// This function will temporarily replace any user data values (set using
    /// [Link::set_user_data]) which are associated with the current link. The user
    /// data values on the `&mut Link` parameter inside the callback are
    /// an implementation detail of this function and must not be modified.
    ///
    /// *WSTP C API Documentation:* [`WSWaitForLinkActivityWithCallback`](https://reference.wolfram.com/language/ref/c/WSWaitForLinkActivityWithCallback.html)
    pub fn wait_with_callback<F>(&mut self, callback: F) -> Result<bool, Error>
    where
        F: FnMut(&mut Link) -> std::ops::ControlFlow<()> + Send + Sync,
    {
        let Link { raw_link } = *self;

        let result: i32;

        unsafe {
            let boxed_closure_ptr = Box::into_raw(Box::new(callback));

            {
                let mut lock = WAIT_CALLBACKS
                    .lock()
                    .expect("failed to acquire lock on WAIT_CALLBACKS");

                let callbacks = &mut lock.0;

                if callbacks.contains_key(&raw_link) {
                    // Drop `lock` so we don't poisen it by panicking here.
                    drop(lock);
                    panic!("wait_with_callback: link is already being waited on with a callback");
                }

                callbacks.insert(raw_link, boxed_closure_ptr as *mut std::ffi::c_void);
            }

            result = sys::WSWaitForLinkActivityWithCallback(
                raw_link,
                Some(link_wait_callback_trampoline::<F>),
            );

            {
                let mut lock = WAIT_CALLBACKS
                    .lock()
                    .expect("failed to acquire lock on WAIT_CALLBACKS");

                let callbacks = &mut lock.0;

                callbacks.remove(&raw_link);
            }

            // Drop the closure value.
            Box::from_raw(boxed_closure_ptr);
        };

        match result as u32 {
            sys::WSWAITSUCCESS => Ok(true),
            sys::WSWAITCALLBACKABORTED => Ok(false),
            sys::WSWAITERROR => Err(self.error_or_unknown()),
            _ => Err(Error::custom(format!(
                "WSWaitForLinkActivity returned unexpected value: {}",
                result
            ))),
        }
    }
}

unsafe extern "C" fn link_wait_callback_trampoline<F>(
    mut raw_link: sys::WSLINK,
    _unused_void: *mut std::ffi::c_void,
) -> i32
where
    F: FnMut(&mut Link) -> std::ops::ControlFlow<()> + Send + Sync,
{
    // Catch any panics which result from `expect()` or `user_closure()` to prevent
    // unwinding over C stack frames.
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        // let (raw_user_closure, _) = link.user_data();
        let raw_user_closure: *mut std::ffi::c_void = {
            let lock = WAIT_CALLBACKS
                .lock()
                .expect("failed to acquire lock on WAIT_CALLBACKS");

            *lock
                .0
                .get(&raw_link)
                .expect("link has no associated wait closure in WAIT_CALLBACKS")
        };

        let link: &mut Link = Link::unchecked_ref_cast_mut(&mut raw_link);

        let user_closure: &mut F = (raw_user_closure as *mut F)
            .as_mut()
            .expect("link wait callback is unexpectedly NULL");

        user_closure(link)
    }));

    match result {
        Ok(std::ops::ControlFlow::Break(())) => 1,
        Ok(std::ops::ControlFlow::Continue(())) => 0,
        // If a panic occurs, stop waiting.
        Err(_) => 1,
    }
}

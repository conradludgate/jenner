use std::{ptr::NonNull, task::Context};

#[doc(hidden)]
// This token may outlive a poll's Context. It must be replaced with the new
// resume argument before the coroutine dereferences it on a subsequent poll.
pub struct UnsafeContextRef(
    // Safety invariant: preserves the pointer obtained from a Context borrow;
    // its lifetime is erased, so it is not necessarily still live.
    NonNull<Context<'static>>,
);

impl UnsafeContextRef {
    #[doc(hidden)]
    /// # Safety
    ///
    /// The caller must use the token only in the poll that supplied it, on that
    /// poll's thread. Its Context and waker must remain live, and access to the
    /// Context must be exclusive for the returned borrow. Neither that borrow
    /// nor references obtained through it may escape the poll or cross a yield.
    pub unsafe fn get_context<'a, 'b>(&mut self) -> &'a mut Context<'b> {
        // SAFETY: From preserves the original Context pointer's alignment and
        // provenance. The caller guarantees it is still live on this thread,
        // that its contents remain valid, and that this borrow is exclusive.
        // The cast erases only the Context lifetime; the caller also confines
        // all uses of the resulting reference to the original active poll.
        unsafe { &mut *self.0.as_ptr().cast::<Context<'b>>() }
    }
}

impl<'a> From<&mut Context<'a>> for UnsafeContextRef {
    /// # Safety-usable invariant
    ///
    /// The token points to the same Context with unchanged alignment and
    /// provenance. This does not extend the Context's or its waker's lifetime.
    fn from(cx: &mut Context<'a>) -> Self {
        // Erase the lifetime on a raw pointer without creating a longer-lived
        // reference. Dereferencing it is restricted by get_context's contract.
        Self(NonNull::from(cx).cast())
    }
}

// SAFETY: Moving the opaque pointer does not access the non-Send Context.
// Fields are private and its only dereference is unsafe get_context, whose
// caller must restrict access to the original poll and thread. The generated
// coroutine replaces this token on resumption before dereferencing it again.
unsafe impl Send for UnsafeContextRef {}

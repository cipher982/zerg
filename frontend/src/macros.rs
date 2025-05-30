//! Small crate-wide convenience macros.

/// Acquire a **mutable** borrow from a `RefCell` (or `Rc<RefCell>`).
/// If another mutable borrow is still active the call panics – the standard
/// panic message emitted by `RefCell::borrow_mut()` is preserved to keep the
/// macro zero-cost.
///
/// ```rust,ignore
/// use std::cell::RefCell;
/// let cell = RefCell::new(1);
/// {
///     let mut n = mut_borrow!(cell);
///     *n += 1;
/// }
/// assert_eq!(*cell.borrow(), 2);
/// ```
#[macro_export]
macro_rules! mut_borrow {
    ($cell:expr) => {
        // The explicit expect message is important – default panic is hard
        // to trace when it bubbles up from deep inside borrowed call-stacks.
        $cell
            .borrow_mut()
    };
}

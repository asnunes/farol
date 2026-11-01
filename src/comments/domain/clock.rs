//! When something was written, and what to call it.

/// The time and the ids a thread is stamped with.
///
/// A port because both come from outside the program: the wall clock and the
/// process. Read directly, a test could not say what a reply was stamped with
/// or which id a thread got.
pub trait Clock: Send + Sync {
    /// Seconds since the epoch, the clock `viewed` is stamped with too.
    fn now(&self) -> String;

    /// An id that survives leaving this machine.
    fn id(&self) -> String;
}

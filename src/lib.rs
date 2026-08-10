pub mod cmd;
pub mod diff;
/// What farol reports when it cannot do what was asked. Above the layers,
/// not beneath them — see the module docs.
pub mod error;
pub mod map;
pub mod progress;
pub mod server;
pub mod shared;

#[cfg(test)]
pub mod testing;

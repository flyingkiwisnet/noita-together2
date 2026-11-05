//! Authenticated, bounded binary local transport. No game or service authority.
pub mod protocol;
pub mod server;
pub mod client;
pub mod private;
pub mod owner;
mod ffi;
#[cfg(test)]mod tests;

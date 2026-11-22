//! Compile-time selectable restaurant sources.

#[cfg(feature = "source-fehd")]
pub mod fehd;
#[cfg(feature = "source-foodpanda")]
pub mod foodpanda;
#[cfg(feature = "source-openrice")]
pub mod openrice;

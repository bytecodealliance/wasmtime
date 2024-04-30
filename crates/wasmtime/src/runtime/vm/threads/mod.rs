#[cfg(feature = "threads")]
mod parking_spot;

#[cfg(feature = "threads")]
mod shared_memory;
#[cfg(feature = "threads")]
pub use shared_memory::SharedMemory;

#[cfg(not(feature = "threads"))]
mod shared_memory_disabled;
#[cfg(not(feature = "threads"))]
pub use shared_memory_disabled::SharedMemory;

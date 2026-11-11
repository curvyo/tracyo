//! Tracyo's pure logic: nothing here touches the file system, the clock, threads, processes or
//! a window; callers pass bytes, times and sizes in.

#![forbid(unsafe_code)]

pub mod job;
pub mod picture;
pub mod progress;
pub mod protocol;
pub mod request;
pub mod settings;
pub mod svg;

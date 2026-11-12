//! Tracyo's pure logic: nothing here touches the file system, the clock, threads, processes or
//! a window; callers pass bytes, times and sizes in.

#![forbid(unsafe_code)]

pub mod analysis;
pub mod clip;
pub mod crop;
pub mod grid;
pub mod job;
pub mod join;
pub mod method;
pub mod output_wire;
pub mod palette;
pub mod parts;
pub mod picture;
pub mod progress;
pub mod protocol;
pub mod region_edit;
pub mod regions;
pub mod request;
pub mod settings;
pub mod shapes;
pub mod stack;
pub mod stages;
pub mod svg;
pub mod text;
pub mod wash;

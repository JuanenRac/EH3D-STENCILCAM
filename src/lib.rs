// EH3D-STENCILCAM - solder paste stencils from the paste layer of a board.
// Copyright (C) 2026 JuanenRac (Electro Hobby 3D). GPL-3.0-or-later.
#![forbid(unsafe_code)]
//! The pads of a paste layer become openings; each opening is shrunk, rounded and compensated for the kerf of the laser; the ratios that decide whether the paste
//! comes out of an opening are checked; and the closed paths are written as G-code for a laser or as HP-GL for a plotter.

pub mod browse;
pub mod config;
pub mod dxf_in;
#[cfg(feature = "gui")]
pub mod gui;
pub mod lang;
pub mod layer;
pub mod openings;
pub mod output;
pub mod profile;
pub mod shot;
pub mod viewport;
pub mod window;

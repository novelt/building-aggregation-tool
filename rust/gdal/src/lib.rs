/*
This file is part of the Building Aggregration Tool
Copyright (C) 2022 Novel-T

The Building Aggregration Tool is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU General Public License for more details.

You should have received a copy of the GNU General Public License
along with this program.  If not, see <http://www.gnu.org/licenses/>.
*/
//! [GDAL](http://gdal.org/) bindings for Rust.
//!
//! A high-level API to access the GDAL library, for vector and raster data.
//!


pub use version::version_info;

pub mod config;
pub mod errors;
mod gdal_major_object;
pub mod metadata;
pub mod raster;
pub mod spatial_ref;
mod utils;
pub mod vector;
pub mod version;


#[cfg(test)]
fn assert_almost_eq(a: f64, b: f64) {
    let f: f64 = a / b;
    assert!(f < 1.00001);
    assert!(f > 0.99999);
}

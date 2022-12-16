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
//! GDAL Raster Data

pub use crate::raster::dataset::{Dataset};
pub use crate::raster::driver::Driver;
pub use crate::raster::rasterband::RasterBand;
pub use crate::raster::warp::reproject;

pub mod dataset;
pub mod driver;
pub mod rasterband;
pub mod types;
pub mod warp;
pub mod global_func;

pub use gdal_sys::GDALDataType;

#[cfg(test)]
mod tests;

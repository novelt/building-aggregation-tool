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
/// Convert between GDAL, GEOS, and Rust Geo objects

mod gdal_to_geos;
mod geos_to_gdal;
mod gdal_to_rustgeo;
mod rustgeo_to_gdal;
pub mod traits;

#[cfg(test)]
mod convert_geo;

pub use gdal_to_geos::*;
pub use geos_to_gdal::*;
pub use gdal_to_rustgeo::*;
pub use rustgeo_to_gdal::*;

pub use traits::*;

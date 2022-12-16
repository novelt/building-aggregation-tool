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
#![crate_name = "geos"]
#![crate_type = "lib"]

extern crate c_vec;
#[cfg(any(feature = "geo", feature = "dox"))]
extern crate geo_types;
#[cfg(all(feature = "json"))]
extern crate geojson;
extern crate geos_sys;
extern crate libc;
extern crate anyhow;
extern crate num;
#[cfg(any(feature = "geo", feature = "dox"))]
extern crate wkt;

#[cfg(all(feature = "geo", test))]
#[macro_use]
extern crate doc_comment;

#[cfg(all(feature = "geo", test))]
doctest!("../README.md");

pub(crate) mod functions;


#[cfg(any(feature = "v3_6_0", feature = "dox"))]
pub use enums::Precision;
pub use enums::{
    ByteOrder, CoordDimensions, Dimensions, GeometryTypes, Ordinate, Orientation, OutputDimension,
};

pub use functions::{ version};
pub use simple_wkb_writer::WKBWriter;
pub use simple_wkb_reader::WKBReader;

//#[cfg(any(feature = "geo", feature = "dox"))]

//mod geometry;
//mod prepared_geometry;
//mod spatial_index;

mod enums;
mod simple_wkb_writer;
mod simple_wkb_reader;
mod prepared_geometry;

mod simple_context_handle;
mod simple_geometry;
mod simple_coordinate_sequence;
mod simple_string;

pub use simple_context_handle::*;
pub use simple_geometry::*;
pub use simple_coordinate_sequence::*;
pub use prepared_geometry::*;

//pub use traits::{ContextHandling, ContextInteractions};
//
// #[cfg(test)]
// mod geos_test;

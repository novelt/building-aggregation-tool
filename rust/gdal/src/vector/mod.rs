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
//! GDAL Vector Data
//!


pub use crate::vector::dataset::Dataset;
pub use crate::vector::layer_definition::{LayerDefinition, };
pub use crate::vector::field::{Field, FieldIterator, FieldDefinition, GeomField, geometry_type_to_name, field_type_to_name};
pub use crate::vector::driver::Driver;
pub use crate::vector::feature::{Feature, FieldValue};
pub use crate::vector::geometry::{Geometry};
pub use crate::vector::layer::{FeatureIterator, Layer};
pub use crate::vector::ops::geometry::intersection::Intersection as GeometryIntersection;
pub use gdal_sys::{OGRFieldType, OGRFieldSubType, OGRwkbGeometryType, OGREnvelope};
pub use crate::vector::global_func::*;

//use crate::errors::Result;

mod dataset;
mod layer_definition;
mod driver;
mod feature;
mod geometry;
mod layer;
pub mod ops;
mod field;
mod global_func;

#[cfg(test)]
mod tests;

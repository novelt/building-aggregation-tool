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
use gdal_sys::GDALDataType;
use crate::utils::_string;
use std::ffi::CString;
use std::fmt::Debug;
use num_integer::Integer;
use num_traits::{FromPrimitive, ToPrimitive};

pub trait IntAlias: Copy + Integer + ToPrimitive + FromPrimitive + Debug {}
impl IntAlias for i32 {}
impl IntAlias for u32 {}
impl IntAlias for u16 {}

pub trait GdalType {
    fn gdal_type() -> GDALDataType::Type;

}

impl GdalType for u8 {
    fn gdal_type() -> GDALDataType::Type {
        GDALDataType::GDT_Byte
    }
}
impl GdalType for u16 {
    fn gdal_type() -> GDALDataType::Type {
        GDALDataType::GDT_UInt16
    }
}
impl GdalType for u32 {
    fn gdal_type() -> GDALDataType::Type {
        GDALDataType::GDT_UInt32
    }
}
impl GdalType for i16 {
    fn gdal_type() -> GDALDataType::Type {
        GDALDataType::GDT_Int16
    }
}
impl GdalType for i32 {
    fn gdal_type() -> GDALDataType::Type {
        GDALDataType::GDT_Int32
    }
}
impl GdalType for f32 {
    fn gdal_type() -> GDALDataType::Type {
        GDALDataType::GDT_Float32
    }
}
impl GdalType for f64 {
    fn gdal_type() -> GDALDataType::Type {
        GDALDataType::GDT_Float64
    }
}

pub fn convert_gdal_type_to_string(gdal_type: GDALDataType::Type) -> String {
    let rv = unsafe { gdal_sys::GDALGetDataTypeName(gdal_type) };
    _string(rv)
}

pub fn convert_string_to_gdal_type(name: &str) -> Result<GDALDataType::Type, bool> {
    let rv = unsafe {
        let c_filename = CString::new(name ).unwrap();
        gdal_sys::GDALGetDataTypeByName(c_filename.as_ptr())
    };

    Ok(rv)
}


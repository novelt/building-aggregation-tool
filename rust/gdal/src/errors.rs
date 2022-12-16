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
use libc::c_int;
use std::str::Utf8Error;

use thiserror::Error;
use gdal_sys::{CPLErr, OGRErr, OGRFieldType};


#[derive(Clone, PartialEq, Debug, Error)]
pub enum ErrorKind {
    #[error("FfiNulError")]
    FfiNulError,
    #[error("StrUtf8Error: {0:?}")]
    StrUtf8Error(Utf8Error),
    #[cfg(feature = "ndarray")]
    #[error("NdarrayShapeError")]
    NdarrayShapeError(),
    #[error(
        "CPL error class: '{class:?}', error number: '{number}', error msg: '{msg}'"
    )]
    CplError {
        class: CPLErr::Type,
        number: c_int,
        msg: String,
    },
    #[error(
        "GDAL method '{}' returned a NULL pointer. Error msg: '{}'",
        method_name, msg
    )]
    NullPointer {
        method_name: &'static str,
        msg: String,
    },
    #[error("Can't cast to f64")]
    CastToF64Error,
    #[error("OGR method '{}' returned error: '{:?}'", method_name, err)]
    OgrError {
        err: OGRErr::Type,
        method_name: &'static str,
    },
    #[error(
         "Unhandled type {:?} on OGR method {}",
        field_type, method_name
    )]
    UnhandledFieldType {
        field_type: OGRFieldType::Type,
        method_name: &'static str,
    },
    #[error(
        "Invalid field name '{}' used on method {}",
        field_name, method_name
    )]
    InvalidFieldName {
        field_name: String,
        method_name: &'static str,
    },
    #[error(
        "Invalid field index {} used on method {}",
        index, method_name
    )]
    InvalidFieldIndex {
        index: usize,
        method_name: &'static str,
    },
    #[error("Unlinked Geometry on method {}", method_name)]
    UnlinkedGeometry { method_name: &'static str },
    #[error(
        "Invalid coordinate range while transforming points from {} to {}: {:?}",
        from, to, msg
    )]
    InvalidCoordinateRange {
        from: String,
        to: String,
        msg: Option<String>,
    },
    #[error("Generic Error")]
    GenericError {}
}

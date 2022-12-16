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
use libc::{c_char, c_double, c_void};

#[repr(C)]
pub struct GEOSWKTReader {
    private: [u8; 0],
}
#[repr(C)]
pub struct GEOSWKBReader {
    private: [u8; 0],
}
#[repr(C)]
pub struct GEOSWKTWriter {
    private: [u8; 0],
}
#[repr(C)]
pub struct GEOSWKBWriter {
    private: [u8; 0],
}
#[repr(C)]
pub struct GEOSPreparedGeometry {
    private: [u8; 0],
}
#[repr(C)]
pub struct GEOSCoordSequence {
    private: [u8; 0],
}
#[repr(C)]
pub struct GEOSGeometry {
    private: [u8; 0],
}
#[repr(C)]
pub struct GEOSContextHandle_HS {
    private: [u8; 0],
}
#[repr(C)]
pub struct GEOSSTRtree {
    private: [u8; 0],
}
#[repr(C)]
pub struct GEOSBufferParams {
    private: [u8; 0],
}

#[allow(non_camel_case_types)]
pub type GEOSContextHandle_t = *mut GEOSContextHandle_HS;
#[allow(non_camel_case_types)]
pub type GEOSMessageHandler =
    Option<unsafe extern "C" fn(message: *const c_char, ...)>;
#[allow(non_camel_case_types)]
pub type GEOSMessageHandler_r =
    Option<unsafe extern "C" fn(message: *const c_char, userdata: *mut c_void)>;
#[allow(non_camel_case_types)]
pub type GEOSQueryCallback =
    Option<unsafe extern "C" fn(item: *mut c_void, userdata: *mut c_void)>;
#[allow(non_camel_case_types)]
pub type GEOSDistanceCallback =
    Option<unsafe extern "C" fn(
        item1: *const c_void,
        item2: *const c_void,
        distance: *mut c_double,
        userdata: *mut c_void)>;
#[allow(non_camel_case_types)]
pub type GEOSInterruptCallback =
    Option<unsafe extern "C" fn()>;

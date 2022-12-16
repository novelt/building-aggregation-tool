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
use gdal_sys::{self, CPLErr};
use libc::c_char;
use std::ffi::{CStr};

use crate::errors::*;

pub fn _string(raw_ptr: *const c_char) -> String {
    let c_str = unsafe { CStr::from_ptr(raw_ptr) };
    c_str.to_string_lossy().into_owned()
}

// TODO: inspect if this is sane...
pub fn _last_cpl_err(cpl_err_class: CPLErr::Type) -> ErrorKind {
    let last_err_no = unsafe { gdal_sys::CPLGetLastErrorNo() };
    let last_err_msg = _string(unsafe { gdal_sys::CPLGetLastErrorMsg() });
    unsafe { gdal_sys::CPLErrorReset() };
    ErrorKind::CplError {
        class: cpl_err_class,
        number: last_err_no,
        msg: last_err_msg,
    }
}

pub fn _last_null_pointer_err(method_name: &'static str) -> ErrorKind {
    let last_err_msg = _string(unsafe { gdal_sys::CPLGetLastErrorMsg() });
    unsafe { gdal_sys::CPLErrorReset() };
    ErrorKind::NullPointer {
        method_name,
        msg: last_err_msg,
    }
}


/*
pub fn convert_to_const_c_string_list(string_slice: &[&str]) -> Vec<*const libc::c_char> {

    let c_strings: Vec<CString> = string_slice.into_iter().map(|s| CString::new(*s).unwrap()).collect();
    //Need the strings as const* const* i8 for gdal, so just cast the char* string (both are 1 byte)
    let mut c_as_i8: Vec<*const libc::c_char> = c_strings.iter().map(|cs| cs.as_ptr() as *const libc::c_char).collect();

    //null terminate the list
    c_as_i8.push(0 as *const libc::c_char);

    c_as_i8
}*/

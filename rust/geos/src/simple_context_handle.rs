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
use geos_sys::*;
use std::ffi::CStr;
use std::os::raw::{c_char, c_void};
use std::ptr::null_mut;


unsafe extern "C" fn message_handler_func(
                message: *const c_char,
                _data: *mut c_void,
            ) {
    let s = CStr::from_ptr(message);
    println!("Recieved message: {}", s.to_string_lossy());
}


pub struct SimpleContextHandle {
    pub(crate) c_handle: GEOSContextHandle_t
}

impl SimpleContextHandle {
    pub fn new() -> Self {
        unsafe {
            Self {
                c_handle: GEOS_init_r()
            }
        }
    }

    pub fn add_message_handlers(&self) {
        unsafe {
            GEOSContext_setNoticeMessageHandler_r(self.c_handle, Some(message_handler_func), null_mut() );

            GEOSContext_setErrorMessageHandler_r(self.c_handle, Some(message_handler_func), null_mut() );
        }
    }
}

impl Drop for SimpleContextHandle {
    fn drop(&mut self) {
        unsafe {
            //println!("Dropping simple context handler");
            GEOS_finish_r(self.c_handle);
        }
    }
}
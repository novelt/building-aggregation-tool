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
use ::{SimpleContextHandle, SimpleGeometry};
use anyhow::{bail, Result};

pub struct WKBReader<'c> {
    pub(crate) c_handle: *mut GEOSWKBReader,
    pub(crate) context_handle: &'c SimpleContextHandle
}

impl<'c> WKBReader<'c> {
    /// Creates a new `WKBReader` instance.
    ///
    /// # Example
    ///
    /// ```

    /// ```
    pub fn new(context: &'c SimpleContextHandle) -> Result<WKBReader<'c>> {
        unsafe {
            let ptr = GEOSWKBReader_create_r(context.c_handle);
            
            if ptr.is_null() {
                bail!("GEOSWKBReader_create_r");
            }
            
            Ok(WKBReader {
                c_handle: ptr,
                context_handle: context
            })
            
        }
    }

    

    /// Writes out the given `geometry` as WKB format.
    ///
    /// # Example
    ///
    /// ```

    /// ```
    pub fn read_wkb(&self, bytes: &[u8]) -> Result<SimpleGeometry> {

        unsafe {
            let w_ptr = GEOSWKBReader_read_r(
                self.context_handle.c_handle,
                self.c_handle,
                bytes.as_ptr(),
                bytes.len()
            );
            if w_ptr.is_null() {
                bail!(
                    "WKBReader::write_wkb failed: GEOSWKBReader_writeHEX_r returned null pointer"
                );
            }

            Ok(SimpleGeometry{
                c_handle: w_ptr,
                owned: true,
                context_handle: self.context_handle
            })

        }
    }




}


impl<'a> Drop for WKBReader<'a> {
    fn drop(&mut self) {
        unsafe { GEOSWKBReader_destroy_r(self.context_handle.c_handle, self.c_handle) };    }
}


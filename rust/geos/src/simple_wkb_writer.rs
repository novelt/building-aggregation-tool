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
use c_vec::CVec;
use enums::TryFrom;
use enums::{ByteOrder, OutputDimension};
use geos_sys::*;
use ::{SimpleContextHandle, SimpleGeometry};
use anyhow::{bail, Result};

/// The `WKBWriter` type is used to generate `HEX` or `WKB` formatted output from [`Geometry`].
///
/// # Example
///
/// ```
/// ```
pub struct WKBWriter<'c> {
    pub(crate) c_handle: *mut GEOSWKBWriter,
    pub(crate) context_handle: &'c SimpleContextHandle
}

impl<'c> WKBWriter<'c> {
    /// Creates a new `WKBWriter` instance.
    ///
    /// # Example
    ///
    /// ```

    /// ```
    pub fn new(context: &'c SimpleContextHandle) -> Result<WKBWriter<'c>> {
        unsafe {
            let ptr = GEOSWKBWriter_create_r(context.c_handle);
            
            if ptr.is_null() {
                bail!("GEOSWKBWriter_create_r");
            }
            
            Ok(WKBWriter {
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
    pub fn write_wkb(&self, geometry: &SimpleGeometry) -> Result<CVec<u8>> {
        let mut size = 0;
        unsafe {
            let w_ptr = GEOSWKBWriter_write_r(
                self.context_handle.c_handle,
                self.c_handle,
                geometry.c_handle,
                &mut size,
            );
            if w_ptr.is_null() {
                bail!(
                    "WKBWriter::write_wkb failed: GEOSWKBWriter_writeHEX_r returned null pointer"
                );
            }

            let c_context_handle = self.context_handle.c_handle;

            let c_vec = CVec::new_with_dtor(w_ptr,
            size, move |c_vet_ptr| {
                   //println!("Running v cet destructor");
                   GEOSFree_r(c_context_handle, c_vet_ptr as _);
                });

            Ok(c_vec)

        }
    }



    /// Sets the number of dimensions to be used when calling [`WKBWriter::write_wkb`] or
    /// [`WKBWriter::write_hex`]. By default, it is 2.
    ///

    /// ```
    pub fn set_output_dimension(&mut self, dimension: OutputDimension) {
        unsafe {
            GEOSWKBWriter_setOutputDimension_r(
                self.context_handle.c_handle,
                self.c_handle,
                dimension.into(),
            )
        }
    }

    /// Returns the number of dimensions to be used when calling [`WKBWriter::write`]. By default,
    /// it is 2.
    ///
    /// ```
    pub fn get_out_dimension(&self) -> Result<OutputDimension> {
        unsafe {
            let out = GEOSWKBWriter_getOutputDimension_r(self.context_handle.c_handle, self.c_handle);
            Ok(OutputDimension::try_from(out).unwrap())
        }
    }

    /// Gets WKB byte order.
    ///
    /// # Example
    ///
    /// ```
    pub fn get_wkb_byte_order(&self) -> Result<ByteOrder> {
        unsafe {
            let out = GEOSWKBWriter_getByteOrder_r(self.context_handle.c_handle, self.c_handle);
            Ok(ByteOrder::try_from(out).unwrap())
        }
    }

    /// Sets WKB byte order.
    ///
    /// # Example
    ///
    /// ```
    pub fn set_wkb_byte_order(&mut self, byte_order: ByteOrder) {
        unsafe {
            GEOSWKBWriter_setByteOrder_r(
                self.context_handle.c_handle, self.c_handle,
                byte_order.into(),
            )
        }
    }

    /// Gets if output will include SRID.
    ///
    /// # Example
    ///
    /// ```
    #[allow(non_snake_case)]
    pub fn get_include_SRID(&self) -> Result<bool> {
        unsafe {
            let out = GEOSWKBWriter_getIncludeSRID_r(self.context_handle.c_handle, self.c_handle);
            if out < 0 {
                bail!(
                    "GEOSWKBWriter_getIncludeSRID_r failed"
                );
            } else {
                Ok(out != 0)
            }
        }
    }

    /// Sets if output will include SRID.
    ///
    /// # Example
    ///
    /// ```
    /// ```
    #[allow(non_snake_case)]
    pub fn set_include_SRID(&mut self, include_SRID: bool) {
        unsafe {
            GEOSWKBWriter_setIncludeSRID_r(
                self.context_handle.c_handle,
                self.c_handle,
                include_SRID as _,
            )
        }
    }
}


impl<'a> Drop for WKBWriter<'a> {
    fn drop(&mut self) {
        unsafe { GEOSWKBWriter_destroy_r(self.context_handle.c_handle, self.c_handle) };    }
}


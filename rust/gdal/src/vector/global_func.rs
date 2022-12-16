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
use gdal_sys::{GDALVectorTranslateOptionsNew, GDALVectorTranslateOptionsFree, GDALVectorTranslate, GDALDatasetH, GDALOpenEx};
use std::ptr::null_mut;
use std::fmt::Debug;
use std::ffi::CString;
use crate::utils::_last_null_pointer_err;

use anyhow::Result;

pub const GDAL_OF_READONLY : u32 = 0x00;
pub const GDAL_OF_UPDATE : u32 = 0x01;
pub const GDAL_OF_ALL : u32 = 0x00;
pub const GDAL_OF_RASTER : u32 = 0x02;
pub const GDAL_OF_VECTOR : u32 = 0x04;
pub const GDAL_OF_GNM : u32 = 0x08;
pub const GDAL_OF_KIND_MASK : u32 = 0x1E;
pub const GDAL_OF_SHARED : u32 = 0x20;
pub const GDAL_OF_VERBOSE_ERROR: u32 = 0x40;

pub fn translate<T>(src: &str, dst: &str, options: &[ T ]) -> Result<()>
where T: AsRef<str> + Debug
{

	unsafe {
		println!("Calling ogr2ogr / vector translate to {:?} with options {:?}", dst, options);

		let src_cstr = CString::new(src)?;

		let src_ds = GDALOpenEx(src_cstr.as_ptr(),
                         GDAL_OF_VECTOR, null_mut(),
                         null_mut(), null_mut());

		let mut vec_ds : Vec<GDALDatasetH> = Vec::new();
		vec_ds.push(src_ds);
		vec_ds.push(0 as GDALDatasetH);

		//do this locally since we don't want the CStrings to be deallocated until this function ends
		let c_strings: Vec<CString> = options.into_iter().map(|s| CString::new(s.as_ref()).unwrap()).collect();
		//Need the strings as const* const* i8 for gdal, so just cast the char* string (both are 1 byte)
		let mut c_as_i8: Vec<*mut libc::c_char> = c_strings.iter().map(|cs| cs.as_ptr() as *mut libc::c_char).collect();

		//null terminate the list
		c_as_i8.push(0 as *mut libc::c_char);

		//println!("Creating GDALVectorTranslateOptionsNew options");
		let ogr2ogr_options = GDALVectorTranslateOptionsNew(
			c_as_i8.as_mut_ptr(),
			null_mut()
		);

		if ogr2ogr_options.is_null() {
			println!("Error with rasterize options");
            Err(_last_null_pointer_err("GDALVectorTranslateOptionsNew"))?;
        }

		let mut usage_error: i32 = 0;

        let dst_cstr = CString::new(dst)?;

		//println!("GDALVectorTranslate");
		let c_dataset = GDALVectorTranslate(
			dst_cstr.as_ptr(),
			null_mut(),
			1,
			//pointer to an array and a pointer are the same thing
			vec_ds.as_mut_ptr(),
			ogr2ogr_options,
			&mut usage_error as *mut libc::c_int,
		);

		//println!("Usage error: {}", usage_error);
		//println!("GDAL options free");
		GDALVectorTranslateOptionsFree(ogr2ogr_options);
		//println!("Done GDAL options free");

		if c_dataset.is_null() {
			println!("Error with ogr2ogr");
            Err(_last_null_pointer_err("GDALVectorTranslate"))?;
        }


		//assert_eq!(c_dataset, dst._c_ptr(), "Returned dataset should be the same as the opened dst dataset");

		//Note, because these are the same, when dst is dropped at the end of this function, the dataset will be closed properly
		//thus there is no need to call gdal_sys::GDALClose(c_dataset);

		//println!("Returning");
        Ok(())
	}

}
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
use gdal_sys::{GDALGetDataTypeName, GDALDataType, GDALRasterize, GDALRasterizeOptionsNew, GDALRasterizeOptionsFree, GDALWarpAppOptionsNew, GDALWarp, GDALWarpAppOptionsFree};
use anyhow::Result as GdalResult;
use crate::utils::{_string, _last_null_pointer_err};
use crate::raster::{Dataset as RasterDataset, Dataset};
use crate::vector::Dataset as VectorDataset;
use std::ptr::{null_mut,};
use crate::gdal_major_object::MajorObject;
use std::path::Path;
use std::ffi::CString;
use std::fmt::Debug;
use log::{debug,};

pub fn get_type_name(gdal_type: GDALDataType::Type) -> GdalResult<String> {
	let c_res = unsafe { GDALGetDataTypeName(gdal_type)};
	if c_res.is_null() {
		Err(_last_null_pointer_err("GDALGetDescription"))?;
	}
	Ok(_string(c_res))
}

pub fn rasterize<T>(src: &VectorDataset, dst_path: &Path, options: &[ T ]) -> GdalResult<()>
where T: AsRef<str> + Debug
{
	debug!("Calling rasterize to {:?} with options {:?}", dst_path, options);

	let dst = RasterDataset::open(&dst_path, false)?;

	return rasterize_dataset(src, &dst, options);

}

pub fn rasterize_dataset<T>(src: &VectorDataset, dst: &Dataset, options: &[ T ]) -> GdalResult<()>
where T: AsRef<str> + Debug
{

	unsafe {
		//println!("Calling rasterize with options {:?}", options);

		//do this locally since we don't want the CStrings to be deallocated until this function ends
		let c_strings: Vec<CString> = options.into_iter().map(|s| CString::new(s.as_ref()).unwrap()).collect();
		//Need the strings as const* const* i8 for gdal, so just cast the char* string (both are 1 byte)
		let mut c_as_i8: Vec<*mut libc::c_char> = c_strings.iter().map(|cs| cs.as_ptr() as *mut libc::c_char).collect();

		//null terminate the list
		c_as_i8.push(0 as *mut libc::c_char);

		//println!("Creating rasterization options");
		let gdal_rasterize_options = GDALRasterizeOptionsNew(
			c_as_i8.as_mut_ptr(),
			null_mut()
		);

		if gdal_rasterize_options.is_null() {
			println!("Error with rasterize options");
            Err(_last_null_pointer_err("GDALRasterizeOptionsNew"))?;
        }

		let mut usage_error: i32 = 0;

		//println!("GDAL rasterize");
		let c_dataset = GDALRasterize(
			null_mut(),
			dst._c_ptr(),
			src.gdal_object_ptr(),
			gdal_rasterize_options,
			&mut usage_error as *mut libc::c_int,
		);

		// println!("Usage error: {}", usage_error);
		// println!("GDAL options free");
		GDALRasterizeOptionsFree(gdal_rasterize_options);
		// println!("Done GDAL options free");

		if c_dataset.is_null() {
			println!("Error with gdal rasterize");
            Err(_last_null_pointer_err("GDALRasterize"))?;
        }

		//println!("Closing dataset");

		assert_eq!(c_dataset, dst._c_ptr(), "Returned dataset should be the same as the opened dst dataset");

		//Note, because these are the same, when dst is dropped at the end of this function, the dataset will be closed properly
		//thus there is no need to call gdal_sys::GDALClose(c_dataset);

		//println!("Returning");
        Ok(())
	}

}


pub fn warp<T>(src: &Path, dst_path: &Path, options: &[ T ]) -> GdalResult<()>
where T: AsRef<str> + Debug
{

	unsafe {
		println!("Calling warp to {:?} with options {:?}", dst_path, options);

		let src = RasterDataset::open(&src, true)?;

		assert!(dst_path.parent().unwrap().exists());

		//do this locally since we don't want the CStrings to be deallocated until this function ends
		let c_strings: Vec<CString> = options.into_iter().map(|s| CString::new(s.as_ref()).unwrap()).collect();
		//Need the strings as const* const* i8 for gdal, so just cast the char* string (both are 1 byte)
		let mut c_as_i8: Vec<*mut libc::c_char> = c_strings.iter().map(|cs| cs.as_ptr() as *mut libc::c_char).collect();

		//null terminate the list
		c_as_i8.push(0 as *mut libc::c_char);

		let dst_dataset = dst_path.to_str().unwrap().to_string();
		let dst_dataset_cstr = CString::new(dst_dataset).unwrap();

		println!("Creating rasterization options");
		let gdal_warp_options = GDALWarpAppOptionsNew(
			c_as_i8.as_mut_ptr(),
			null_mut()
		);

		if gdal_warp_options.is_null() {
			println!("Error with warp options");
            Err(_last_null_pointer_err("GDALWarpAppOptionsNew"))?;
        }

		let mut usage_error: i32 = 0;

		println!("GDAL rasterize");
		let c_dataset = GDALWarp(
			dst_dataset_cstr.as_ptr(),
			null_mut(),
			1,
			vec![src._c_ptr()].as_mut_ptr(),
			gdal_warp_options,
			&mut usage_error as *mut libc::c_int,
		);

		println!("Usage error: {}", usage_error);
		println!("GDAL options free");
		GDALWarpAppOptionsFree(gdal_warp_options);
		println!("Done GDAL options free");

		if c_dataset.is_null() {
			println!("Error with gdal rasterize");
            Err(_last_null_pointer_err("GDALRasterize"))?;
        }

		println!("Closing dataset");

		//assert_eq!(c_dataset, dst._c_ptr(), "Returned dataset should be the same as the opened dst dataset");

		//Note, because these are the same, when dst is dropped at the end of this function, the dataset will be closed properly
		//thus there is no need to call gdal_sys::GDALClose(c_dataset);

		println!("Returning");
        Ok(())
	}

}
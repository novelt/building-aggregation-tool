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
use core::ptr;
use crate::gdal_major_object::MajorObject;
use crate::metadata::Metadata;
use crate::raster::types::GdalType;
use crate::raster::Dataset;
use crate::utils::{_last_null_pointer_err, _string};
use gdal_sys::{self, GDALDriverH, GDALMajorObjectH, GDALDataType};
use libc::c_int;
use std::ffi::{CString};
use std::sync::Once;

use anyhow::Result;
use std::path::Path;
use std::fs::create_dir_all;

static START: Once = Once::new();

pub fn _register_drivers() {
    unsafe {
        START.call_once(|| {
            gdal_sys::GDALAllRegister();
        });
    }
}

#[allow(missing_copy_implementations)]
pub struct Driver {
    c_driver: GDALDriverH,
}

pub const GTIFF_DRIVER: &str = "GTiff";
pub const MEM_DRIVER: &str = "MEM";

pub const DEFAULT_RASTER_OPTIONS: [&str; 4] = [
                "TILED=YES",
                "BLOCKXSIZE=128",
                "BLOCKYSIZE=128",
                "COMPRESS=LZW",];

impl Driver {
    pub fn get(name: &str) -> Result<Driver> {
        _register_drivers();
        let c_name = CString::new(name)?;
        let c_driver = unsafe { gdal_sys::GDALGetDriverByName(c_name.as_ptr()) };
        if c_driver.is_null() {
            Err(_last_null_pointer_err("GDALGetDriverByName"))?;
        };
        Ok(Driver { c_driver })
    }

    pub unsafe fn _with_c_ptr(c_driver: GDALDriverH) -> Driver {
        Driver { c_driver }
    }

    pub unsafe fn _c_ptr(&self) -> GDALDriverH {
        self.c_driver
    }

    pub fn short_name(&self) -> String {
        let rv = unsafe { gdal_sys::GDALGetDriverShortName(self.c_driver) };
        _string(rv)
    }

    pub fn long_name(&self) -> String {
        let rv = unsafe { gdal_sys::GDALGetDriverLongName(self.c_driver) };
        _string(rv)
    }

    pub fn create(
        &self,
        filename: &str,
        size_x: isize,
        size_y: isize,
        bands: isize,
    ) -> Result<Dataset> {
        self.create_with_band_type::<&str, _>(filename, size_x, size_y, bands,
                                   u8::gdal_type(),
                                   &[] )
    }

    pub fn create_in_memory(&self, size_x: isize,
        size_y: isize,
    ) -> Result<Dataset> {
        unsafe {


            let dummy_name = CString::new("").unwrap();
            let c_dataset =
                gdal_sys::GDALCreate(
                    self.c_driver,
                    dummy_name.as_ptr(),
                    size_x as c_int,
                    size_y as c_int,
                    0 as c_int,
                    u8::gdal_type(),
                    ptr::null_mut()
                );


            if c_dataset.is_null() {
                Err(_last_null_pointer_err("GDALCreate"))?;
            };
            Ok(  Dataset::_with_c_ptr(c_dataset) )
        }
    }

    ///
    /// Options must be in form ["key1=val1", "key2=val2"]
    pub fn create_with_band_type<T, P>(
        &self,
        filename: P,
        size_x: isize,
        size_y: isize,
        bands: isize,
        gdal_type: GDALDataType::Type,
        create_options: &[ T ]
    ) -> Result<Dataset>
    where T: AsRef<str>, P: AsRef<Path>
    {
        if let Some(par_dir) = filename.as_ref().parent()
        {
            if !par_dir.is_dir() {
                create_dir_all(par_dir).expect("Creating parent raster directory");
            }
        }

        let c_filename = CString::new(filename.as_ref().to_str().expect("Path is not a string") )?;

        //do this locally since we don't want the CStrings to be deallocated until this function ends
		let c_strings: Vec<CString> = create_options.into_iter().map(|s| CString::new(s.as_ref()).unwrap()).collect();
		//Need the strings as const* const* i8 for gdal, so just cast the char* string (both are 1 byte)
		let mut c_options: Vec<*mut libc::c_char> = c_strings.iter().map(|cs| cs.as_ptr() as *mut libc::c_char).collect();

		//null terminate the list
		c_options.push(0 as _);

        let c_dataset = unsafe {
            gdal_sys::GDALCreate(
                self.c_driver,
                c_filename.as_ptr(),
                size_x as c_int,
                size_y as c_int,
                bands as c_int,
                gdal_type,
                c_options.as_mut_ptr()
                //c_options as *const *const libc::c_char ,
            )
        };

        if c_dataset.is_null() {
            Err(_last_null_pointer_err("GDALCreate"))?;
        };
        Ok(unsafe { Dataset::_with_c_ptr(c_dataset) })
    }
}

impl MajorObject for Driver {
    unsafe fn gdal_object_ptr(&self) -> GDALMajorObjectH {
        self.c_driver
    }
}

impl Metadata for Driver {}

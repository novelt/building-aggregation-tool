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
use crate::utils::_last_null_pointer_err;
use crate::vector::{Dataset, GDAL_OF_VECTOR, GDAL_OF_READONLY, GDAL_OF_UPDATE, GDAL_OF_VERBOSE_ERROR};
use gdal_sys::{self, OGRSFDriverH};
use std::ffi::CString;
use std::ptr::null_mut;
use std::sync::Once;

use anyhow::Result;

static START: Once = Once::new();

pub fn _register_drivers() {
    unsafe {
        START.call_once(|| {
            gdal_sys::OGRRegisterAll();
        });
    }
}

pub struct Driver {
    c_driver: OGRSFDriverH,

}

impl Driver {
    pub const DRIVER_NAME_SHAPEFILE : &'static str = "ESRI Shapefile";
    pub const DRIVER_NAME_GEOPACKAGE : &'static str = "GPKG";
    pub const DRIVER_NAME_GEOJSON : &'static str = "GeoJSON";
    pub const DRIVER_NAME_FILEGDB : &'static str = "FileGDB";
    pub const DRIVER_NAME_FLATGEOBUF : &'static str = "FlatGeobuf";
    pub const DRIVER_NAME_POSTGRESQL : &'static str = "PostgreSQL";
    pub const DRIVER_NAME_MEMORY : &'static str = "Memory";

    pub fn get(name: &str) -> Result<Driver> {
        _register_drivers();
        let c_name = CString::new(name)?;
        let c_driver = unsafe { gdal_sys::OGRGetDriverByName(c_name.as_ptr()) };
        if c_driver.is_null() {
            Err(_last_null_pointer_err("OGRGetDriverByName"))?
        } else {
            Ok(Driver { c_driver })
        }
    }

    pub fn create<T>(&self, path: T) -> Result<Dataset>
    where T: AsRef<str>
    {
        let c_filename = CString::new(path.as_ref())?;
        let c_dataset = unsafe {
            gdal_sys::OGR_Dr_CreateDataSource(self.c_driver, c_filename.as_ptr(), null_mut())
        };
        if c_dataset.is_null() {
            Err(_last_null_pointer_err("OGR_Dr_CreateDataSource"))?
        } else {
            Ok(unsafe { Dataset::_with_c_dataset(c_dataset) })
        }
    }

    pub fn open<T>(&self, path: T, read_only: bool) -> Result<Dataset>
    where T: AsRef<str>
    {
        let c_filename = CString::new(path.as_ref())?;
        let c_dataset = unsafe {
            gdal_sys::OGR_Dr_Open(self.c_driver, c_filename.as_ptr(), if read_only {0} else {1})
        };
        if c_dataset.is_null() {
            Err(_last_null_pointer_err("OGR_Dr_Open"))?
        } else {
            Ok(unsafe { Dataset::_with_c_dataset(c_dataset) })
        }

    }

    pub fn open_vector_static<T>(conn_str: T, read_only: bool, open_options: &[String]) -> Result<Dataset>
    where T: AsRef<str>
    {
        _register_drivers();
        let mut flags = GDAL_OF_VECTOR;

        if read_only {
            flags |= GDAL_OF_READONLY;
        } else {
            flags |= GDAL_OF_UPDATE;
        }

        flags |= GDAL_OF_VERBOSE_ERROR;

        //Add any drivers this needs to work with here
        let drivers = [
            Driver::DRIVER_NAME_FLATGEOBUF,
            Driver::DRIVER_NAME_GEOJSON,
            Driver::DRIVER_NAME_POSTGRESQL,
            Driver::DRIVER_NAME_SHAPEFILE,
            Driver::DRIVER_NAME_FILEGDB
        ];

        let driver_strings: Vec<CString> = drivers.iter().map(|s| CString::new(s.to_string()).unwrap()).collect();
        let mut driver_ptrs: Vec<*const libc::c_char> = driver_strings.iter().map(|cs| cs.as_ptr() as *const libc::c_char).collect();
        driver_ptrs.push(0 as *mut libc::c_char);

        //do this locally since we don't want the CStrings to be deallocated until this function ends
		let c_strings: Vec<CString> = open_options.into_iter().map(|s| CString::new(s.as_str()).unwrap()).collect();
		//Need the strings as const* const* i8 for gdal, so just cast the char* string (both are 1 byte)
		let mut c_options: Vec<*const libc::c_char> = c_strings.iter().map(|cs| cs.as_ptr() as *const libc::c_char).collect();

		//null terminate the list
		c_options.push(0 as *mut libc::c_char);

        let c_ogr_conn_str = CString::new(conn_str.as_ref())?;

        let c_dataset = unsafe {
            gdal_sys::GDALOpenEx(c_ogr_conn_str.as_ptr(),
                         flags, driver_ptrs.as_ptr(),
                         c_options.as_ptr(),
                                 null_mut())
        };
        if c_dataset.is_null() {
            Err(_last_null_pointer_err("GDALOpenEx"))?
        } else {
            Ok(unsafe { Dataset::_with_c_dataset(c_dataset) })
        }
    }
}

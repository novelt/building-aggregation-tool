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
use anyhow::Result;
use crate::gdal_major_object::MajorObject;
use crate::utils::{_last_cpl_err, _last_null_pointer_err, _string};
use gdal_sys::{self, CPLErr};
use std::ffi::CString;

pub trait Metadata: MajorObject {
    fn description(&self) -> Result<String> {
        let c_res = unsafe { gdal_sys::GDALGetDescription(self.gdal_object_ptr()) };
        if c_res.is_null() {
            Err(_last_null_pointer_err("GDALGetDescription"))?;
        }
        Ok(_string(c_res))
    }

    fn metadata_item(&self, key: &str, domain: &str) -> Option<String> {
        if let Ok(c_key) = CString::new(key.to_owned()) {
            if let Ok(c_domain) = CString::new(domain.to_owned()) {
                let c_res = unsafe {
                    gdal_sys::GDALGetMetadataItem(
                        self.gdal_object_ptr(),
                        c_key.as_ptr(),
                        c_domain.as_ptr(),
                    )
                };
                if !c_res.is_null() {
                    return Some(_string(c_res));
                }
            }
        }
        None
    }

    fn set_metadata_item(&mut self, key: &str, value: &str, domain: &str) -> Result<()> {
        let c_key = CString::new(key.to_owned())?;
        let c_domain = CString::new(domain.to_owned())?;
        let c_value = CString::new(value.to_owned())?;

        let c_res = unsafe {
            gdal_sys::GDALSetMetadataItem(
                self.gdal_object_ptr(),
                c_key.as_ptr(),
                c_value.as_ptr(),
                c_domain.as_ptr(),
            )
        };
        if c_res != CPLErr::CE_None {
            Err(_last_cpl_err(c_res))?;
        }
        Ok(())
    }
}

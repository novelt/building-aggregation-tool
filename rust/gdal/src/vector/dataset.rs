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
use crate::gdal_major_object::MajorObject;
use crate::metadata::Metadata;
use crate::spatial_ref::SpatialRef;
use crate::utils::{_last_null_pointer_err};
use crate::vector::driver::_register_drivers;
use crate::vector::Layer;
use gdal_sys::{self, GDALMajorObjectH, OGRDataSourceH, OGRwkbGeometryType};
use libc::c_int;
use std::ffi::CString;
use std::ptr::{null, null_mut};

use anyhow::Result;

/// Vector dataset
///

pub struct Dataset {
    pub(crate) c_dataset: OGRDataSourceH,
}

impl MajorObject for Dataset {
    unsafe fn gdal_object_ptr(&self) -> GDALMajorObjectH {
        self.c_dataset
    }
}

impl Metadata for Dataset {}

impl Dataset {
    pub(crate) unsafe fn _with_c_dataset(c_dataset: OGRDataSourceH) -> Dataset {
        Dataset {
            c_dataset,
        }
    }

    /// Open the dataset
    pub fn open<T>(dataset: T) -> Result<Dataset>
        where T: AsRef<str>
    {
        Dataset::open_rw(dataset, true)
    }
    pub fn open_rw<T>(dataset: T, read_only: bool) -> Result<Dataset>
        where T: AsRef<str>
    {
        _register_drivers();
        //println!("Open dataset {}", dataset.as_ref());
        let c_dataset_str = CString::new(dataset.as_ref())?;
        let c_dataset = unsafe { gdal_sys::OGROpen(c_dataset_str.as_ptr(), if read_only {0} else {1}, null_mut()) };

        if c_dataset.is_null() {
            Err(_last_null_pointer_err("OGROpen"))?;
        };
        Ok(Dataset {
            c_dataset,
        })
    }

    /// Get number of layers.
    pub fn count(&self) -> isize {
        (unsafe { gdal_sys::OGR_DS_GetLayerCount(self.c_dataset) }) as isize
    }

    /*
    fn _child_layer(&mut self, c_layer: OGRLayerH) -> &Layer {
        let layer = unsafe { Layer::_with_c_layer(c_layer) };
        self.layers.push(layer);
        self.layers.last().unwrap()
    }*/

    pub fn layer_by_sql(&self, ogr_sql: &str, use_ogr_sql: bool) -> Result<Layer> {
        unsafe {
            let cstr_ogr_sql = CString::new(ogr_sql)?;
            let cstr_dialect = CString::new("OGRSQL")?;

            //let lyr = gdal_sys::GDALDatasetExecuteSQL(
            //This is an owned layer and must be destroyed
            let lyr = gdal_sys::OGR_DS_ExecuteSQL(
                self.c_dataset,
                cstr_ogr_sql.as_ptr(),
                null_mut(),
                if use_ogr_sql {
                    cstr_dialect.as_ptr()
                } else {
                    null()
                }
            );


            Ok( Layer{
                c_layer: lyr,
                //pass reference to ensure dataset is deleted after the layer
                _dataset: self,
                //Delete when dropped
                owned: true
            })
        }
    }

    /// Get layer number `idx`.
    pub fn layer(&self, idx: isize) -> Result<Layer> {
        //No delete is needed, but we want the dataset to live longer
        let c_layer = unsafe { gdal_sys::OGR_DS_GetLayer(self.c_dataset, idx as c_int) };
        if c_layer.is_null() {
            Err(_last_null_pointer_err("OGR_DS_GetLayer"))?;
        }
        Ok(Layer {
            c_layer,
            _dataset: self,
            owned: false
        })
    }

    /// Get layer with `name`.
    pub fn layer_by_name(&self, name: &str) -> Result<Layer> {
        let c_name = CString::new(name)?;
        //No delete is needed
        let c_layer = unsafe { gdal_sys::OGR_DS_GetLayerByName(self.c_dataset, c_name.as_ptr()) };
        if c_layer.is_null() {
            Err(_last_null_pointer_err("OGR_DS_GetLayerByName"))?;
        }
        Ok(Layer {
            c_layer,
            _dataset: self,
            owned: false
        })
    }

    // Create a new layer with a blank definition.
    pub fn create_layer(&mut self) -> Result<Layer> {
        let c_name = CString::new("")?;
        let c_layer = unsafe {
            gdal_sys::OGR_DS_CreateLayer(
                self.c_dataset,
                c_name.as_ptr(),
                null_mut(),
                OGRwkbGeometryType::wkbUnknown,
                null_mut(),
            )
        };
        if c_layer.is_null() {
            Err(_last_null_pointer_err("OGR_DS_CreateLayer"))?;
        };

        //Appears like dataset will handle cleanup, got a seg fault when owned was true
        Ok(Layer {
            c_layer,
            _dataset: self,
            //do not drop when out of scope
            owned: false
        })
    }


    /// Create a new layer with name, spatial ref. and type.
    pub fn create_layer_ext<T>(
        &self,
        name: &str,
        srs: &SpatialRef,
        ty: OGRwkbGeometryType::Type,
        create_options: &[ T ]
    ) -> Result<Layer>
    where T: AsRef<str>
    {
        let c_name = CString::new(name)?;

        //do this locally since we don't want the CStrings to be deallocated until this function ends
		let c_strings: Vec<CString> = create_options.into_iter().map(|s| CString::new(s.as_ref()).unwrap()).collect();
		//Need the strings as const* const* i8 for gdal, so just cast the char* string (both are 1 byte)
		let mut c_options: Vec<*mut libc::c_char> = c_strings.iter().map(|cs| cs.as_ptr() as *mut libc::c_char).collect();

		//null terminate the list
		c_options.push(0 as *mut libc::c_char);

        let c_layer = unsafe {
            gdal_sys::OGR_DS_CreateLayer(self.c_dataset, c_name.as_ptr(), srs.c_spatial_ref, ty, c_options.as_mut_ptr())
        };

        if c_layer.is_null() {
            Err(_last_null_pointer_err("OGR_DS_CreateLayer"))?;
        };
        Ok( Layer {
                c_layer,
                _dataset: self,
                owned: false
            }
        )
    }
}

impl Drop for Dataset {
    fn drop(&mut self) {
        unsafe {
            //println!("Calling destroy dataset");
            gdal_sys::OGR_DS_Destroy(self.c_dataset);
            //println!("Done Calling destroy dataset");
        }
    }
}

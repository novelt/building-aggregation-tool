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
use crate::raster::driver::_register_drivers;
use crate::raster::{Driver, RasterBand};
use crate::utils::{_last_cpl_err, _last_null_pointer_err, _string};
use gdal_sys::{self, CPLErr, GDALAccess, GDALDataType, GDALDatasetH, GDALMajorObjectH, CPLPrintPointer};
use libc::{c_double, c_int};
use std::ffi::{c_void, CStr, CString};
use std::path::Path;
use std::ptr::null_mut;

use anyhow::Result;
use num_integer::Integer;
use num_traits::NumCast;
use crate::raster::GDALDataType::GDT_Byte;
use crate::spatial_ref::SpatialRef;

pub type GeoTransform = [c_double; 6];

pub struct Dataset {
    c_dataset: GDALDatasetH,
}

impl MajorObject for Dataset {
    unsafe fn gdal_object_ptr(&self) -> GDALMajorObjectH {
        self.c_dataset
    }
}

impl Metadata for Dataset {}

impl Drop for Dataset {
    fn drop(&mut self) {
        unsafe {
            gdal_sys::GDALClose(self.c_dataset);
        }
    }
}

impl Dataset {
    pub fn open(path: &Path, readonly: bool) -> Result<Dataset> {
        _register_drivers();
        let filename = path.to_string_lossy();
        let c_filename = CString::new(filename.as_ref())?;
        let c_dataset = unsafe { gdal_sys::GDALOpen(c_filename.as_ptr(),
                                                    if readonly {
                                                        GDALAccess::GA_ReadOnly
                                                    } else {
                                                    GDALAccess::GA_Update})
                                                    };
        if c_dataset.is_null() {
            Err(_last_null_pointer_err("GDALOpen"))?;
        }
        Ok(Dataset { c_dataset })
    }

    pub unsafe fn _with_c_ptr(c_dataset: GDALDatasetH) -> Dataset {
        Dataset { c_dataset }
    }

    pub unsafe fn _c_ptr(&self) -> GDALDatasetH {
        self.c_dataset
    }

    // Note band_index is 1 based !
    pub fn rasterband(&self, band_index: isize) -> Result<RasterBand> {
        unsafe {
            let c_band = gdal_sys::GDALGetRasterBand(self.c_dataset, band_index as c_int);
            if c_band.is_null() {
                Err(_last_null_pointer_err("GDALGetRasterBand"))?;
            }
            Ok(RasterBand::_with_c_ptr(c_band, self))
        }
    }

    pub fn size<I: Integer + Copy + NumCast>(&self) -> (I,I) {
        let size_x = unsafe { gdal_sys::GDALGetRasterXSize(self.c_dataset) } ;
        let size_y = unsafe { gdal_sys::GDALGetRasterYSize(self.c_dataset) } ;
        (I::from(size_x).unwrap(), I::from(size_y).unwrap())
    }

    /// Get block size from a 'Dataset'.
    /// # Arguments
    /// * band_index - the band_index
    /*
    pub fn size_block(&self, band_index: isize) -> (usize, usize) {
        let band = self.rasterband(band_index)?;
        band.size_block()
    }
    */

    pub fn driver(&self) -> Driver {
        unsafe {
            let c_driver = gdal_sys::GDALGetDatasetDriver(self.c_dataset);
            Driver::_with_c_ptr(c_driver)
        }
    }

    pub fn count(&self) -> isize {
        (unsafe { gdal_sys::GDALGetRasterCount(self.c_dataset) }) as isize
    }

    pub fn spatial_reference(&self) -> Result<SpatialRef> {
        let c_obj = unsafe { gdal_sys::GDALGetSpatialRef(self.c_dataset) };
        if c_obj.is_null() {
            Err(_last_null_pointer_err("GDALGetSpatialRef"))?;
        }
        //clones
        SpatialRef::from_c_obj(c_obj)
    }

    pub fn projection(&self) -> String {
        let rv = unsafe { gdal_sys::GDALGetProjectionRef(self.c_dataset) };
        _string(rv)
    }

    pub fn set_projection(&self, projection: &str) -> Result<()> {
        let c_projection = CString::new(projection)?;
        unsafe { gdal_sys::GDALSetProjection(self.c_dataset, c_projection.as_ptr()) };
        Ok(())
    }

    /// Affine transformation called geotransformation.
    ///
    /// This is like a linear transformation preserves points, straight lines and planes.
    /// Also, sets of parallel lines remain parallel after an affine transformation.
    /// # Arguments
    /// * transformation - coeficients of transformations
    ///
    /// x-coordinate of the top-left corner pixel (x-offset)
    /// width of a pixel (x-resolution)
    /// row rotation (typically zero)
    /// y-coordinate of the top-left corner pixel
    /// column rotation (typically zero)
    /// height of a pixel (y-resolution, typically negative)
    pub fn set_geo_transform(&self, transformation: &GeoTransform) -> Result<()> {
        assert_eq!(transformation.len(), 6);
        let rv = unsafe {
            gdal_sys::GDALSetGeoTransform(self.c_dataset, transformation.as_ptr() as *mut f64)
        };
        if rv != CPLErr::CE_None {
            Err(_last_cpl_err(rv))?;
        }
        Ok(())
    }

    /// Get affine transformation coefficients.
    ///
    /// x-coordinate of the top-left corner pixel (x-offset)
    /// width of a pixel (x-resolution)
    /// row rotation (typically zero)
    /// y-coordinate of the top-left corner pixel
    /// column rotation (typically zero)
    /// height of a pixel (y-resolution, typically negative)
    pub fn geo_transform(&self) -> Result<GeoTransform> {
        let mut transformation = GeoTransform::default();
        let rv =
            unsafe { gdal_sys::GDALGetGeoTransform(self.c_dataset, transformation.as_mut_ptr()) };

        // check if the dataset has a GeoTransform
        if rv != CPLErr::CE_None {
            Err(_last_cpl_err(rv))?;
        }
        Ok(transformation)
    }

    pub fn create_copy(&self, driver: &Driver, filename: &str) -> Result<Dataset> {
        let c_filename = CString::new(filename)?;
        let c_dataset = unsafe {
            gdal_sys::GDALCreateCopy(
                driver._c_ptr(),
                c_filename.as_ptr(),
                self.c_dataset,
                0,
                null_mut(),
                None,
                null_mut(),
            )
        };
        if c_dataset.is_null() {
            Err(_last_null_pointer_err("GDALCreateCopy"))?;
        }
        Ok(Dataset { c_dataset })
    }

    pub fn band_type(&self, band_index: isize) -> Result<GDALDataType::Type> {
        self.rasterband(band_index).map(|band| band.band_type())
    }


    /// Adds an in memory band to the raster, pointing to the data passed in
    pub fn add_memory_band(&self, data: &Vec<u8>) {
        unsafe {
            let mut sz_ptr_value = vec![0i8; 100];
            let n_ret = CPLPrintPointer(sz_ptr_value.as_mut_ptr(),
                                        data.as_ptr() as *mut c_void,
                                        100);

            assert!(n_ret > 0);

            let str_ptr_value = CStr::from_ptr(sz_ptr_value.as_ptr());


            let mut create_options: Vec<String> = Vec::new();
            create_options.push(format!(
                "DATAPOINTER={}",
                str_ptr_value.to_str().unwrap(),).to_string());

            let c_strings: Vec<CString> = create_options.into_iter().map(|s| CString::new(s).unwrap()).collect();
		    //Need the strings as const* const* i8 for gdal, so just cast the char* string (both are 1 byte)
		    let mut c_options: Vec<*mut libc::c_char> = c_strings.iter().map(|cs| cs.as_ptr() as *mut libc::c_char).collect();

		    //null terminate the list
		    c_options.push(0 as _);


            gdal_sys::GDALAddBand(self.c_dataset, GDT_Byte, c_options.as_mut_ptr());
        }
    }

}


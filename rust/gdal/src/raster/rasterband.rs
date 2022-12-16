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
use crate::raster::types::{GdalType, IntAlias};
use crate::raster::{Dataset};
use crate::utils::_last_cpl_err;
use gdal_sys::{self, CPLErr, GDALDataType, GDALMajorObjectH, GDALRWFlag, GDALRasterBandH, GDALPolygonize};
use libc::c_int;



#[cfg(feature = "ndarray")]
use ndarray::Array2;

use anyhow::Result;
use std::convert::TryFrom;
use crate::vector::Layer;

pub struct RasterBand<'a> {
    c_rasterband: GDALRasterBandH,

    //needs to be dropped after the dataset, so enforce this via a reference
    owning_dataset: &'a Dataset,
}

impl<'a> RasterBand<'a> {
    pub fn owning_dataset(&self) -> &'a Dataset {
        self.owning_dataset
    }

    pub unsafe fn _with_c_ptr(c_rasterband: GDALRasterBandH, owning_dataset: &'a Dataset) -> Self {
        RasterBand {
            c_rasterband,
            owning_dataset,
        }
    }

    /// Get block size from a 'Dataset'.
    pub fn block_size(&self) -> (i32, i32) {
        let mut size_x = 0;
        let mut size_y = 0;

        unsafe { gdal_sys::GDALGetBlockSize(self.c_rasterband, &mut size_x, &mut size_y) };
        (size_x, size_y)
    }

    /// Get x-size of the band
    pub fn x_size(&self) -> i32 {
        let out;
        unsafe {
            out = gdal_sys::GDALGetRasterBandXSize(self.c_rasterband);
        }
        out
    }

    /// Get y-size of the band
    pub fn y_size(&self) -> i32 {
        let out;
        unsafe { out = gdal_sys::GDALGetRasterBandYSize(self.c_rasterband) }
        out
    }

    /// Get dimensions of the band.
    /// Note that this may not be the same as `size` on the
    /// `owning_dataset` due to scale.
    pub fn size(&self) -> (i32, i32) {
        (self.x_size(), self.y_size())
    }

    /// Read a 'Buffer<T>' from a 'Dataset'. T implements 'GdalType'
    /// # Arguments
    /// * band_index - the band_index
    /// * window - the window position from top left
    /// * window_size - the window size (GDAL will interpolate data if window_size != buffer_size)
    /// * buffer_size - the desired size of the 'Buffer'
    pub fn read_as<T: Copy + GdalType, I: IntAlias>(
        &self,
        window_offset: (I, I),
        size: (I, I),
    ) -> Result<Vec<T>> {
        let pixels = (size.0 * size.1).to_usize().unwrap();
        let mut data: Vec<T> = Vec::with_capacity(pixels);
        //let no_data:
        let rv = unsafe {
            gdal_sys::GDALRasterIO(
                self.c_rasterband,
                GDALRWFlag::GF_Read,
                window_offset.0.to_i32().unwrap(),
                window_offset.1.to_i32().unwrap(),
                size.0.to_i32().unwrap(),
                size.1.to_i32().unwrap(),
                data.as_mut_ptr() as GDALRasterBandH,
                size.0.to_i32().unwrap(),
                size.1.to_i32().unwrap(),
                T::gdal_type(),
                0,
                0,
            )
        };
        if rv != CPLErr::CE_None {
            Err(_last_cpl_err(rv))?;
        }

        unsafe {
            data.set_len(pixels);
        };

        Ok( data )
    }

    pub fn read_into_vec<T: Copy + GdalType>(
        &self,
        window_offset: (i32, i32),
        size: (i32, i32),
        data: &mut Vec<T>
    ) -> Result<()> {
        let pixels = (size.0 * size.1) as usize;
        assert!(data.capacity() >= pixels);
        //let no_data:
        let rv = unsafe {
            gdal_sys::GDALRasterIO(
                self.c_rasterband,
                GDALRWFlag::GF_Read,
                window_offset.0 as c_int,
                window_offset.1 as c_int,
                size.0 as c_int,
                size.1 as c_int,
                data.as_mut_ptr() as GDALRasterBandH,
                size.0 as c_int,
                size.1 as c_int,
                T::gdal_type(),
                0,
                0,
            )
        };
        if rv != CPLErr::CE_None {
            Err(_last_cpl_err(rv))?;
        }

        unsafe {
            data.set_len(pixels);
        };

        Ok(())
    }

    #[cfg(feature = "ndarray")]
    /// Read a 'Array2<T>' from a 'Dataset'. T implements 'GdalType'.
    /// # Arguments
    /// * window - the window position from top left
    /// * window_size - x size, y size -- the window size (GDAL will interpolate data if window_size != array_size)
    /// * array_size - the desired size of the 'Array'
    /// # Docs
    /// The Matrix shape is (rows, cols) and raster shape is (cols in x-axis, rows in y-axis).
    pub fn read_as_array<T: Copy + GdalType>(
        &self,
        window: (i32, i32),
        window_size: (i32, i32),
    ) -> Result<Array2<T>> {
        assert!(window_size.0 > 0);
        assert!(window_size.1 > 0);

        let pixels = (window_size.0 * window_size.1) as usize;
        let mut data: Vec<T> = Vec::with_capacity(pixels);

        let values = unsafe {
            gdal_sys::GDALRasterIO(
                self.c_rasterband,
                GDALRWFlag::GF_Read,
                window.0,
                window.1,
                window_size.0,
                window_size.1,
                data.as_mut_ptr() as GDALRasterBandH,
                window_size.0,
                window_size.1,
                T::gdal_type(),
                0,
                0,
            )
        };
        if values != CPLErr::CE_None {
            Err(_last_cpl_err(values))?;
        }

        unsafe {
            data.set_len(pixels);
        };

        // Matrix shape is (rows, cols) and raster shape is (cols in x-axis, rows in y-axis)
        Array2::from_shape_vec((window_size.1 as usize, window_size.0 as usize), data).map_err(Into::into)
    }

    /// Read a full 'Dataset' as 'Buffer<T>'.
    /// # Arguments
    /// * band_index - the band_index
    pub fn read_band_as<T: Copy + GdalType>(&self) -> Result<Vec<T>> {
        let size = self.owning_dataset.size::<i32>();
        self.read_as::<T, i32>(
            (0, 0),
            (size.0 , size.1 ),
        )
    }

    #[cfg(feature = "ndarray")]
    /// Read a 'Array2<T>' from a 'Dataset' block. T implements 'GdalType'
    /// # Arguments
    /// * block_index - the block index
    /// # Docs
    /// The Matrix shape is (rows, cols) and raster shape is (cols in x-axis, rows in y-axis).
    pub fn read_block<T: Copy + GdalType>(&self, block_index: (usize, usize)) -> Result<Array2<T>> {
        let size = self.block_size();
        let pixels = (size.0 * size.1) as usize;
        let mut data: Vec<T> = Vec::with_capacity(pixels);

        //let no_data:
        let rv = unsafe {
            gdal_sys::GDALReadBlock(
                self.c_rasterband,
                block_index.0 as c_int,
                block_index.1 as c_int,
                data.as_mut_ptr() as GDALRasterBandH,
            )
        };
        if rv != CPLErr::CE_None {
            Err(_last_cpl_err(rv))?;
        }

        unsafe {
            data.set_len(pixels);
        };

        Array2::from_shape_vec((size.1 as _, size.0 as _), data).map_err(Into::into)
    }

    // Write a 'Buffer<T>' into a 'Dataset'.
    /// # Arguments
    /// * band_index - the band_index
    /// * window - the window position from top left
    /// * window_size - (x size, y size) -- the window size (GDAL will interpolate data if window_size != Buffer.size)
    pub fn write<T: GdalType + Copy>(
        &self,
        window_offset: (i32, i32),
        window_size: (i32, i32),
        buffer: &[T],
    ) -> Result<()> {

        assert_eq!(buffer.len(), usize::try_from(window_size.0 * window_size.1).unwrap(),  "Not correct amount of data for raster");

        let rv = unsafe {

            gdal_sys::GDALRasterIO(
                self.c_rasterband,
                GDALRWFlag::GF_Write,
                window_offset.0 as c_int,
                window_offset.1 as c_int,
                window_size.0 as c_int,
                window_size.1 as c_int,
                buffer.as_ptr() as GDALRasterBandH,
                window_size.0 as c_int,
                window_size.1 as c_int,
                T::gdal_type(),
                0,
                0,
            )
        };
        if rv != CPLErr::CE_None {
            Err(_last_cpl_err(rv))?;
        }
        Ok(())
    }

    pub fn write_block_using_raster_io<T: GdalType + Copy>(
        &self,
        window_offset: (i32, i32),
        window_size: (i32, i32),
        buffer: &[T],
    ) -> Result<()> {

        let rv = unsafe {
            gdal_sys::GDALRasterIO(
                self.c_rasterband,
                GDALRWFlag::GF_Write,
                window_offset.0 as c_int,
                window_offset.1 as c_int,
                window_size.0 as c_int,
                window_size.1 as c_int,
                buffer.as_ptr() as GDALRasterBandH,
                window_size.0 as c_int,
                window_size.1 as c_int,
                T::gdal_type(),
                0,
                0,
            )
        };
        if rv != CPLErr::CE_None {
            Err(_last_cpl_err(rv))?;
        }
        Ok(())
    }

    pub fn write_block<T: GdalType + Copy>(
        &self,
        block_x_idx: i32,
        block_y_idx: i32,
        buffer: &mut Vec<T>,
    ) -> Result<()> {

        let rv = unsafe {
            gdal_sys::GDALWriteBlock(
                self.c_rasterband,
                block_x_idx,
                block_y_idx,
                buffer.as_mut_ptr() as * mut libc::c_void
            )
        };
        if rv != CPLErr::CE_None {
            Err(_last_cpl_err(rv))?;
        }
        Ok(())
    }

    pub fn band_type(&self) -> GDALDataType::Type {
        unsafe { gdal_sys::GDALGetRasterDataType(self.c_rasterband) }
    }

    pub fn no_data_value(&self) -> Option<f64> {
        let mut pb_success = 1;
        let no_data =
            unsafe { gdal_sys::GDALGetRasterNoDataValue(self.c_rasterband, &mut pb_success) };
        if pb_success == 1 {
            return Some(no_data as f64);
        }
        None
    }

    pub fn set_no_data_value(&self, no_data: f64) -> Result<()> {
        let rv = unsafe { gdal_sys::GDALSetRasterNoDataValue(self.c_rasterband, no_data) };
        if rv != CPLErr::CE_None {
            Err(_last_cpl_err(rv))?;
        }
        Ok(())
    }

    pub fn fill(&self, val: f64) -> Result<()> {
        let imaginary_value = 0.0;
        let rv = unsafe { gdal_sys::GDALFillRaster(self.c_rasterband, val, imaginary_value) };
        if rv != CPLErr::CE_None {
            Err(_last_cpl_err(rv))?;
        }
        Ok(())
    }

    /// Get actual block size (at the edges) when block size
    /// does not divide band size.
    pub fn actual_block_size(&self, block_x_idx: i32, block_y_idx: i32) -> Result<(i32, i32)> {
        let mut block_size_x = 0;
        let mut block_size_y = 0;
        let rv = unsafe {
            gdal_sys::GDALGetActualBlockSize(
                self.c_rasterband,
                block_x_idx,
                block_y_idx,
                &mut block_size_x,
                &mut block_size_y,
            )
        };
        if rv != CPLErr::CE_None {
            Err(_last_cpl_err(rv))?;
        }
        Ok((block_size_x, block_size_y))
    }

    pub fn blocks(&self) -> BlockIterator {

        let (block_num_cols, block_num_rows) = self.block_size();

        let num_x_blocks = (self.x_size() + block_num_cols - 1) / block_num_cols;
        let num_y_blocks = (self.y_size() + block_num_rows - 1) / block_num_rows;

        BlockIterator {
            cur_x_block_idx: 0,
            cur_y_block_idx: 0,
            block_num_cols,
            block_num_rows,
            num_x_blocks,
            num_y_blocks,
            raster_band: self
        }
    }

    pub fn polygonize(&self, lyr: &Layer) -> Result<()> {

        unsafe {
            let rv = GDALPolygonize(
                self.c_rasterband,
                self.c_rasterband,
                //0 as GDALRasterBandH,
                lyr.c_layer,
                -1,
                ptr::null_mut(),
                None,
                ptr::null_mut()
            );
            if rv != CPLErr::CE_None {
                Err(_last_cpl_err(rv))?;
            }
            Ok(())
        }
    }
}

impl<'a> MajorObject for RasterBand<'a> {
    unsafe fn gdal_object_ptr(&self) -> GDALMajorObjectH {
        self.c_rasterband
    }
}

impl<'a> Metadata for RasterBand<'a> {}


pub struct BlockIterator<'a> {
    cur_x_block_idx: i32,
    cur_y_block_idx: i32,
    //how many blocks
    num_x_blocks: i32,
    num_y_blocks: i32,
    //natural column width / height
    block_num_cols: i32,
    block_num_rows: i32,
    raster_band: &'a RasterBand<'a>
}

pub struct BlockIteratorItem {
    pub x_block_idx: i32,
    pub y_block_idx: i32,
    pub x_block_num_cols: i32,
    pub y_block_num_rows: i32,
    pub is_normal_block_size: bool,
    pub x_left: i32,
    pub y_top: i32
}

impl BlockIteratorItem {
    /// idx is within the block itself, returns the raster x and y
    pub fn coords_for_idx(&self, idx: i32) -> (i32, i32)
    {
        let x = self.x_left + idx % self.x_block_num_cols;
        let y = self.y_top + idx / self.x_block_num_cols;
        (x, y)
    }
}

impl <'a> Iterator for BlockIterator<'a> {
    type Item = BlockIteratorItem;

    #[inline]
    fn next(&mut self) -> Option<BlockIteratorItem> {

        if self.cur_y_block_idx >= self.num_y_blocks {
            return None;
        }

        let (x_block_num_cols, y_block_num_rows) = self.raster_band.actual_block_size(
            self.cur_x_block_idx,
            self.cur_y_block_idx
        ).unwrap();

        let ret = BlockIteratorItem {
            x_block_idx: self.cur_x_block_idx,
            y_block_idx: self.cur_y_block_idx,
            x_block_num_cols,
            y_block_num_rows,
            is_normal_block_size: x_block_num_cols == self.block_num_cols && y_block_num_rows == self.block_num_rows,
            x_left: self.cur_x_block_idx * self.block_num_cols,
            y_top: self.cur_y_block_idx * self.block_num_rows
        };

        self.cur_x_block_idx += 1;
        if self.cur_x_block_idx >= self.num_x_blocks {
            self.cur_x_block_idx = 0;
            self.cur_y_block_idx += 1;
        }

        Some(ret)
    }
}

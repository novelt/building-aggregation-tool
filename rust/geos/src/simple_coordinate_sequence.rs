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
use anyhow::{bail, Result};
use SimpleContextHandle;

pub struct SimpleCoordinateSequence<'c>
{
    pub(crate) c_handle: *mut GEOSCoordSequence,
    pub(crate) owned: bool,
    pub(crate) context_handle: &'c SimpleContextHandle
}

impl <'c> SimpleCoordinateSequence<'c> {
    pub fn new(length: u32, context_handle: &'c SimpleContextHandle) -> Result<Self> {
        unsafe {
            let ptr = GEOSCoordSeq_create_r(context_handle.c_handle, length, 2);

            if ptr.is_null() {
                bail!("GEOSCoordSeq_create_r");
            }

            Ok(SimpleCoordinateSequence {
                c_handle: ptr,
                owned: true,
                context_handle
            })
        }
    }

    pub fn from_slice_pts(slice: &[ [f64; 2] ], context_handle: &'c SimpleContextHandle) -> Result<Self> {
        let mut ret = Self::new(slice.len() as u32, context_handle)?;

        for (idx, [x, y]) in slice.iter().enumerate() {
            ret.set_x(idx as u32, *x)?;
            ret.set_y(idx as u32, *y)?;
        }

        Ok(ret)

    }

    pub fn num_points(&self) -> Result<u32> {
        let mut n = 0;
        let ret_val =
            unsafe { GEOSCoordSeq_getSize_r(self.context_handle.c_handle, self.c_handle, &mut n) };
        if ret_val == 0 {
            bail!("getting size from CoordSeq");
        } else {
            Ok(n)
        }
    }

    pub fn points(&self) -> Result<PointIterator> {
        PointIterator::new(self)
    }

    /// Gets the X position value at the given `line`.
    pub fn get_x(&self, index: u32) -> Result<f64> {

        let mut n = 0.;
        let ret_val = unsafe {
            GEOSCoordSeq_getX_r(self.context_handle.c_handle, self.c_handle, index, &mut n)
        };
        if ret_val == 0 {
            bail!("failed to get coordinates from CoordSeq");
        } else {
            Ok(n)
        }
    }

    /// Gets the Y position value at the given `line`.
    pub fn get_y(&self, index: u32) -> Result<f64> {

        let mut n = 0.;
        let ret_val = unsafe {
            GEOSCoordSeq_getY_r(self.context_handle.c_handle, self.c_handle, index, &mut n)
        };
        if ret_val == 0 {
            bail!("failed to get coordinates from CoordSeq");
        } else {
            Ok(n)
        }
    }

    pub fn is_ccw(&self) -> Result<bool> {

        let mut is_ccw_ret: i8 = 0;
        let ret_val = unsafe {
            GEOSCoordSeq_isCCW_r(self.context_handle.c_handle, self.c_handle, &mut is_ccw_ret)
        };
        if ret_val == 0 {
            bail!("failed to set coordinates from CoordSeq");
        }

        Ok(is_ccw_ret != 0)
    }

    pub fn set_x(&mut self, index: u32, value: f64) -> Result<()> {

        let ret_val = unsafe {
            GEOSCoordSeq_setX_r(self.context_handle.c_handle, self.c_handle, index, value)
        };
        if ret_val == 0 {
            bail!("failed to set coordinates from CoordSeq");
        } else {
            Ok(())
        }
    }

    /// Gets the Y position value
    pub fn set_y(&mut self, index: u32, value: f64) -> Result<()> {

        let ret_val = unsafe {
            GEOSCoordSeq_setY_r(self.context_handle.c_handle, self.c_handle, index, value)
        };
        if ret_val == 0 {
            bail!("failed to set coordinates from CoordSeq");
        } else {
            Ok(())
        }
    }
}

impl <'c> Drop for SimpleCoordinateSequence<'c> {
    fn drop(&mut self) {
        unsafe {
            if self.owned {
                println!("Dropping simple coordinate sequence");
                GEOSCoordSeq_destroy_r(self.context_handle.c_handle, self.c_handle);
            }
        }
    }
}

/// Lifetime of dataset must at least be as long of the layer
pub struct PointIterator<'c> {
    coord_seq: &'c SimpleCoordinateSequence<'c>,
    length: u32,
    cur_point: u32
}

impl<'c> Iterator for PointIterator<'c> {
    type Item = [f64; 2];

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {

        if self.cur_point == self.length {
            None
        } else {
            let x = self.coord_seq.get_x(self.cur_point);
            let y = self.coord_seq.get_y(self.cur_point);

            self.cur_point += 1;

            if x.is_err() || y.is_err() {
                None
            } else {
                Some( [x.unwrap(), y.unwrap()])
            }
        }
    }
}

impl<'c> PointIterator<'c> {
    pub(crate) fn new(coord_seq: &'c SimpleCoordinateSequence) -> Result<Self> {
        Ok(PointIterator {
            coord_seq,
            length: coord_seq.num_points()?,
            cur_point: 0
        })
    }
}
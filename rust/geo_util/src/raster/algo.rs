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
use std::path::{Path};

use anyhow::{Result};
use gdal::raster::{Driver};
use gdal::raster::driver::{DEFAULT_RASTER_OPTIONS, GTIFF_DRIVER};

use crate::raster::{RasterStats};
use float_cmp::{ApproxEq, F32Margin, F64Margin};
use std::fs::create_dir_all;
use log::{debug,};

pub fn create_empty_raster(raster_path: &Path,
                           snap_stats: &RasterStats,
    fill_with_nodata: bool
) -> Result<()>
{
    //debug!("Creating output tif {:?}", &raster_path);

    if let Some(a) = raster_path.parent() {
        if !a.exists() {
            create_dir_all(a)?;
        }
    }

    let drv = Driver::get(GTIFF_DRIVER)?;

    //just want to create it and close it
    let ds = drv.create_with_band_type::<&str, _>(
        &raster_path,
        snap_stats.num_cols as isize,
        snap_stats.num_rows as isize, 1, snap_stats.gdal_type,
        &DEFAULT_RASTER_OPTIONS
        )?;

    //debug!("Created output tif {:?}", &raster_path);

    let output_raster_band = ds.rasterband(1)?;

    //debug!("Setting output No data value to {}", snap_stats.no_data_value);
    output_raster_band.set_no_data_value(snap_stats.no_data_value)?;
    if fill_with_nodata {
        output_raster_band.fill(snap_stats.no_data_value)?;
    }

    let left = snap_stats.origin_x;
    let top = snap_stats.origin_y;
    let raster_tile_size_x = snap_stats.pixel_width;
    let raster_tile_size_y = snap_stats.pixel_height;

    //because y is the top not the bottom
    assert!(raster_tile_size_y < 0.0);
    //debug!("setting geo transform & projection");
    ds.set_geo_transform(&[left, raster_tile_size_x, 0.0, top, 0.0, raster_tile_size_y])?;

    //let srs = SpatialRef::from_epsg(4326)?;

    ds.set_projection(&snap_stats.projection)?;

    //debug!("Set projection to {}", &ds.projection());

    Ok(())
}


pub fn create_empty_raster_with_options(raster_path: &Path,
                           snap_stats: &RasterStats,
    fill_with_nodata: bool,
    create_options: &[&str],
) -> Result<()>
{
    debug!("Creating output tif {:?}", &raster_path);

    let drv = Driver::get(GTIFF_DRIVER)?;

    //just want to create it and close it
    let ds = drv.create_with_band_type::<&str, _>(
        &raster_path,
        snap_stats.num_cols as isize,
        snap_stats.num_rows as isize, 1, snap_stats.gdal_type,
        create_options
        )?;

    debug!("Created output tif {:?}", &raster_path);

    let output_raster_band = ds.rasterband(1)?;

    debug!("Setting output No data value to {}", snap_stats.no_data_value);
    output_raster_band.set_no_data_value(snap_stats.no_data_value)?;
    if fill_with_nodata {
        output_raster_band.fill(snap_stats.no_data_value)?;
    }

    let left = snap_stats.origin_x;
    let top = snap_stats.origin_y;
    let raster_tile_size_x = snap_stats.pixel_width;
    let raster_tile_size_y = snap_stats.pixel_height;

    //because y is the top not the bottom
    assert!(raster_tile_size_y < 0.0);
    debug!("setting geo transform & projection");
    ds.set_geo_transform(&[left, raster_tile_size_x, 0.0, top, 0.0, raster_tile_size_y])?;

    //let srs = SpatialRef::from_epsg(4326)?;

    ds.set_projection(&snap_stats.projection)?;

    debug!("Set projection to {}", &ds.projection());

    Ok(())
}



#[inline]
pub fn is_nodata(val: f32, no_data_value: f32) -> bool {

    //seems like Gdal can read nodata values as NaN
    if !val.is_finite() {
        return true;
    }

    if !no_data_value.is_finite() {
        return false;
    }

    no_data_value.approx_eq(val, F32Margin{ ulps: 5, epsilon: f32::EPSILON * 5.0})
}

#[inline]
pub fn is_nodata_f64(val: f64, no_data_value: f64) -> bool {

    //seems like Gdal can read nodata values as NaN
    if !val.is_finite() {
        return true;
    }

    if !no_data_value.is_finite() {
        return false;
    }

    no_data_value.approx_eq(val, F64Margin{ ulps: 5, epsilon: f64::EPSILON * 5.0})
}


pub trait IsNoData {
    fn is_value_nodata(self, no_data_val: f64) -> bool;

}


impl IsNoData for f32 {
    fn is_value_nodata(self, no_data_val: f64) -> bool {
        is_nodata(self, no_data_val as f32)
    }
}
impl IsNoData for f64 {
    fn is_value_nodata(self, no_data_val: f64) -> bool {
        is_nodata_f64(self, no_data_val )
    }
}
impl IsNoData for i32 {
    fn is_value_nodata(self, no_data_val: f64) -> bool {
        is_nodata_f64(self as f64, no_data_val )
    }
}

#[cfg(test)]
mod test {
    use crate::raster::{is_nodata, is_nodata_f64};

    #[test]
    fn test_is_nodata()  {
        let nodata = f32::MIN;

        assert!(is_nodata(nodata + 10000., nodata));
        assert!(is_nodata(nodata - 10000., nodata));
        assert!(is_nodata(f32::NAN, nodata));
        assert!(is_nodata(f32::INFINITY, nodata));

        assert!(!is_nodata(nodata + 1e34, nodata));

        let nodata = f32::MAX;

        assert!(is_nodata(nodata - 10000., nodata));
        assert!(!is_nodata(nodata - 1e36, nodata));

        let nodata = f32::NAN;

        assert!(is_nodata(f32::NAN , nodata));
        assert!(!is_nodata( 1e30, nodata));
    }

    #[test]
    fn test_is_nodata_64()  {
        let nodata = f32::MIN as f64;

        assert!(is_nodata_f64(nodata + 10000., nodata));
        assert!(is_nodata_f64(nodata - 10000., nodata));
        assert!(is_nodata_f64(f64::NAN, nodata));
        assert!(is_nodata_f64(f64::INFINITY, nodata));

        assert!(!is_nodata_f64(nodata + 1e34, nodata));

        let nodata = f32::MAX as f64;

        assert!(is_nodata_f64(nodata - 10000., nodata));
        assert!(!is_nodata_f64(nodata - 1e36, nodata));

        let nodata = f64::MIN;

        assert!(is_nodata_f64(nodata + 10000., nodata));
        assert!(is_nodata_f64(nodata - 10000., nodata));

        assert!(!is_nodata_f64(nodata + 1e306, nodata));

        let nodata = f64::MAX;

        assert!(is_nodata_f64(nodata - 10000., nodata));
        assert!(!is_nodata_f64(nodata - 1e306, nodata));

        let nodata = f64::NAN;

        assert!(is_nodata_f64(f64::NAN , nodata));
        assert!(is_nodata_f64(f32::NAN as f64 , nodata));
        assert!(is_nodata_f64(f64::NAN , f32::NAN as f64));
        assert!(!is_nodata_f64( 1e306, nodata));


    }
}
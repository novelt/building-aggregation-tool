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
use serde::Deserialize;
use gdal::raster::{Dataset, RasterBand, GDALDataType};
use std::cmp::{max, min};
use core::fmt;
use geo::{Point as GeoPoint};
use geo::algorithm::haversine_distance::HaversineDistance;
use gdal::raster::types::{convert_gdal_type_to_string, GdalType, IntAlias};
use float_cmp::{ApproxEq, F64Margin};

use gdal::vector::OGREnvelope;
use crate::raster::{is_nodata_f64, is_nodata};

/// Helper struct to hold stats of a raster
#[derive(Debug, Deserialize, Clone, Default)]
pub struct RasterStats {
    pub origin_y: f64,
    pub origin_x: f64,
    pub pixel_height: f64,
    pub pixel_width: f64,
    pub num_rows: u32,
    pub num_cols: u32,
    pub no_data_value: f64,
    pub gdal_type: GDALDataType::Type,

    //WKT projection string
    pub projection: String
}

pub fn float_within(a: f64, b: f64) -> bool {
    return (a-b).abs() <= 5.0 * f64::EPSILON;
}
//const SMALL_EPSILON: f64 = 5.0 * f64::EPSILON;
pub const MEDIUM_EPSILON: f64 = 1e-10;

// In lat/lon this is less than a meter
pub const LARGE_EPSILON: f64 = 1e-6;

pub fn assert_float_within_eps(a: f64, b: f64, eps: f64, msg: &str) {
    let diff =  (a-b).abs();
    if diff > eps {
        let message = format!("{} Val 1: {} Val 2: {} Abs. Difference: {}  Eps: {}", msg,
                              a, b, diff, eps);
        panic!("{}", message);
    }
}

impl fmt::Display for RasterStats {
    // This trait requires `fmt` with this exact signature.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {

        write!(f, "Origin X,Y: {}, {}\nRight/Bottom: {},{}\nPixel Width/Height: {},{}\nRows: {} Cols: {}\nNo data value: {}\nGdal Type: {} - {}\nProjection: {}",
               self.origin_x,
               self.origin_y,
            self.calc_x_coord(self.num_cols),
            self.calc_y_coord(self.num_rows),
               self.pixel_width,
                self.pixel_height,
            self.num_rows,
            self.num_cols,
            self.no_data_value,
            convert_gdal_type_to_string(self.gdal_type),
            self.gdal_type,
            &self.projection
        )
    }
}


impl RasterStats {

    pub fn calc_center<I: IntAlias>(&self, raster_xy: (I, I)) -> [f64;2] {
        [self.origin_x + self.pixel_width * (raster_xy.0.to_f64().unwrap() + 0.5),
        self.origin_y + self.pixel_height * (raster_xy.1.to_f64().unwrap() + 0.5) ]
    }

    /// Calculates the left side
    /// Calculates projected x coordinate from raster_x
    pub fn calc_x_coord<I: IntAlias>(&self, raster_x: I) -> f64 {
        self.origin_x + self.pixel_width * raster_x.to_f64().unwrap()
    }
    pub fn right_x_coord(&self) -> f64 {
        self.calc_x_coord(self.num_cols)
    }
    ///calculates the top side
    /// Note pixel height is negative
    pub fn calc_y_coord<I: IntAlias>(&self, raster_y: I) -> f64 {
        self.origin_y + self.pixel_height * raster_y.to_f64().unwrap()
    }
    pub fn bottom_y_coord(&self) -> f64 {
        self.calc_y_coord(self.num_rows)
    }

    //Converts projected coordinate to raster_x
    pub fn calc_x(&self, x_coord: f64) -> i32 {
        ((x_coord - self.origin_x) / self.pixel_width).floor() as _
    }
    pub fn calc_y(&self, y_coord: f64) -> i32 {
        ((y_coord - self.origin_y) / self.pixel_height).floor() as _
    }

    pub fn bounds_x(&self, raster_x: i32) -> i32 {
        if raster_x < 0 {
            return 0
        }

        if raster_x >= self.num_cols as i32 {
            return self.num_cols as i32 - 1;
        }

        raster_x
    }

    pub fn bounds_y(&self, raster_y: i32) -> i32 {
        if raster_y < 0 {
            return 0
        }

        if raster_y >= self.num_rows as i32 {
            return self.num_rows as i32 - 1;
        }

        raster_y
    }

    //Only used for common offset stuff; for projected coord => raster coords, use calc_x / calc_y
    fn calc_x_round(&self, x_coord: f64) -> i32 {
        ((x_coord - self.origin_x) / self.pixel_width).round() as i32
    }
    fn calc_y_round(&self, y_coord: f64) -> i32 {
        ((y_coord - self.origin_y) / self.pixel_height).round() as i32
    }

    pub fn new(dataset: &Dataset, band: &RasterBand) -> Self {

        /*
        println!("rasterband description: {:?}", rasterband.description());
    println!("rasterband no_data_value: {:?}", rasterband.no_data_value());
    println!("rasterband type: {:?} = {} ", rasterband.band_type(), get_type_name(rasterband.band_type()).unwrap());
    println!("rasterband scale: {:?}", rasterband.scale());
    println!("rasterband offset: {:?}", rasterband.offset());
        */
        let geotransform = dataset.geo_transform().unwrap();

        let pixel_width = geotransform[1];
        let pixel_height = geotransform[5];
        let origin_x = geotransform[0];
        let origin_y = geotransform[3];

        let (num_cols, num_rows) = dataset.size::<u32>();
        //let num_rows = num_rows as u32;
        //let num_cols = num_cols as u32;

        let no_data_value = band.no_data_value().unwrap_or(f64::MIN);

        let gdal_type = band.band_type();

        let projection = dataset.projection();

        RasterStats {
            origin_y,
            origin_x,
            pixel_width,
            pixel_height,
            num_cols ,
            num_rows ,
            no_data_value,
            gdal_type,
            projection
        }
    }

    pub fn avg_area_m2(&self) -> f64
    {
        //average height
        let top = self.calc_y_coord(0);
        let bottom = self.calc_y_coord(self.num_rows as i32);
        assert!(top > bottom);
        let left = self.calc_x_coord(0);
        let right = self.calc_x_coord(self.num_cols as i32);
        assert!(left < right);

        let top_left = GeoPoint::new(left, top);
        let bottom_left = GeoPoint::new(left, bottom);

        let top_right = GeoPoint::new(right, top);
        let bottom_right = GeoPoint::new(right, bottom);

        let avg_height = {
            let h = (top_left.haversine_distance(&bottom_left) + top_right.haversine_distance(&bottom_right))/2.0;
            h / self.num_rows as f64
        };

        let avg_width = {
            let w = (top_left.haversine_distance(&top_right) +
                bottom_left.haversine_distance(&top_right)) / 2.0;
            w / self.num_cols as f64
        };

        return avg_height * avg_width;
    }

    ///
    pub fn common_offsets(&self, rhs: &RasterStats) -> Offsets
    {
        //calculate the raster x coordinate with respect to the origin of the other raster
        let x1 = self.calc_x_round(rhs.origin_x);
        let x2 = rhs.calc_x_round(self.origin_x);

        //ensure no rounding problems
        assert_eq!(x1, -x2);

        let y1 = self.calc_y_round(rhs.origin_y);
        let y2 = rhs.calc_y_round(self.origin_y);

        assert_eq!(y1, -y2);

        //we cant go negative, these are areas where the rasters do not intersect
        let offset_x_1 = max(0, x1) ;
        let offset_x_2 = max(0, x2) ;
        let offset_y_1 = max(0, y1) ;
        let offset_y_2 = max(0, y2) ;

        assert!(offset_x_1 < self.num_cols as i32);
        assert!(offset_x_2 < rhs.num_cols as i32);

        assert!(offset_y_1 < self.num_rows as i32);
        assert!(offset_y_2 < rhs.num_rows as i32);

        Offsets
        {
            offset_x_1,
            offset_x_2,
            offset_y_1,
            offset_y_2,
            num_cols: min(self.num_cols as i32  - offset_x_1 , rhs.num_cols as i32 - offset_x_2) as u32,
            num_rows: min(self.num_rows as i32 - offset_y_1, rhs.num_rows as i32 - offset_y_2) as u32
        }
    }

    pub fn assert_equals_except_no_data(&self, rhs: &Self) {

        assert_eq!(self.num_cols, rhs.num_cols);
        assert_eq!(self.num_rows, rhs.num_rows);
        assert_float_within_eps(self.origin_x, rhs.origin_x, LARGE_EPSILON, "Origin X");
        assert_float_within_eps(self.origin_y, rhs.origin_y, LARGE_EPSILON, "Origin Y");

        assert_float_within_eps(self.pixel_height, rhs.pixel_height, MEDIUM_EPSILON, "pixel height");
        assert_float_within_eps(self.pixel_width, rhs.pixel_width, MEDIUM_EPSILON, "pixel width");

    }

    pub fn is_aligned(&self, rhs: &Self) -> bool {

        if self.projection != rhs.projection {
            println!("Not aligned, different projection");
            return false;
        }

        assert_float_within_eps(self.pixel_height, rhs.pixel_height, MEDIUM_EPSILON, "pixel height");
        assert_float_within_eps(self.pixel_width, rhs.pixel_width, MEDIUM_EPSILON, "pixel width");

        //check the origin x difference is an integer multiple of pixel_width
        let ox_diff = (self.origin_x - rhs.origin_x) / self.pixel_width;

        let oy_diff = (self.origin_y - rhs.origin_y) / self.pixel_height;

        if !(ox_diff.round() - ox_diff).approx_eq(0.0, F64Margin{epsilon: LARGE_EPSILON, ulps: 0 }) {
            println!("Not aligned - X: Origin diff: {} div: {}",
                     (ox_diff.round() - ox_diff),
                     ox_diff);
            return false;
        }

        if !(oy_diff.round() - oy_diff).approx_eq(0.0, F64Margin{epsilon: LARGE_EPSILON, ulps: 0 }) {
            println!("Not aligned - Y: Origin diff: {} div: {}",
                     (oy_diff.round() - oy_diff),
                     oy_diff);
            return false;
        }

        return true;

    }

    //Shortcut when dealing with f64 values & nodata.  Handles f32 case
    pub fn is_nodata(&self, in_value: f64) -> bool {
        if is_nodata_f64(in_value, self.no_data_value) {
            return true;
        }
        //We need to do this since the comparison with f64 doesn't work since a f32
        //rounding error will be much migger than the "Unit of least precision" of f64
        if self.gdal_type == f32::gdal_type() && is_nodata(in_value as f32, self.no_data_value as f32) {
            return true;
        }
        return false;
    }

    pub fn get_chunk_width_height(&self, chunk_rows: u32, chunk_cols: u32) -> (f64, f64) {

        //We want the chunk_width / height to be exactly on a raster square boundary
        let raster_cols_in_chunk = num::Integer::div_ceil(&self.num_cols, &chunk_cols);
        let raster_rows_in_chunk = num::Integer::div_ceil(&self.num_rows, &chunk_rows);

        let chunk_width = self.pixel_width *  raster_cols_in_chunk as f64;
        let chunk_height = self.pixel_height * raster_rows_in_chunk as f64;

        assert!(chunk_height < 0.);

        (chunk_width, chunk_height)
    }

    pub fn get_chunk_width_height_non_aligned(&self, chunk_rows: u32, chunk_cols: u32) -> (f64, f64) {

        let raster_width_coords = self.right_x_coord() - self.origin_x;
        let raster_height_coords = self.bottom_y_coord() - self.origin_y;

        assert!(raster_width_coords > 0.0);
        assert!(raster_height_coords < 0.0);

        (raster_width_coords / chunk_cols as f64,
        raster_height_coords / chunk_rows as f64)
    }

    pub fn get_chunk_index(&self, chunk_wh: &(f64, f64),
                           chunk_rows: u32,
                           chunk_cols: u32,
                           env: &OGREnvelope) -> Option<usize> {
        let center_x = (env.MaxX + env.MinX) / 2.0;
        let center_y = (env.MaxY + env.MinY) / 2.0;

        if center_x < self.origin_x || center_x > self.right_x_coord() {
            return None;
        }
        if center_y > self.origin_y || center_y < self.bottom_y_coord() {
            return None;
        }

        let chunk_x = ((center_x - self.origin_x) / chunk_wh.0).floor() as isize;
        let chunk_y = ((center_y - self.origin_y) / chunk_wh.1).floor() as isize;

        //If it's outside then we just skip it
        assert!(chunk_x >= 0 && chunk_x < chunk_cols as isize);
        assert!(chunk_y >= 0 && chunk_y < chunk_rows as isize);

        let chunk_index = (chunk_x + chunk_y * chunk_cols as isize) as usize;
        Some(chunk_index)
    }
}

///
/// Represents what is needed for the common squares between 2 rasters
#[derive(Default, Debug)]
pub struct Offsets {
    //value added to raster_x (column) of 1st raster
    pub offset_x_1: i32,
    //value added to raster_x (column) of 2nd raster
    pub offset_x_2: i32,
    //value added to raster_y (row) of 1st raster
    pub offset_y_1: i32,
    //value added to raster_y (row) of 2nd raster
    pub offset_y_2: i32,

    //# of columns in common
    pub num_cols: u32,
    pub num_rows: u32
}


#[cfg(test)]
mod tests {
    use super::*;
    use gdal::raster::types::GdalType;

    #[test]
    fn test_coords() {
        let r1 = RasterStats {
            origin_x: 4.0,
            origin_y: 5.0,
            pixel_height: -2.0,
            pixel_width: 1.0,
            num_rows: 4,
            num_cols: 5,
            no_data_value: 3.2,
            gdal_type: f32::gdal_type(),
            projection: "".to_string()
        };

        assert_eq!(r1.calc_x(4.0), 0);
        assert_eq!(r1.calc_x(4.999), 0);
        assert_eq!(r1.calc_x(5.0), 1);

        assert_eq!(r1.calc_x_round(4.0), 0);
        assert_eq!(r1.calc_x_round(4.999), 1);
        assert_eq!(r1.calc_x_round(5.0), 1);

    }

    #[test]
    fn test_offsets() {
        let r1 = RasterStats {
            origin_x: 4.0,
            origin_y: 5.0,
            pixel_height: -2.0,
            pixel_width: 1.0,
            num_rows: 4,
            num_cols: 5,
            no_data_value: 3.2,
            gdal_type: f32::gdal_type(),
            projection: "".to_string()
        };

        let r2 = RasterStats {
            origin_x: 5.0,
            origin_y: 9.0,
            pixel_height: -2.0,
            pixel_width: 1.0,
            num_rows: 3,
            num_cols: 10,
            no_data_value: 3.2,
            gdal_type: f32::gdal_type(),
            projection: "".to_string()
        };

        let offsets = r1.common_offsets(&r2);

        assert_eq!(offsets.offset_x_1, 1);
        assert_eq!(offsets.offset_x_2, 0);

        assert_eq!(offsets.offset_y_1, 0);
        assert_eq!(offsets.offset_y_2, 2);

        assert_eq!(offsets.num_cols, 4);
        assert_eq!(offsets.num_rows, 1);

    }

    #[test]
    fn test_offsets_2() {
        let r1 = RasterStats {
            origin_x: -31.266805555555553,
            origin_y: 39.709861111111117,
            pixel_height: -0.000277777777778,
            pixel_width: 0.000277777777778,
            num_rows: 268347,
            num_cols: 341152,
            no_data_value: 3.2,
            gdal_type: f32::gdal_type(),
            projection: "".to_string()
        };

        let r2 = RasterStats {
            origin_x: 16.626250000000002,
            origin_y: -22.500138888888891,
            pixel_height: -0.000277777777778,
            pixel_width: 0.000277777777778,
            num_rows: 44391,
            num_cols: 58590,
            no_data_value: 3.2,
            gdal_type: f32::gdal_type(),
            projection: "".to_string()
        };

        let offsets = r1.common_offsets(&r2);

        assert_eq!(offsets.offset_x_1, 172415,);
        assert_eq!(offsets.offset_x_2, 0);

        assert_eq!(offsets.offset_y_1, 223956,);
        assert_eq!(offsets.offset_y_2, 0);

        assert_eq!(offsets.num_cols, r2.num_cols);
        assert_eq!(offsets.num_rows, r2.num_rows);

    }

    #[test]
    fn test_is_aligned() {


        let r1 = RasterStats {
            origin_x: -13.261527777777777,
            origin_y: 35.324305555555554,
            pixel_height: -0.000277777777778,
            pixel_width: 0.000277777777778,
            num_rows: 4,
            num_cols: 5,
            no_data_value: 3.2,
            gdal_type: f32::gdal_type(),
            projection: "".to_string()
        };

        let r2 = RasterStats {
            origin_x: 34.908472222222223,
            origin_y: 5.457361111111111,
            pixel_height: r1.pixel_height,
            pixel_width: r1.pixel_width,
            num_rows: 3,
            num_cols: 10,
            no_data_value: 3.2,
            gdal_type: f64::gdal_type(),
            projection: "".to_string()
        };

        assert!(r1.is_aligned(&r2));

        let r3 = RasterStats {
            origin_x: r2.origin_x,
            origin_y: r2.origin_y + 0.05 * r2.pixel_height,
            pixel_height: r2.pixel_height,
            pixel_width: r2.pixel_width,
            num_rows: 3,
            num_cols: 10,
            no_data_value: 3.2,
            gdal_type: f64::gdal_type(),
            projection: "".to_string()
        };

        assert!(!r1.is_aligned(&r3));
    }

    #[test]
    fn test_chunks() {
        let r = RasterStats {
            origin_x: -0.147916651080003,
            origin_y: 11.138750156820009,
            pixel_height: -0.000833333330000,
            pixel_width: 0.000833333330000,
            num_rows: 6036,
            num_cols: 2350,
            no_data_value: 3.2,
            gdal_type: f64::gdal_type(),
            projection: "".to_string()
        };
        
        let chunk_wh = r.get_chunk_width_height(10, 10);
        
        let env1 = OGREnvelope {
            MinX: 1.3953136,
            MaxX: 1.3953136,
            MinY: 6.6087473,
            MaxY: 6.6087473,
        };

        let chunk_index1 = r.get_chunk_index(&chunk_wh, 10, 10, &env1);

        let env2 = OGREnvelope {
            MinX: 1.3945833427499974,
            MaxX: 1.3954166760799975,
            MinY: 6.60791684161001,
            MaxY: 6.60875017494001,
        };

        let chunk_index2 = r.get_chunk_index(&chunk_wh, 10, 10, &env2);

        assert!(chunk_index1.is_some());
        assert!(chunk_index2.is_some());

        assert_eq!(chunk_index1.unwrap(), chunk_index2.unwrap());
        assert_eq!(97, chunk_index1.unwrap());

    }
}

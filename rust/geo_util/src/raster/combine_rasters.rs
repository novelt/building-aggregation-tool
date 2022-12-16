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
use std::path::{Path,  };
use std::time::Instant;

use crate::raster::{create_empty_raster, Raster, IsNoData};
//use gdal::raster::{Dataset as RasterDataset, RasterBand};
use crate::util::{format_duration, print_remaining_time, RasterChunkIterator};

use anyhow::{Result, bail};
use std::fs::{remove_file, create_dir_all};
use gdal::raster::types::GdalType;

/// Combines 2 rasters
pub fn combine_rasters<T, F>(raster_lhs: &Path, raster_rhs: &Path,
                      raster_output: &Path,
                      nodata_output: f64,
                      mut combine_func: F) -> Result<()>
where T: Copy + GdalType + IsNoData,
F: FnMut(T, bool, T, bool) -> Result<T>
{

    let now = Instant::now();
    let mut last_output = Instant::now();


    let raster_lhs = Raster::read(&raster_lhs, true);

    let raster_rhs = Raster::read(&raster_rhs, true);

    println!("Stats Left: {}\n", raster_lhs.stats);
    println!("No data left: {} Right: {}", raster_lhs.stats.no_data_value,
        raster_rhs.stats.no_data_value);

    raster_lhs.stats.assert_equals_except_no_data(&raster_rhs.stats);

    if raster_output.exists() {
        remove_file(&raster_output)?;
    }

    if !raster_output.parent().unwrap().exists() {
        create_dir_all(raster_output.parent().unwrap())?;
    }

    let mut new_stats = raster_rhs.stats.clone();

    new_stats.no_data_value = nodata_output;
    new_stats.gdal_type = T::gdal_type();

    create_empty_raster(&raster_output, &new_stats, false)?;

    let output_raster = Raster::read(&raster_output, false);
    let output_band = output_raster.band();

    let no_data_left = raster_lhs.stats.no_data_value ;
    let no_data_right = raster_rhs.stats.no_data_value ;

    let number_of_chunks = 10;

    for raster_window in RasterChunkIterator::<i32>::new( output_raster.stats.num_rows as _,
                                                                              output_raster.stats.num_cols as _,number_of_chunks as _)
    {
        //println!("Combining {:?} and {:?}", window_offset, window_size);

        let left_data = raster_lhs.band().read_as::<T, i32>(
            raster_window.window_offset, raster_window.window_size

        )?;

        let right_data = raster_rhs.band().read_as::<T, i32>(
            raster_window.window_offset, raster_window.window_size
        )?;

        assert_eq!(left_data.len(), right_data.len());
        assert!(!left_data.is_empty());

        let mut output_data = Vec::with_capacity(left_data.len());

        for idx in 0..left_data.len() {

            let result = combine_func(left_data[idx], left_data[idx].is_value_nodata(no_data_left),
                                           right_data[idx], right_data[idx].is_value_nodata(no_data_right));
            if let Ok(result_value) = result {
                output_data.push(result_value);
            } else {

                let idx = idx as i32;
                let offset_x = idx % raster_window.window_size.0;
                let offset_y = idx / raster_window.window_size.0;
                let raster_x = raster_window.window_offset.0 + offset_x;
                let raster_y = raster_window.window_offset.1 + offset_y;
                let coord_x = raster_lhs.stats.calc_x_coord(raster_x);
                let coord_y = raster_lhs.stats.calc_y_coord(raster_y);

                bail!("Combine Raster Problem at {},{} coords {lon},{lat}.  {}", raster_x, raster_y, result.err().unwrap(), lon=coord_x, lat=coord_y, );
            }
        }

        assert_eq!(output_data.len(), left_data.len());

        output_band.write(
            raster_window.window_offset ,
            raster_window.window_size, &output_data)?;


        if last_output.elapsed().as_secs() >= 3 {
            last_output = Instant::now();

            print_remaining_time(&now, raster_window.current_step as u32, raster_window.num_steps as u32);
        }
    }

    println!("Finished in {}", format_duration(now.elapsed()));

    Ok(())


}


#[cfg(test)]
mod raster_combine_test {
    use super::*;
    use crate::raster::{RasterStats, create_test_raster, get_temp_filename};
    use gdal::spatial_ref::SpatialRef;
    use gdal::raster::types::GdalType;
    use itertools::Itertools;

    #[test]
    fn test_simple_add() {

        let srs = SpatialRef::from_epsg(4326).unwrap();

        let origin_y = 46.242485;
        let origin_x = 6.021557;

        let lhs_stats = RasterStats {
            origin_y,
            origin_x,
            pixel_height: -0.005,
            pixel_width: 0.004,
            num_rows: 3,
            num_cols: 3,
            no_data_value: -1000.0,
            gdal_type: i16::gdal_type(),
            projection: srs.to_wkt().unwrap()
        };

        let mut rhs_stats = lhs_stats.clone();
        rhs_stats.gdal_type = u32::gdal_type();
        rhs_stats.no_data_value = 10000.0;

        let mut lhs_data: Vec<i16> = (-9..0).rev().collect_vec();
        lhs_data[8] = -50;
        lhs_data[7] = lhs_stats.no_data_value as i16;
        lhs_data[5] = lhs_stats.no_data_value as i16;
        let lhs_path = create_test_raster("lhs.tif", &lhs_stats, &lhs_data ).unwrap();

        let mut rhs_data: Vec<u32> = (10..19).collect_vec();
        rhs_data[8] = 200;
        rhs_data[7] = rhs_stats.no_data_value as u32;
        rhs_data[6] = rhs_stats.no_data_value as u32;
        let rhs_path = create_test_raster("rhs.tif", &rhs_stats, &rhs_data ).unwrap();

        let output = get_temp_filename("add_result.tif");

        if output.exists() {
            remove_file(&output).unwrap();
        }

        assert!(!output.exists());

        let nodata_output = -999999.0;
        combine_rasters::<i32,_>(&lhs_path, &rhs_path, &output, nodata_output, |v1, is_nodata1, v2, is_nodata2| {

            if is_nodata1 && is_nodata2 {
                return Ok(nodata_output as i32);
            }

            println!("{} + {} = ...  nd1 {} nd2 {}", v1, v2, is_nodata1, is_nodata2);

            if is_nodata1 {
                return Ok(v2);
            }

            if is_nodata2 {
                return Ok(v1);
            }

            Ok(v1 + v2)

        }).unwrap();

        assert!(output.exists());

        let output_raster = Raster::read(&output, true);

        let data: Vec<i32> = output_raster.dataset.rasterband(1).unwrap().read_as( (0,0), (lhs_stats.num_cols as i32, lhs_stats.num_rows as i32)).unwrap();

        assert_eq!(data[0], 9);
        //nodata on right
        assert_eq!(data[5], rhs_data[5] as i32);
        assert_eq!(data[6], lhs_data[6] as i32);
        assert_eq!(data[7], nodata_output as i32);
        assert_eq!(data[8], 150);
    }
}
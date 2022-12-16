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
use std::time::Instant;
use geo_util::raster::Raster;
use geo_util::util::{RasterChunkIterator, format_duration, print_remaining_time};
use format_num::NumberFormat;
use ndarray::{Array2, Zip};
use rayon::iter::IntoParallelIterator;
use rayon::iter::IntoParallelRefIterator;
use rayon::iter::ParallelIterator;
use gdal::metadata::Metadata;
use gdal::version_info;
use crate::lhs_rhs_args::LhsRhsArgs;

/// Diff stats between 2 rasters
pub fn print_stats(args: &LhsRhsArgs) -> Result<()> {

    let now = Instant::now();
    let mut last_output = Instant::now();

    let version_text = version_info("--version");

    println!("GDAL version: {}", version_text);

    //let path = Path::new(r"/data/TCD_working/tcd_rpe_2020_08.tif");
    //

    let raster_lhs = Raster::read(&args.raster_lhs, true);

    //let path_old = Path::new(r"/data/tcd_rpe_2020_07.tif");
    let raster_rhs = Raster::read(&args.raster_rhs, true);

    println!("dataset description: {:?}\n", raster_lhs.dataset.description());

    println!("Stats Left: {}\n", raster_lhs.stats);
    println!("Stats Right: {}\n", raster_rhs.stats);

    let offsets = raster_rhs.stats.common_offsets(&raster_lhs.stats);

    let mut total_diff = 0f64;
    let mut total_sum_rhs = 0f64;
    let mut total_sum_lhs = 0f64;

    //10 is the number of slices, so this will be 100 windows
    for raster_window in RasterChunkIterator::new(offsets.num_rows as i32, offsets.num_cols as i32, 10)
    {
        let window_size = raster_window.window_size;
        let (start_x, _stop_x) = raster_window.x_range_inclusive;
        let (start_y, _stop_y) = raster_window.y_range_inclusive;

        if let (Ok(rv_lhs), Ok(rv_rhs)) = (
            raster_lhs.band().read_as_array::<f64>((start_x + offsets.offset_x_2 as i32,
                                                    start_y + offsets.offset_y_2 as i32), window_size),
            raster_rhs.band().read_as_array::<f64>(
                (start_x + offsets.offset_x_1 as i32,
                 start_y + offsets.offset_y_1 as i32), window_size) )
        {
            let sum_lhs = calc_sum_par(&rv_lhs, raster_lhs.stats.no_data_value);
            let sum_rhs = calc_sum_par(&rv_rhs, raster_rhs.stats.no_data_value);
            let diff = calc_diff_par(
                &rv_rhs, &rv_lhs,
                raster_rhs.stats.no_data_value,
                raster_lhs.stats.no_data_value

            );

            total_sum_lhs += sum_lhs;
            total_sum_rhs += sum_rhs;

            total_diff += diff;

            /*
            println!("\nIn thread {:?}\nx {} to {}\ny {} to {}\nSum old {:.2} new {:.2} Diff {:.2}",
                thread::current().id(),
                        start_x, stop_x,
                        start_y, stop_y,
                        sum_old, sum_new, diff); */
        }

        if last_output.elapsed().as_secs() >= 3 {
            last_output = Instant::now();

            print_remaining_time(&now, raster_window.current_step as u32, raster_window.num_steps as u32);
        }
    }


    let num = NumberFormat::new();
    let num_format_str = ",.2f";
    println!("\nTOTAL\nSum LHS {}\nSum RHS {}\nPixel wise Diff {}\nAbs pop diff {}",
             num.format(num_format_str, total_sum_lhs),
             num.format(num_format_str, total_sum_rhs),
             num.format(num_format_str, total_diff),
             num.format(num_format_str,  (total_sum_rhs - total_sum_lhs).abs() )
    );

    println!("Finished in {}", format_duration(now.elapsed()));

    //https://gdal.org/api/gdalrasterband_cpp.html#classGDALRasterBand_1a75d4af97b3436a4e79d9759eedf89af4

    Ok(())
}


fn calc_sum_par(arr: &Array2<f64>, no_data_value: f64) -> f64 {
    arr.par_iter().cloned().reduce( || 0f64, |a, b| {
            let a_val = get_value(a, no_data_value);
            let b_val = get_value(b, no_data_value);

            a_val + b_val
        })
}

fn calc_diff_par(arr1: &Array2<f64>, arr2: &Array2<f64>, no_data_1: f64, no_data_2: f64) -> f64 {
    Zip::from(arr1).and(arr2).into_par_iter().fold( || 0f64, |acc, (a,b)| {
        let a_val = get_value(*a, no_data_1);
        let b_val =  get_value(*b, no_data_2);
        acc + (a_val - b_val).abs()
    }).sum::<f64>()
}


#[inline]
/// Nodata is considered to be 0
fn get_value(val: f64, no_data_value: f64) -> f64 {

    if !val.is_finite() {
        return 0.0;
    }

    assert!(no_data_value.is_finite());

    let no_data_diff = (val - no_data_value).abs();

    assert!(no_data_diff.is_finite());

    let eps = no_data_value.abs() / 10000.0 + f64::EPSILON * 10.0;

    if no_data_diff < eps {
        return 0.0
    } else {
        val
    }
}

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
use std::fs::remove_file;
use std::path::PathBuf;
use structopt::StructOpt;
use crate::lhs_rhs_args::LhsRhsArgs;

use geo_util::raster::combine_rasters::combine_rasters;

#[derive(StructOpt)]
pub struct RatioArgs {

    #[structopt(flatten)]
    lhs_rhs: LhsRhsArgs,

    #[structopt(long, parse(from_os_str))]
    output: PathBuf,

    #[structopt(long)]
    clean: bool,

}

pub fn create_ratio_raster(args: &RatioArgs) -> Result<()> {

    if args.clean && args.output.exists() {
        remove_file(&args.output)?;
    }

    if args.output.exists() {
        println!("{:?} already exists and --clean not passed, doing nothing", &args.output);
        return Ok(());
    }

    let output_nodata = -100000000.;
    combine_rasters(
        &args.lhs_rhs.raster_lhs,
        &args.lhs_rhs.raster_rhs,
        &args.output,
        output_nodata,
        |v1: f64, is_nodata1, v2, is_nodata2| {
            if is_nodata1 || is_nodata2 {
                return Ok(output_nodata);
            }

            Ok(v1 / v2)
        }
    )
}


#[cfg(test)]
mod cmdline_tools_ration_tests {
    use float_cmp::{ApproxEq, F64Margin, };
    use num::traits::float::Float;

    use gdal::raster::types::GdalType;
    use gdal::spatial_ref::SpatialRef;
    //use itertools::Itertools;
    use geo_util::raster::{create_test_raster, get_temp_filename, RasterStats, Raster};

    use super::*;

     #[test]
    fn test_ratio() {
         let srs = SpatialRef::from_epsg(4326).unwrap();

        let origin_y = 46.242485;
        let origin_x = 6.021557;

        let lhs_stats = RasterStats {
            origin_y,
            origin_x,
            pixel_height: -0.005,
            pixel_width: 0.004,
            num_rows: 2,
            num_cols: 2,
            no_data_value: -1000.0,
            gdal_type: f32::gdal_type(),
            projection: srs.to_wkt().unwrap()
        };

        let mut rhs_stats = lhs_stats.clone();
        rhs_stats.gdal_type = f64::gdal_type();
        rhs_stats.no_data_value = 10000.0;

        let lhs_data: Vec<f32> = vec![
            1., lhs_stats.no_data_value as f32,
            3., 4.
        ];

        let rhs_data: Vec<f64> = vec![
            -2., -2.,
            rhs_stats.no_data_value, 5.
        ];

        let lhs_path = create_test_raster("lhs.tif", &lhs_stats, &lhs_data ).unwrap();
        let rhs_path = create_test_raster("rhs.tif", &rhs_stats, &rhs_data ).unwrap();

        let output = get_temp_filename("ratio_result.tif");

        if output.exists() {
            remove_file(&output).unwrap();
        }

        assert!(!output.exists());

        create_ratio_raster(
            &RatioArgs {
                lhs_rhs: LhsRhsArgs {
                    raster_lhs: lhs_path,
                    raster_rhs: rhs_path
                },
                output: output.clone(),
                clean: false
            }
        ).unwrap();

        assert!(output.exists());

        let or = Raster::read(&output, true);
        let data: Vec<f64> = or.band().read_as((0,0), (2,2)).unwrap();

        let margin = F64Margin{ epsilon:  10. * f64::epsilon(), ulps: 3 };
        assert!( data[0]
        .approx_eq(-0.5, margin));
        assert!( data[1]
        .approx_eq(or.stats.no_data_value, margin));
        assert!( data[2]
        .approx_eq(or.stats.no_data_value, margin));
        assert!( data[3]
        .approx_eq(0.8, margin));

    }
}
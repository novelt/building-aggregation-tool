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
use std::path::PathBuf;
use std::fs::{remove_file, File};
use std::cmp::min;
use structopt::StructOpt;
use std::io::Write;
use crate::lhs_rhs_args::LhsRhsArgs;
use geo_util::raster::combine_rasters::combine_rasters;

#[derive(StructOpt)]
pub struct DiffArgs {

    #[structopt(flatten)]
    lhs_rhs: LhsRhsArgs,

    #[structopt(long, parse(from_os_str))]
    output: PathBuf,

    #[structopt(long)]
    clean: bool,

    #[structopt(long, parse(from_os_str))]
    color_ramp: Option<PathBuf>,

    #[structopt(long="equal", default_value="1")]
    consider_equal: f32
}


pub fn create_diff_raster(args: &DiffArgs) -> Result<()> {

    if args.clean && args.output.exists() {
        remove_file(&args.output)?;
    }

    if args.output.exists() {
        println!("{:?} already exists and --clean not passed, doing nothing", &args.output);
        return Ok(());
    }

    assert!(args.consider_equal >= 0.0);

    let new_stats_no_data_value : f32 = -1e-10;

    let mut all_values = Vec::new();

    combine_rasters(
        &args.lhs_rhs.raster_lhs,
        &args.lhs_rhs.raster_rhs,
        &args.output,
        new_stats_no_data_value as f64,
        |v1: f32, is_left_nodata, v2, is_right_nodata| {
            let is_left_0 = v1.abs() < 0.000_1;
            let is_right_0 = v2.abs() < 0.000_1;

            let diff =
                if (is_left_nodata || is_left_0) && (is_right_nodata || is_right_0) {
                    new_stats_no_data_value
                } else if is_left_nodata {
                    -v2
                } else if is_right_nodata {
                    v1
                } else {
                    v1 - v2
                };

            if diff != new_stats_no_data_value && diff.abs() > args.consider_equal {
                all_values.push(diff);
            }

            Ok(diff)
        }
    )?;


    let mut pos = Vec::new();
    let mut neg = Vec::new();

    for v in all_values {
        if v < 0.0 {
            neg.push(v)
        } else if v > 0.0 {
            pos.push(v);
        }
    }

    neg.sort_by(|a, b| a.partial_cmp(b).unwrap());
    pos.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let max_num_chunks = 155;
    let pos_chunks = min(pos.len(), max_num_chunks);
    let neg_chunks = min(neg.len(), max_num_chunks);

    if let Some(cr_path) = &args.color_ramp {
        if cr_path.exists() {
            remove_file(&cr_path)?;
        }

        let mut cr_file = File::create(&cr_path)?;

        cr_file.write(
            "# QGIS Generated Color Map Export File\n\
                 INTERPOLATION:DISCRETE\n".as_bytes())?;

        //https://www.color-hex.com/color-palette/49436
        for nc in 0..neg_chunks {
            let idx = nc as f64 / neg_chunks as f64 * (neg.len() - 1) as f64;
            let idx = idx.floor() as usize;
            let value = neg[idx];
            //start with most negative value
            let alpha = 255 - nc;
            cr_file.write(format!("{},213,94,0,{},<= {}\n",
                                  value, alpha, value
            ).as_bytes())?;
        }

        //less than a bit above 0 = green
        cr_file.write(format!("{a},0,158,115,255, Equal (-{a} <= value <= {a})\n", a=args.consider_equal).as_bytes())?;

        for nc in 0..pos_chunks {
            let idx = nc as f64 / pos_chunks as f64 * (pos.len() - 2) as f64;
            let idx = idx.ceil() as usize + 1;
            let value = pos[idx];
            let alpha = (255-pos_chunks) + nc;
            cr_file.write(format!("{},240,228,66,{},<= {}\n",
                                  value, alpha, value
            ).as_bytes())?;
        }

    }

    //https://gdal.org/api/gdalrasterband_cpp.html#classGDALRasterBand_1a75d4af97b3436a4e79d9759eedf89af4

    Ok(())
}


#[cfg(test)]
mod cmdline_tools_diff_tests {
    use float_cmp::{ApproxEq,  F32Margin};
    use num::traits::float::Float;

    use gdal::raster::types::GdalType;
    use gdal::spatial_ref::SpatialRef;
    //use itertools::Itertools;
    use geo_util::raster::{create_test_raster, get_temp_filename, RasterStats, Raster};

    use super::*;
    use std::fs::create_dir_all;


    #[test]
    fn test_diff() {
         let srs = SpatialRef::from_epsg(4326).unwrap();

        let origin_y = 46.242485;
        let origin_x = 6.021557;

        let lhs_stats = RasterStats {
            origin_y,
            origin_x,
            pixel_height: -0.005,
            pixel_width: 0.004,
            num_rows: 3,
            num_cols: 2,
            no_data_value: -1000.0,
            gdal_type: f32::gdal_type(),
            projection: srs.to_wkt().unwrap()
        };

        let mut rhs_stats = lhs_stats.clone();
        rhs_stats.no_data_value = 10000.0;

        let lhs_data: Vec<f32> = vec![
            1., lhs_stats.no_data_value as f32,
            3., 0.,
            lhs_stats.no_data_value as f32, lhs_stats.no_data_value as f32,
        ];

        let rhs_data: Vec<f32> = vec![
            -7., -2.,
            rhs_stats.no_data_value as f32, 5.,
            0.0000001, rhs_stats.no_data_value as f32
        ];

        let lhs_path = create_test_raster("lhs.tif", &lhs_stats, &lhs_data ).unwrap();
        let rhs_path = create_test_raster("rhs.tif", &rhs_stats, &rhs_data ).unwrap();

        let output = get_temp_filename("diff_result.tif");
        let color_ramp = get_temp_filename("color_ramp.txt");

        //temp directories are unique
        assert!(!output.exists());
        assert!(!color_ramp.exists());

        create_dir_all(&output.parent().unwrap()).unwrap();
        create_dir_all(&color_ramp.parent().unwrap()).unwrap();

        assert!(!output.exists());

        create_diff_raster(
            &DiffArgs {
                lhs_rhs: LhsRhsArgs {
                    raster_lhs: lhs_path,
                    raster_rhs: rhs_path
                },
                output: output.clone(),
                clean: false,
                color_ramp: Some(color_ramp.clone()),
                consider_equal: 0.0
            }
        ).unwrap();

        assert!(output.exists());

        let or = Raster::read(&output, true);
        let data: Vec<f32> = or.band().read_as((0,0), (2,3)).unwrap();

        println!("{:?}", data);

        let margin = F32Margin{ epsilon:  10. * f32::epsilon(), ulps: 3 };
        assert!( data[0]
        .approx_eq(lhs_data[0] - rhs_data[0], margin));
        assert!( data[1]
        .approx_eq(- rhs_data[1], margin));
        assert!( data[2]
        .approx_eq(lhs_data[2], margin));
        assert!( data[3]
        .approx_eq(lhs_data[3] - rhs_data[3], margin));

        //if both are 0 or nodata, the output is nodata
        assert!( data[4]
        .approx_eq(or.stats.no_data_value as f32, margin));
        assert!( data[5]
        .approx_eq(or.stats.no_data_value as f32, margin));

    }
}
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
use geo_util::util::{print_remaining_time, RasterChunkIterator};
use std::collections::HashMap;
use geo_util::raster::{Raster};
use std::fs::{File, remove_file};
use std::io::{BufWriter, Write};
use std::path::{PathBuf};
use std::time::Instant;
use anyhow::Result;
use structopt::StructOpt;
use itertools::Itertools;
use gdal::raster::types::GdalType;

/// Produces a CSV with feature id, the count (how many squares had data), and the sum

#[derive(StructOpt)]
struct Cli {

    #[structopt(parse(from_os_str), long, help="Feature raster")]
    feature_raster: PathBuf,

    #[structopt(parse(from_os_str), long, help="Data raster")]
    data_raster: PathBuf,

    #[structopt(parse(from_os_str), long, help="Path to CSV results")]
    summary_csv: PathBuf,

    #[structopt(long)]
    clean: bool,
}

fn main() {
    let args = Cli::from_args();
    run(&args).unwrap();
}

fn run(args: &Cli) -> Result<()> {

    let feature_raster = Raster::read(&args.feature_raster, true);

    //Only support unsigned types, nodata is usually 0
    assert!(
        feature_raster.stats.gdal_type == u8::gdal_type() ||
        feature_raster.stats.gdal_type == u16::gdal_type() ||
        feature_raster.stats.gdal_type == u32::gdal_type()
    );

    let data_raster = Raster::read(&args.data_raster, true);

    feature_raster.stats.assert_equals_except_no_data(&data_raster.stats);

    if args.clean && args.summary_csv.exists() {
        remove_file(&args.summary_csv)?
    }

    if args.summary_csv.exists() {
        println!("{:?} already exists, nothing to do", &args.summary_csv);
        return Ok(());
    }

    let f = File::create(&args.summary_csv).expect("Unable to create file");
    let mut f = BufWriter::new(f);

    let step_size = 2500;

    let now = Instant::now();
    let mut last_output = Instant::now();

    //feature_id => pop,count
    let mut group_by: HashMap<u32, (f64, u32)> = HashMap::new();

    let feature_nodata = feature_raster.stats.no_data_value as u32;

    for raster_window in RasterChunkIterator::new(feature_raster.stats.num_rows as i32, feature_raster.stats.num_cols as i32, step_size)
    {
        let window_size = raster_window.window_size;
        //println!("X {} to {}, Y {} to {}, window size: {:?}", start_x, stop_x, start_y, stop_y, window_size);

        //admin id
        let feature_ids = feature_raster.band().read_as_array::<u32>(raster_window.window_offset, window_size).unwrap();
        let data_values = data_raster.band().read_as_array::<f64>(raster_window.window_offset, window_size).unwrap();


        for col in 0..window_size.0 as usize {
            for row in 0..window_size.1 as usize {
                let idx = (row, col);
                let data_value = data_values.get(idx).unwrap();
                let feature_id = feature_ids.get(idx).unwrap();

                if *feature_id == feature_nodata {
                    continue;
                }

                if data_raster.stats.is_nodata(*data_value) {
                    continue;
                }

                let hm_val = group_by.entry(*feature_id).or_insert( (0.0, 0) );

                hm_val.0 += data_value;
                hm_val.1 += 1;

            }
        }

        if last_output.elapsed().as_secs() >= 1 {
            last_output = Instant::now();
            print_remaining_time(&now, raster_window.current_step as _, raster_window.num_steps as u32);
        }

    }

    for feature_id in group_by.keys().sorted() {

        let (pop_cell_value, count) = group_by[feature_id];

        f.write(format!("{}, {}, {}\n",
                    feature_id, count, pop_cell_value).as_bytes()).unwrap();
    }

    Ok(())
}


#[cfg(test)]
mod zonal_stats_test {
    use super::*;
    use gdal::spatial_ref::SpatialRef;
    use gdal::raster::types::GdalType;
    use geo_util::raster::{RasterStats, create_test_raster, get_temp_filename};
    use std::fs::{read_to_string, create_dir_all};

    #[test]
    fn test_zonal_stats_f32() {
        let srs = SpatialRef::from_epsg(4326).unwrap();

        let origin_y = 46.242485;
        let origin_x = 6.021557;

        let data_raster_stats = RasterStats {
            origin_y,
            origin_x,
            pixel_height: -0.005,
            pixel_width: 0.004,
            num_rows: 3,
            num_cols: 3,
            no_data_value: -1000.0,
            gdal_type: f32::gdal_type(),
            projection: srs.to_wkt().unwrap()
        };

        let mut feature_raster_stats = data_raster_stats.clone();
        feature_raster_stats.no_data_value = 0.;
        feature_raster_stats.gdal_type = u16::gdal_type();

        let data_raster = vec![
            1., data_raster_stats.no_data_value as f32, 3.,
            4., 5., 6.,
            7., data_raster_stats.no_data_value as f32, 9.5
        ];
        let feature_raster = vec![
            1, 1, 0,
            1, 1, 2,
            0, 0, 2,
        ];

        let args = Cli {
            feature_raster: create_test_raster("feature.tif", &feature_raster_stats, &feature_raster).unwrap(),
            data_raster: create_test_raster("data.tif", &data_raster_stats, &data_raster).unwrap(),
            summary_csv: get_temp_filename("summary.csv"),
            clean: false
        };

        if args.summary_csv.exists() {
            remove_file(&args.summary_csv).unwrap();
        }

        assert!(!args.summary_csv.exists());

        create_dir_all(&args.summary_csv.parent().unwrap() ).unwrap();

        run(&args).unwrap();

        assert!(args.summary_csv.exists());

        let summary_csv_data = read_to_string(&args.summary_csv).unwrap();

        //feature #1 had 3 squares, total 10
        //feature #2 has 2 squares, total 15.5
        assert_eq!(summary_csv_data, "\
        1, 3, 10\n\
        2, 2, 15.5\n\
        ");

    }
}
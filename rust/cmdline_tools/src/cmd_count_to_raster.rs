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
//Counts points or centroids and outputs a raster
use structopt::StructOpt;
use anyhow::Result;
use std::path::PathBuf;
use std::time::Instant;
use std::fs::remove_file;
use itertools::Itertools;
use geo_util::raster::{RasterStats, Raster, create_empty_raster};
use geo_util::util::{RasterChunkIterator, print_remaining_time};
use gdal::vector::Dataset;
use gdal::raster::types::GdalType;
use gdal::vector::OGRwkbGeometryType::wkbPoint;

///
/// Burns counts of input vector layer to raster
/// Projection of raster will be the same as the input
#[derive(StructOpt)]
pub struct CountToRasterCli {

    #[structopt(short="c", long = "ogr-connection")]
    ogr_conn_string: Vec<String>,

    #[structopt(short="l", long)]
    ogr_layer_name: Vec<String>,

    #[structopt(long, parse(from_os_str))]
    output_tif: PathBuf,

    #[structopt(long)]
    clean: bool,

    #[structopt(parse(from_os_str),long)]
    pub snap_raster: Option<std::path::PathBuf>,

    //to override any of the snap raster values
    #[structopt(help = "New Y origin, blank to keep the same.  In output_tif coordinates", short="y", long)]
    pub origin_y: Option<f64>,

    #[structopt(help = "New X origin, blank to keep the same", short="x", long)]
    pub origin_x: Option<f64>,

    #[structopt(help = "Num Cols", short="c", long)]
    pub num_cols: Option<u32>,

    #[structopt(help = "Num Rows", short="r", long)]
    pub num_rows: Option<u32>,

    #[structopt(help = "Pixel height", short="h", long)]
    pub pixel_height: Option<f64>,

    #[structopt(help = "Pixel width", short="w", long)]
    pub pixel_width: Option<f64>,

}


fn get_output_stats(args: &CountToRasterCli) -> Result<RasterStats> {
    //Either use the provided snap raster or default to the input tif values
    let mut stats: RasterStats = if let Some(sr) = args.snap_raster.as_ref() {
        let snap_raster = Raster::read(sr, true);
        snap_raster.stats.clone()
    } else {
        Default::default()
    };

    if let Some(o_x) = args.origin_x {
        stats.origin_x = o_x;
    }
    if let Some(o_y) = args.origin_y {
        stats.origin_y = o_y;
    }
    if let Some(nc) = args.num_cols {
        stats.num_cols = nc;
    }
    if let Some(nr) = args.num_rows {
        stats.num_rows = nr;
    }
    if let Some(ph) = args.pixel_height {
        stats.pixel_height = ph;
    }
    if let Some(pw) = args.pixel_width {
        stats.pixel_width = pw;
    }

    Ok(stats)
}

pub fn burn_count_to_raster(args: &CountToRasterCli) -> Result<()>
{
    println!("Starting");

    let now = Instant::now();
    let mut last_output = Instant::now();


    if args.output_tif.is_file() {
        if args.clean {
            remove_file(&args.output_tif)?;
        } else {
            panic!("{:?} exists already and --clean is not specified", args.output_tif);
        }
    }

    assert!(!args.output_tif.is_file());

    let input_datasets = args.ogr_conn_string.iter().map(|c| Dataset::open(&c).unwrap() ).collect_vec();
    let input_layers = input_datasets.iter().enumerate().map(|(idx, d)| d.layer_by_name(&args.ogr_layer_name[idx]).unwrap()).collect_vec();

    let mut output_stats = get_output_stats(&args)?;
    //output_stats.projection = input_sr.to_wkt()?;
    output_stats.gdal_type = i32::gdal_type();
    output_stats.no_data_value = 0.0;

    println!("Going to resize/resample to {}",
        &output_stats
    );

    assert!(output_stats.pixel_height < 0.0);

    create_empty_raster(&args.output_tif, &output_stats, false)?;

    assert!(args.output_tif.is_file());

    let output_raster = Raster::read(&args.output_tif, false);
    let output_raster_band = output_raster.band();

    print!("Processing {} layers", input_datasets.len());

    for raster_window in RasterChunkIterator::<i32>::new(
        output_raster.stats.num_rows as _,
        output_raster.stats.num_cols as _, 10)
    {

        let output_x_coords = [
            output_raster.stats.calc_x_coord(raster_window.x_range_inclusive.0),
            output_raster.stats.calc_x_coord(1 + raster_window.x_range_inclusive.1)
        ];
        let output_y_coords = [
            output_raster.stats.calc_y_coord(1 + raster_window.y_range_inclusive.1),
            output_raster.stats.calc_y_coord(raster_window.y_range_inclusive.0)
        ];
        assert!(output_x_coords[0] < output_x_coords[1]);
        assert!(output_y_coords[0] < output_y_coords[1]);

        let mut count_data = vec![0; (raster_window.window_size.0 * raster_window.window_size.1) as usize];

        for input_layer in input_layers.iter() {
            input_layer.set_spatial_filter_rect(output_x_coords[0], output_y_coords[0], output_x_coords[1], output_y_coords[1]);

            for (_feature_idx, input_feature) in input_layer.features().enumerate() {
                let shape = input_feature.geometry().as_geom();

                let (x, y) = if shape.geometry_type() == wkbPoint {
                    let [x, y] = shape.get_point(0);
                    (x, y)
                } else {
                    let centroid = shape.centroid()?;
                    let [x, y] = centroid.get_point(0);
                    (x, y)
                };

                let raster_x = output_raster.stats.calc_x(x);
                let raster_y = output_raster.stats.calc_y(y);

                let window_column_x = raster_x - raster_window.window_offset.0;
                let window_row_y = raster_y - raster_window.window_offset.1;

                if window_column_x >= 0 && window_column_x < raster_window.window_size.0 {
                    if window_row_y >= 0 && window_row_y < raster_window.window_size.1 {
                        let idx = (window_row_y * raster_window.window_size.0 + window_column_x) as usize;
                        count_data[idx] += 1;
                    }
                }
            }
        }

        output_raster_band.write(raster_window.window_offset, raster_window.window_size, &count_data)?;

        if last_output.elapsed().as_secs() >= 3 {
            last_output = Instant::now();

            print_remaining_time(&now, raster_window.current_step as _, raster_window.num_steps as _);
        }

    }

    Ok(())
}
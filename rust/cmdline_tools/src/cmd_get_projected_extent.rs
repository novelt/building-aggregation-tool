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
use std::fs;
use anyhow::Result;
use std::path::PathBuf;
use std::time::Instant;
use structopt::StructOpt;
use gdal::spatial_ref::{CoordTransform, OSRAxisMappingStrategy, SpatialRef};
use gdal::vector::Dataset;
use geo_util::util::print_remaining_time;

//Because extents aren't true rectanges and can be warped in other projections
//We need to project each feature to know the true projected extent

#[derive(StructOpt)]
pub struct GetProjectedExtentArgs {

    #[structopt(long, short="c", help = "OGR Connection string for inputs")]
    pub(crate) in_ogr_conn: Vec<String>,

    #[structopt(long, short="l", help = "Layer names for input, use - to use default, and all for everything")]
    pub(crate) in_ogr_layer: Vec<String>,

    #[structopt(long, help = "Target WKT string")]
    output_proj: String,

    #[structopt(parse(from_os_str), long)]
    output_file: PathBuf
}


fn float_max(f1: f64, f2: f64) -> f64 {
    if f1 > f2 {
        return f1;
    }
    return f2;
}

fn float_min(f1: f64, f2: f64) -> f64 {
    if f1 < f2 {
        return f1;
    }
    return f2;
}

pub fn get_projected_extent(args: &GetProjectedExtentArgs) -> Result<()> {

    let mut target_sr = SpatialRef::from_wkt(&args.output_proj).unwrap();
    target_sr.set_axis_mapping_strategy(OSRAxisMappingStrategy::OAMS_TRADITIONAL_GIS_ORDER);

    let mut x_min = f64::MAX;
    let mut y_min = f64::MAX;
    let mut x_max = f64::MIN;
    let mut y_max = f64::MIN;

    let mut last_output = Instant::now();

    for input_idx in 0..args.in_ogr_conn.len() {
        let input_ds = Dataset::open(&args.in_ogr_conn[input_idx])?;
        let input_lyr = input_ds.layer_by_name(&args.in_ogr_layer[input_idx])?;

        let input_sr = input_lyr.spatial_reference()?;
        let transform = CoordTransform::new(&input_sr, &target_sr).unwrap();

        let num_steps = input_lyr.count(false);

        let start = Instant::now();

        for (f_idx, f) in input_lyr.features().enumerate() {

            let g = f.geometry().as_geom();
            let xformed = g.transform(&transform)?;

            let env = xformed.envelope();

            x_min = float_min(x_min, env.MinX);
            y_min = float_min(y_min, env.MinY);

            x_max = float_max(x_max, env.MaxX);
            y_max = float_max(y_max, env.MaxY);

            if last_output.elapsed().as_secs() >= 3 {
                last_output = Instant::now();
                print_remaining_time(&start,
                                     f_idx as _,
                                     num_steps as _);
            }
        }
    }

    fs::write(&args.output_file, format!("{}\n{}\n{}\n{}", x_min, y_min, x_max, y_max))?;

    Ok(())
}
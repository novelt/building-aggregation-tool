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
use std::path::PathBuf;
use std::time::Instant;
use anyhow::{bail, Result};
use gdal::vector::{Dataset};
use structopt::StructOpt;

use geo_util::vector::{get_multi_poly_area};

use geo_util::convert::ToRustGeo;

use geo::{Geometry as GeoGeometry};
use geo::prelude::ChamberlainDuquetteArea;
use gdal::raster::types::GdalType;
use geo_util::io::get_sub_dir;
use geo_util::raster::{create_empty_raster, Raster};
use geo_util::util::print_remaining_time_msg;

#[derive(StructOpt)]
pub struct CountAreaTotalArgs {
    #[structopt(long, parse(from_os_str), help="FGBS will be read from this directory")]
    pub(crate) in_dir: PathBuf,

    #[structopt(long, parse(from_os_str), help="Rasters will be written to this directory")]
    pub(crate) out_dir: PathBuf,

    #[structopt(long, parse(from_os_str), help="For overall dimensions")]
    pub(crate) ref_raster: PathBuf,

    #[structopt(long)]
    pub(crate) out_raster_width: u32,

    #[structopt(long)]
    pub(crate) out_raster_height: u32,
}

pub fn count_area_total(args: &CountAreaTotalArgs) -> Result<()>
{
    let vec_size = (args.out_raster_height * args.out_raster_width) as usize;
    let mut total_area = vec![0.; vec_size];
    let mut total_count = vec![0; vec_size];

    let snap_raster = Raster::read(&args.ref_raster, true);
    let snap_raster_stats = &snap_raster.stats;

    let chunk_wh = snap_raster_stats.get_chunk_width_height_non_aligned(args.out_raster_height, args.out_raster_width);

    let mut last_output = Instant::now();

    for (sub_file_idx, sub_file) in fs::read_dir(&args.in_dir).unwrap().enumerate() {

        let sub_file = sub_file.unwrap();

        if !sub_file.path().is_file() {
            continue;
        }

        if sub_file.path().extension().unwrap().to_str().unwrap() != "fgb" {
            continue;
        }

        let conn = Dataset::open(sub_file.path().to_str().unwrap()).unwrap();
        let lyr = conn.layer_by_name(sub_file.path().file_stem().unwrap().to_str().unwrap()).unwrap();

        let spatial_ref = lyr.spatial_reference().unwrap();
        //spatial_ref.auto_identify_epsg()
        assert_eq!("EPSG", spatial_ref.auth_name().unwrap());
        assert_eq!(4326, spatial_ref.auth_code().unwrap());

        let now  = Instant::now();
        let total_to_process = lyr.count(false);

        for (f_idx,f) in lyr.features().enumerate() {

            if last_output.elapsed().as_secs() >= 3 {
                last_output = Instant::now();
                print_remaining_time_msg(&now, f_idx as _, total_to_process as u32,
                    &format!("In file {}", sub_file_idx+1)
                );
            }

            let geom = f.geometry().as_geom();

            //Find which chunk this belongs to
            let env = geom.envelope();

            let chunk_index = snap_raster_stats.get_chunk_index(&chunk_wh,
            args.out_raster_height, args.out_raster_width, &env);

            if chunk_index.is_none() {
                continue;
            }
            let chunk_index = chunk_index.unwrap();

            let geo_geom = geom.to_rust_geo();

            //Get the area in meters, target sr is 4326
            let area: f32 =

                match &geo_geom {
                    GeoGeometry::MultiPolygon(mp) => {
                        get_multi_poly_area(mp) as f32
                    }
                    GeoGeometry::Polygon(p) => {
                        p.chamberlain_duquette_unsigned_area() as f32
                    }
                    _ => { bail!("Not a mp nor polygon"); }
                };


            total_area[chunk_index] += area;
            total_count[chunk_index] += 1;
        }

        //println!("\n{}\n{}", total_area_m2, total_features);
    }

    println!("Creating raster files");

    let area_path = get_sub_dir(&args.out_dir, "area.tif");
    let count_path = get_sub_dir(&args.out_dir, "count.tif");

    let mut output_stats = snap_raster_stats.clone();
    output_stats.no_data_value = -1.;
    output_stats.num_rows = args.out_raster_height as _;
    output_stats.num_cols = args.out_raster_width as _;
    output_stats.pixel_width = chunk_wh.0;
    output_stats.pixel_height = chunk_wh.1;
    output_stats.gdal_type = f32::gdal_type();
    create_empty_raster(&area_path, &output_stats, false)?;

    output_stats.gdal_type = i32::gdal_type();
    create_empty_raster(&count_path, &output_stats, false)?;

    let area_raster = Raster::read(&area_path, false);

    area_raster.dataset.rasterband(1)?.write((0,0),
                                             (args.out_raster_width as i32,
                                             args.out_raster_height as i32),
        &total_area).unwrap();

    let count_raster = Raster::read(&count_path, false);
    count_raster.dataset.rasterband(1)?.write((0,0),
                                             (args.out_raster_width as i32,
                                             args.out_raster_height as i32),
        &total_count).unwrap();

    Ok(())
}
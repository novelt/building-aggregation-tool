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
use std::path::Path;
use std::time::Instant;

use anyhow::{Result};
//use itertools::Itertools;
use structopt::StructOpt;

use gdal::vector::{Dataset, Driver, OGRwkbGeometryType, Feature};
use geo_util::util::print_remaining_time;
use geo_util::convert::{convert_from_gdal_to_geos, convert_geos_to_gdal};
use crate::utm_proj_cache::UtmProj;
use crate::utm_proj_cache::UtmCache;
use geos::{ GeometryTypes, SimpleContextHandle, };
use geo_util::vector::{transform_geos_multipolygon, transform_geos};


mod utm_proj_cache;

#[derive(StructOpt)]
struct Cli {


    #[structopt(long)]
    in_ogr_conn: String,

    #[structopt( long)]
    in_ogr_layer: String,

    #[structopt(long)]
    out_ogr_conn: String,

    #[structopt( long)]
    out_ogr_layer: String,

    #[structopt(long, help="What is the output format")]
    out_driver: Option<String>,

    //#[structopt(long, help="EPSG code of output projection")]
    //out_proj: Option<u32>,

    //#[structopt(long, parse(from_os_str))]
    //work_dir: PathBuf,

    #[structopt( long)]
    buffer_meters: u16,

    #[structopt(long, default_value="8")]
    quad_segs: u8,
    //
    // #[structopt(long)]
    // clean: bool,
}

fn run() -> Result<()> {

    let args: Cli = Cli::from_args();

    let now = Instant::now();
    let mut last_output = Instant::now();

    //Open the hamlet area layer
    let in_dataset = Dataset::open(&args.in_ogr_conn)?;
    let in_layer = in_dataset.layer_by_name(&args.in_ogr_layer)?;

    let in_proj = in_layer.spatial_reference()?;

    //Gather some information about input layer
    let total = in_layer.count(false);

    //let input_columns = get_input_column_names(&args.in_ogr_conn, &args.in_ogr_layer)?;

    let output_driver_name = args.out_driver.as_ref().map_or(
        Driver::DRIVER_NAME_FLATGEOBUF, |od| &od );
    let drv = Driver::get(output_driver_name)?;

    let mut n_processed = 0;

    let ds =
        if Path::new(&args.out_ogr_conn).is_file() {
            drv.open(&args.out_ogr_conn, false)?
        } else {
            drv.create(&args.out_ogr_conn)?
        };

    let out_lyr = if !ds.layer_by_name(&args.out_ogr_layer).is_ok() {
        ds.create_layer_ext::<String>(
            &args.out_ogr_layer,
            &in_proj,
            OGRwkbGeometryType::wkbMultiPolygon,
            &vec![]
        )?
    } else {
        ds.layer_by_name(&args.out_ogr_layer)?
    };

    //add_columns_to_layer(&mut out_lyr, &input_columns);

    let out_def = out_lyr.layer_definition();

    let context_handle = SimpleContextHandle::new();
    context_handle.add_message_handlers();


    let mut utm_proj_cache = UtmCache::new(in_proj.clone() );

    for feature in in_layer.features() {

        // if n_processed % 1000 == 0 && n_processed > 0 {
        //     break;
        // }
        //f.geometry()

        //println!("Feature {}", feature.fid());

        //Find center point
        let shape = convert_from_gdal_to_geos(&feature.geometry().as_geom(),
                                                  &context_handle, false)?;

        //shape.set_srid(orig_srid);

        let envelope = shape.envelope()?;

        let center = envelope.center()?;

        let utm_proj = UtmProj::find_utm(center.0, center.1);

        let utm_trans = utm_proj_cache.get(&utm_proj);

        //println!("Envelope {} Center {:?} UTM Proj {}", envelope.to_wkt()?, center, utm_proj);

        let shape_meters = transform_geos(
                &context_handle,
                &utm_trans.transform_to_meters,
                &shape)?;

        //println!("Transformed Center {:?}", shape_meters.envelope()?.center());

        let mut buffered_shape = shape_meters.buffer(&context_handle,
    args.buffer_meters as _, args.quad_segs as _)?;

        if buffered_shape.geometry_type() == GeometryTypes::Polygon {
            buffered_shape = buffered_shape.polygon_to_multipolygon(&context_handle)?;
        } else {
            assert_eq!(buffered_shape.geometry_type(), GeometryTypes::MultiPolygon);
        }


        let buffered_shape_orig = transform_geos_multipolygon(
                &context_handle,
                &utm_trans.transform_to_source,
                &buffered_shape)?;


        let gdal_geom = convert_geos_to_gdal(&buffered_shape_orig)?;


        let mut out_ft = Feature::new(&out_def).unwrap();
        out_ft.set_geometry_directly(gdal_geom)?;

        // Copy fields over
        // for idx in 0..input_columns.len() {
        //     let input_field_value = feature.field_from_idx(idx as _)?;
        //     out_ft.set_field_by_index(idx as _, &input_field_value)?;
        // }

        // Add the feature to the layer:
        out_ft.create(&out_lyr)?;

        n_processed += 1;

        if last_output.elapsed().as_secs() >= 3 {
            last_output = Instant::now();
            print_remaining_time(&now, n_processed, total as _);
        }
    }

    Ok(())
}


fn main() {
    run().unwrap()
}

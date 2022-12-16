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
use std::path::{PathBuf};
use anyhow::{Result, bail};
use gdal::spatial_ref::{SpatialRef, OSRAxisMappingStrategy, };
use gdal::vector::{Driver, Feature, OGRwkbGeometryType, Dataset, Geometry};
use structopt::StructOpt;

use std::time::Instant;
use geo_util::io::{InputOgrLayer,};
use geo_util::util::print_remaining_time;
use itertools::Itertools;
use geo_util::raster::Raster;

/*
A faster geospatial filter

Any polygon whose extremes are inside the raster are filtered out,
everything else the shape is written to the output
 */

#[derive(StructOpt)]
pub struct FilterUsingRasterArgs {

    //Note this isn't generic with respect to the fields, so for now keeping osm & dg
    #[structopt(long, help="OGR Connection string for inputs")]
    pub(crate) in_ogr_conn: Vec<String>,

    #[structopt(long, help="Layer names for input, use - to use default, and all for everything")]
    pub(crate) in_ogr_layer: Vec<String>,


    #[structopt( long)]
    pub(crate) filter_raster_path: PathBuf,

    #[structopt( long)]
    pub(crate) out_ogr_conn: String,

    #[structopt( long)]
    pub(crate) out_ogr_layer: String,

}

fn build_input_ogr_layers(args: &FilterUsingRasterArgs) -> Result<Vec<InputOgrLayer>>
{
    assert_eq!(args.in_ogr_layer.len(), args.in_ogr_conn.len());

    assert!(args.in_ogr_conn.len() >= 1);

    let mut ret = Vec::new();

    for (i, ocs) in args.in_ogr_conn.iter().enumerate() {

        //open it to inspect the layers
        let dataset = Dataset::open(&ocs )?;

        let input_layer_name = &args.in_ogr_layer[i];

        let mut layer_names: Vec<String> = Vec::new();

        let layer_count = dataset.count();

        //assert!(!input_layer.layer_name.is_empty(), "Call try_set_empty_layer_name to set empty layer name");
        let mut at_least_one = false;
        for layer_index in 0..layer_count {
            let layer = dataset.layer(layer_index)?;
            let layer_name = layer.name();
            layer_names.push(layer.name());

            if !( (layer_index == 0 && input_layer_name == "-") ||
                (input_layer_name == "all") ||
                (input_layer_name == &layer_name) ) {
                continue;
            }

            at_least_one = true;

            ret.push(InputOgrLayer {
                    name: layer_name.clone(),
                    layer_name,
                    ogr_conn_str: ocs.clone(),
                    attribute_filter: None,
                    layer_creation_option: vec![],
                    ogr_format: None
                });

        }

        if !at_least_one {
            let layer_names = layer_names.iter().sorted().join(", ");
            bail!("No layer name found {:?} in dataset: {}.  Found layers: {}",
                                   input_layer_name, ocs, layer_names);
        }

    }

    Ok(ret)
}




/// Combines building layers, in order of priority, not including ones that intersect an already existing building
pub fn filter_using_raster(args: &FilterUsingRasterArgs) -> Result<()> {
    println!("Starting merging vector layers");

    let now  = Instant::now();
    let mut last_output = Instant::now();

    let mut target_sr = SpatialRef::from_epsg(4326).unwrap();
    target_sr.set_axis_mapping_strategy(OSRAxisMappingStrategy::OAMS_TRADITIONAL_GIS_ORDER);

    let ogr_inputs = build_input_ogr_layers(args)?;

    let drv = Driver::get(Driver::DRIVER_NAME_FLATGEOBUF)?;

    let mut n_processed = 0;
    let mut n_skipped = 0;

    let ds = drv.create(&args.out_ogr_conn)?;

    let output_lyr = ds.create_layer_ext::<String>(
        &args.out_ogr_layer,
        &target_sr,
        OGRwkbGeometryType::wkbMultiPolygon,
        &vec![]
    )?;


    let output_layer_def = output_lyr.layer_definition();

    let total_to_process = {
        let mut t = 0;
        for input in ogr_inputs.iter() {
            let ds = Dataset::open(&input.ogr_conn_str)?;
            let lyr = ds.layer_by_name(&input.layer_name)?;
            t += lyr.count(false);
        }
        t
    };

    let filter_raster = Raster::read(&args.filter_raster_path, true);
    let filter_band = &filter_raster.band();
    let filter_stats = &filter_raster.stats;

    let mut data: Vec<u8> = vec![0;9];

    for input in ogr_inputs.iter() {
        let ds = Dataset::open(&input.ogr_conn_str)?;
        let lyr = ds.layer_by_name(&input.layer_name)?;


        for input_feature in lyr.features()
        {
            if last_output.elapsed().as_secs() >= 3 {
                last_output = Instant::now();
                print_remaining_time(&now, n_processed, total_to_process as u32);
            }

            n_processed += 1;

            let input_geom = input_feature.geometry().as_geom();

            //assert!(input_geom.is_valid());
            assert_eq!(input_geom.geometry_type(), OGRwkbGeometryType::wkbMultiPolygon);

            let mut output_geom = Geometry::empty(OGRwkbGeometryType::wkbMultiPolygon)?;

            let poly_count = input_geom.geometry_count();

            for p in 0..poly_count {
                let poly = input_geom.get_geometry(p);
                let geom_env = poly.envelope();

                let mut skip = true;

                //check the extent of the building
                for (x, y) in [
                    (geom_env.MaxX, geom_env.MaxY),
                    (geom_env.MinX, geom_env.MaxY),
                    (geom_env.MaxX, geom_env.MinY),
                    (geom_env.MinX, geom_env.MinY),
                ].iter() {

                    //we'll check a 3x3 around the point
                    let x_coord = filter_stats.calc_x(*x);
                    let y_coord = filter_stats.calc_y(*y);

                    if x_coord - 1 < 0 || x_coord + 1 >= filter_stats.num_cols as i32 {
                        skip = false;
                        break;
                    }

                    if y_coord - 1 < 0 || y_coord + 1 >= filter_stats.num_rows as i32 {
                        skip = false;
                        break;
                    }

                    filter_band.read_into_vec(
                        (x_coord - 1, y_coord - 1),
                        (3, 3),
                        &mut data
                    ).unwrap();

                    //how many squares not in the flood fill
                    let outside_count = data.iter().filter(|d| **d <= 0).count();

                    //if any square is outside the flood fill, we don't skip
                    if outside_count > 0 {
                        skip = false;
                        break;
                    }
                }

                if skip {
                    n_skipped += 1;
                    continue;
                }

                output_geom.add_geometry(poly.clone()).unwrap();
            }

            let mut ft = Feature::new(&output_layer_def)?;

            ft.set_geometry(output_geom)?;
            ft.create(&output_lyr)?;

            // if n_processed % 10 == 0 {
            //     break;
            // }
        }
    }

    println!("Processed {} multipolygons, skipped {} polygons", n_processed , n_skipped);
    Ok(())
}


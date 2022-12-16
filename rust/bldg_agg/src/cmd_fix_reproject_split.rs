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
use anyhow::{bail, Result};
use gdal::spatial_ref::{SpatialRef, OSRAxisMappingStrategy, CoordTransform};
use gdal::vector::{Driver, Feature, OGRwkbGeometryType, Dataset, LayerDefinition, FieldDefinition, Layer};
use structopt::StructOpt;

use std::time::Instant;
use geo_util::io::{get_index_width_len, get_sub_dir};
use geo_util::util::{print_remaining_time_msg};
use log::{debug};
use geo_util::raster::Raster;
use geo_util::vector::get_fixed_geom;

/*
A faster geospatial filter

Any polygon whose extremes are inside the raster are filtered out,
everything else the shape is written to the output
 */

#[derive(StructOpt)]
pub struct FixReprojectSplitArgs {

    //Note this isn't generic with respect to the fields, so for now keeping osm & dg
    #[structopt(long, short="c", help="OGR Connection string for inputs")]
    pub(crate) in_ogr_conn: Vec<String>,

    #[structopt(long, short="l", help="Layer names for input, use - to use default, and all for everything")]
    pub(crate) in_ogr_layer: Vec<String>,

    #[structopt( long)]
    pub(crate) snap_raster_path: PathBuf,

    #[structopt(parse(from_os_str), long)]
    output_path: PathBuf,

    #[structopt( long, default_value="10")]
    pub chunk_rows: u32,

    #[structopt( long, default_value="10")]
    pub chunk_cols: u32,

    #[structopt(long, short="f", help="Which fields to copy over")]
    pub fields_to_copy: Vec<String>,

    #[structopt( long )]
    pub use_centroid: bool,
}

pub fn fix_reproject_split(args: &FixReprojectSplitArgs) -> Result<()>
{

    let mut last_output = Instant::now();

    //create all the output fgb datasets
    let vec_ds = build_output_datasets(args);

    let vec_lyr = build_output_layers(&vec_ds, args).unwrap();

    let vec_lyr_def = {
        let mut vec_lyr_def: Vec<LayerDefinition> = Vec::with_capacity(vec_lyr.len());
        for lyr in vec_lyr.iter() {
            vec_lyr_def.push(lyr.layer_definition());
        }
        vec_lyr_def
    };

    let snap_raster = Raster::read(&args.snap_raster_path, true);
    let snap_raster_stats = &snap_raster.stats;

    let chunk_width_height = snap_raster_stats.get_chunk_width_height(args.chunk_rows, args.chunk_cols);

    println!("Chunk width height: {:?}", chunk_width_height);

    let mut target_sr = SpatialRef::from_wkt(&snap_raster_stats.projection).unwrap();
    target_sr.set_axis_mapping_strategy(OSRAxisMappingStrategy::OAMS_TRADITIONAL_GIS_ORDER);

    //now that we have the output layers created, we start processing the input layers
    for input_lyr_idx in 0..args.in_ogr_conn.len() {
        let input_conn = Dataset::open(&args.in_ogr_conn[input_lyr_idx]).expect(
            &format!("Cannot open {}", &args.in_ogr_conn[input_lyr_idx])
        );
        let input_lyr = input_conn.layer_by_name(&args.in_ogr_layer[input_lyr_idx]).unwrap();

        //If the input has no CRS, assume its 4326
        let input_srs = input_lyr.spatial_reference().unwrap_or_else(|_|
            SpatialRef::from_epsg(4326).unwrap()
        );
        let transform = CoordTransform::new(&input_srs, &target_sr)?;

        let now  = Instant::now();
        let total_to_process = input_lyr.count(true);

        for (num_processed,input_feature) in input_lyr.features().enumerate() {

            let geom = input_feature.geometry().as_geom();

            let mut fixed_geom = get_fixed_geom(geom);

            if fixed_geom.geometry_type() != OGRwkbGeometryType::wkbMultiPolygon {

                //with the addition of settlement parts on raster square lines, this can be common
                // for i in 0..input_feature.field_count() {
                //     warn!("Field #{} / {} = {:?}", i,
                //         input_lyr.layer_definition().get_field(i).name(),
                //         input_feature.field_from_idx(i).unwrap())
                // }

                debug!("Problem, fixed geometry is not a multipolygon!!");
                continue;
            }

            if fixed_geom.transform_inplace(&transform).is_err() {
                debug!("Had a problem with {} {:?}", input_feature.fid(), input_feature.field("idx"));
                continue;
            }


            //assert!(fixed_geom.is_valid());

            let mut env = fixed_geom.envelope();

            if args.use_centroid {
                let centroid = fixed_geom.centroid().unwrap();
                let xy = centroid.get_point(0);
                env.MaxY = xy[1];
                env.MinY = xy[1];

                env.MaxX = xy[0];
                env.MinX = xy[0];
            }

            let chunk_index = snap_raster_stats.get_chunk_index(&chunk_width_height,
                                                                args.chunk_rows, args.chunk_cols, &env);

            if chunk_index.is_none() {
                debug!("Chunk index is none: chunk wh {:?} rows {} cols {} env: {:?}", chunk_width_height, args.chunk_rows, args.chunk_cols, env);
                continue;
            }

            let mut ft = Feature::new(&vec_lyr_def[chunk_index.unwrap()]).unwrap();

            //let wkt = fixed_geom.wkt().unwrap();

            ft.set_geometry_directly(fixed_geom)?;

            for f in args.fields_to_copy.iter() {
                let iv = input_feature.field(f)?;
                ft.set_field(f, &iv)?;
            }

            if ft.create(&vec_lyr[chunk_index.unwrap()]).is_err() {
                bail!("Problem creating {:?} num proc {}  fixed geom ", chunk_index, num_processed);
            }

            if last_output.elapsed().as_secs() >= 3 {
                last_output = Instant::now();
                print_remaining_time_msg(&now, num_processed as _, total_to_process as u32,
                    &format!("In layer {} of {}", input_lyr_idx+1, args.in_ogr_conn.len())
                );
            }
        }
    }

    Ok(())
}


fn build_output_datasets(args: &FixReprojectSplitArgs) -> Vec<Dataset> {
    let total_chunks = args.chunk_cols * args.chunk_rows;
    let drv = Driver::get(Driver::DRIVER_NAME_FLATGEOBUF).unwrap();
    let layer_index_padding = get_index_width_len(total_chunks as _);

    let mut vec_ds = Vec::new();

    //let mut vec_out_lyr = Vec::new();

    for chunk_idx in 0..total_chunks {
        let output_path = get_sub_dir(&args.output_path,
                                      format!("chunk_{:0width$}.fgb", chunk_idx, width = layer_index_padding));

        let ds = drv.create(&output_path.to_str().unwrap()).unwrap();

        vec_ds.push(ds);
    }

    vec_ds
}

fn build_output_layers<'a>(vec_ds: &'a Vec<Dataset>, args: &'a FixReprojectSplitArgs) -> Result< Vec<Layer<'a>> > {
    let total_chunks = args.chunk_cols * args.chunk_rows;
    let layer_index_padding = get_index_width_len(total_chunks as _);

    let mut target_sr = SpatialRef::from_epsg(4326).unwrap();
    target_sr.set_axis_mapping_strategy(OSRAxisMappingStrategy::OAMS_TRADITIONAL_GIS_ORDER);

    //We use the indexes from the 1st layer
    let input_conn = Dataset::open(&args.in_ogr_conn[0]).unwrap();
    let input_lyr = input_conn.layer_by_name(&args.in_ogr_layer[0]).unwrap();
    let input_lyr_def = input_lyr.layer_definition();

    let mut vec_lyr = Vec::with_capacity(vec_ds.len());
    for chunk_idx in 0..total_chunks {
        let output_path = get_sub_dir(&args.output_path,
                                      format!("chunk_{:0width$}.fgb", chunk_idx, width = layer_index_padding));

        let mut lyr = vec_ds[chunk_idx as usize].create_layer_ext::<String>(
            &output_path.file_stem().unwrap().to_str().unwrap(),
            &target_sr,
            OGRwkbGeometryType::wkbMultiPolygon,
            &[
                //"SPATIAL_INDEX=NO".to_string()
            ]
        ).unwrap();

        for f in args.fields_to_copy.iter() {

            let field_idx = input_lyr_def.get_field_index(f)?;
            let field = input_lyr_def.get_field(field_idx);

            let field_defn = FieldDefinition::new(f, field.field_type())?;
            field_defn.add_to_layer(&mut lyr)?;
        }

        vec_lyr.push(lyr);
    }

    Ok(vec_lyr)
}
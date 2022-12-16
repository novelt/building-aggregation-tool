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
use std::time::Instant;

use anyhow::{Result};
//use itertools::Itertools;
use structopt::StructOpt;

use gdal::vector::{Dataset, Driver, OGRwkbGeometryType, Feature, OGREnvelope, FieldDefinition, OGRFieldType, Geometry};
use geo_util::util::print_remaining_time;
use geo_util::convert::{convert_from_gdal_to_geos, convert_from_gdal_to_geos_no_holes, convert_geos_to_gdal, convert_geos_to_gdal_no_holes};

use geos::{SimpleGeometry, SimpleContextHandle, GeometryTypes};

use rstar::{RTree, AABB};

use crate::rtree_index_object::{FeatureKey, RTreeIndexObject};
use std::collections::{HashMap, HashSet};
use log::{debug, LevelFilter};
use simple_logger::SimpleLogger;
use geos::GeometryTypes::{GeometryCollection, MultiPolygon};


/*
 */

mod rtree_index_object;

#[derive(StructOpt)]
struct UnionArgs {
    //Note this isn't generic with respect to the fields, so for now keeping osm & dg
    #[structopt(long, short="c", help = "OGR Connection string for inputs")]
    pub(crate) in_ogr_conn: Vec<String>,

    #[structopt(long, short="l", help = "Layer names for input, use - to use default, and all for everything")]
    pub(crate) in_ogr_layer: Vec<String>,

    #[structopt(long)]
    out_ogr_conn: String,

    #[structopt(long)]
    out_ogr_layer: String,

    #[structopt(long, help = "What is the output format")]
    out_driver: Option<String>,

    #[structopt(long, default_value = "Warn")]
    log_level: LevelFilter,

    #[structopt(long)]
    no_holes: bool
}


fn run() -> Result<()> {
    let args: UnionArgs = UnionArgs::from_args();

    SimpleLogger::new().with_level(args.log_level).init()?;

    let now = Instant::now();
    let mut last_output = Instant::now();


    let in_proj = {
        let in_dataset = Dataset::open(&args.in_ogr_conn[0])?;
        let in_layer = in_dataset.layer_by_name(&args.in_ogr_layer[0])?;
        in_layer.spatial_reference()?
    };

    let output_driver_name = args.out_driver.as_ref().map_or(
        Driver::DRIVER_NAME_FLATGEOBUF, |od| &od);
    let drv = Driver::get(output_driver_name)?;

    let mut n_processed = 0;

    let ds = drv.create(&args.out_ogr_conn)?;

    let mut out_lyr = ds.create_layer_ext::<String>(
        &args.out_ogr_layer,
        &in_proj,
        OGRwkbGeometryType::wkbMultiPolygon,
        &[]
        // &vec!["GEOMETRY_NAME=shape".to_string(),
        // "FID=id".to_string()
        // ],
    )?;

    let field_defn = FieldDefinition::new("orig_fid", OGRFieldType::OFTInteger)?;
    field_defn.add_to_layer(&mut out_lyr)?;

    let mut orig_fid = 1;

    let out_def = out_lyr.layer_definition();

    let context_handle = SimpleContextHandle::new();
    context_handle.add_message_handlers();

    //let id_field_index = in_layer.layer_definition().get_field_index(&args.in_id_field).unwrap_or(-1);
    //let count_field_index = in_layer.layer_definition().get_field_index("count")?;

    //process input features in extents to limit the amount we need in memory

    let mut total_extent = OGREnvelope {
        MinX: f64::MAX,
        MaxX: f64::MIN,
        MinY: f64::MAX,
        MaxY: f64::MIN,
    };

    //Gather some information about input layer
    let mut total = 0;

    for (idx, ocs) in args.in_ogr_conn.iter().enumerate() {
        let in_dataset = Dataset::open(ocs)?;
        let in_layer = in_dataset.layer_by_name(&args.in_ogr_layer[idx])?;

        debug!("Extent for {}", idx);
        let extent = in_layer.get_extent(true).unwrap();

        total_extent.MinX = float_min(total_extent.MinX, extent.MinX);
        total_extent.MinY = float_min(total_extent.MinY, extent.MinY);
        total_extent.MaxX = float_max(total_extent.MaxX, extent.MaxX);
        total_extent.MaxY = float_max(total_extent.MaxY, extent.MaxY);

        total += in_layer.count(false);
    }

    //We want to iterate sub squares of the total extent
    let iterate_chunk_width = 10;

    let x_chunk_width = (total_extent.MaxX - total_extent.MinX) / iterate_chunk_width as f64;
    let y_chunk_width = (total_extent.MaxY - total_extent.MinY) / iterate_chunk_width as f64;

    let mut seen: HashSet<FeatureKey> = HashSet::new();

    let mut union_rtree: RTree<RTreeIndexObject> = RTree::new();
    let mut current_shapes: HashMap<FeatureKey, (RTreeIndexObject, SimpleGeometry)> = HashMap::new();

    //Only write those shapes which are fully contained in the chunk
    for x_chunk in 0..iterate_chunk_width {
        for y_chunk in 0..iterate_chunk_width {
            debug!("Processing chunk x {} y {}", x_chunk, y_chunk);

            let x = total_extent.MinX + x_chunk_width * x_chunk as f64;
            let y = total_extent.MinY + y_chunk_width * y_chunk as f64;

            for (idx, ocs) in args.in_ogr_conn.iter().enumerate() {
                let in_dataset = Dataset::open(ocs)?;
                let in_layer = in_dataset.layer_by_name(&args.in_ogr_layer[idx])?;

                in_layer.set_spatial_filter_rect(x, y, x + x_chunk_width,
                                                 y + y_chunk_width);

                for feature in in_layer.features() {


                    let feature_key = FeatureKey {
                            fid: feature.fid(),
                            layer_idx: idx as _
                        };

                    if seen.contains(&feature_key) {
                        continue;
                    }

                    seen.insert(feature_key);
                    //println!("Feature {}", feature.fid());

                    let mut shape = if args.no_holes {
                        let s = convert_from_gdal_to_geos_no_holes(&feature.geometry().as_geom(),
                                                  &context_handle, false,
                        )?;

                        if s.geometry_type() == MultiPolygon || s.geometry_type() == GeometryCollection {
                            s.union_unary(&context_handle)?
                        } else {
                            s
                        }
                    }
                    else {
                        convert_from_gdal_to_geos(&feature.geometry().as_geom(),
                                                  &context_handle, false,
                        )?
                    };


                    //shape.set_srid(orig_srid);
                    //let envelope = shape.envelope()?;

                    //let center = envelope.center()?;

                    //println!("{} {} {} {}  vs {:?}", x_min, y_min, x_max, y_max, center);

                    let bbox = shape.envelope()?.bbox()?;
                    let envelope_aabb = AABB::from_corners([bbox[0], bbox[1]], [bbox[2], bbox[3]]);

                    let mut objects_to_remove = Vec::new();

                    //Now check what intersects
                    for inter in union_rtree.locate_in_envelope_intersecting(&envelope_aabb) {
                        let inter_shape = &current_shapes.get(&inter.feature_key).unwrap().1;

                        let does_intersect = inter_shape.intersects(&shape)?;

                        if !does_intersect {
                            continue;
                        }

                        //println!("Intersection found!  {} with {}", current_index, inter.fid);

                        shape = shape.union(&context_handle, inter_shape)?;

                        if args.no_holes && shape.has_holes() {
                            shape = shape.remove_holes(&context_handle)?;
                            assert!(!shape.has_holes());

                            if shape.geometry_type() == MultiPolygon || shape.geometry_type() == GeometryCollection {
                                shape = shape.union_unary(&context_handle)?;
                            }
                        }

                        //println!("Shape type {:?}", shape.geometry_type());

                        objects_to_remove.push(inter.clone());
                    }

                    //now remove them
                    for o in objects_to_remove {
                        union_rtree.remove(&o);

                        current_shapes.remove(&o.feature_key);

                    }

                    //recalculate bbox
                    let bbox = shape.envelope()?.bbox()?;
                    let envelope_aabb = AABB::from_corners([bbox[0], bbox[1]], [bbox[2], bbox[3]]);

                    let rio = RTreeIndexObject {
                        feature_key,
                        envelope: envelope_aabb,
                    };
                    union_rtree.insert(rio.clone());
                    current_shapes.insert(feature_key, (rio,shape));

                    n_processed += 1;

                    if last_output.elapsed().as_secs() >= 3 {
                        last_output = Instant::now();
                        print_remaining_time(&now, n_processed, total as _);
                    }
                }
            }

            debug!("Writing {} shapes", current_shapes.len());

            //anything we write, we remove from current shapes
            current_shapes.retain(|_key, cs|  {

                //If extent is outside of current then retain it
                let upper = cs.0.envelope.upper();
                let lower = cs.0.envelope.upper();
                if upper[0] > x + x_chunk_width {
                    return true;
                }
                if upper[1] > y + y_chunk_width {
                    return true;
                }
                if lower[0] < x {
                    return true;
                }
                if lower[1] < y {
                    return true;
                }

                let mut out_ft = Feature::new(&out_def).unwrap();

                let gdal_geom = get_gdal_geom(&args, &cs.1, &context_handle);

                out_ft.set_geometry_directly(gdal_geom).unwrap();

                out_ft.set_field_integer_by_index(0, orig_fid).unwrap();
                orig_fid+=1;

                // Add the feature to the layer:
                out_ft.create(&out_lyr).unwrap();

                //Also remove from rtree
                union_rtree.remove(&cs.0);

                return false;
            });

            debug!("{} shapes carrying over", current_shapes.len());
        }
    }

    debug!("Writing final {} shapes", current_shapes.len());
    //Remaining shapes
    for (_out_id, cs) in current_shapes.iter() {

        let mut out_ft = Feature::new(&out_def)?;

        let gdal_geom = get_gdal_geom(&args, &cs.1, &context_handle);

        out_ft.set_geometry_directly(gdal_geom)?;

        out_ft.set_field_integer_by_index(0, orig_fid)?;
        orig_fid+=1;

        // Add the feature to the layer:
        out_ft.create(&out_lyr)?;
    }

    println!("num processed total: {}", n_processed);

    Ok(())
}

fn get_gdal_geom(
    args: &UnionArgs,
    shape: &SimpleGeometry,
    context_handle: &SimpleContextHandle) -> Geometry
{
    if shape.geometry_type() == GeometryTypes::Polygon {
        let mp_shape = shape.polygon_to_multipolygon(&context_handle).unwrap();

        if args.no_holes {
            //we removed holes above...
            assert!(!mp_shape.has_holes());
            convert_geos_to_gdal_no_holes(&mp_shape).unwrap()
        } else {
            convert_geos_to_gdal(&mp_shape).unwrap()
        }
    } else {
        if args.no_holes {
            //We should already have no holes
            assert!(!shape.has_holes());
            convert_geos_to_gdal_no_holes(shape).unwrap()
        } else {
            convert_geos_to_gdal(shape).unwrap()
        }

    }
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

fn main() {
    run().unwrap()
}

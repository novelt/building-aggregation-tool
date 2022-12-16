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
use std::collections::VecDeque;
use std::path::PathBuf;
//use std::process;

use anyhow::Result;
use bitvec::prelude::*;
//Counts points or centroids and outputs a raster
use structopt::StructOpt;

use gdal::raster::types::GdalType;
use gdal::vector::{Driver, Feature, Geometry, geometry_type_to_name, OGRwkbGeometryType};
use gdal::raster::{Driver as RasterDriver};
use gdal::raster::driver::{ MEM_DRIVER};

use geo_util::convert::{convert_from_gdal_to_geos, convert_geos_to_gdal};

use geo_util::raster::{ Raster, RasterStats};
//use geo_util::util::{print_remaining_time, RasterChunkIterator};
use geos::SimpleContextHandle;

#[derive(StructOpt)]
pub struct FillCenterBuildingGroupsArgs {
    //Note this isn't generic with respect to the fields, so for now keeping osm & dg
    #[structopt(long, short = "c", help = "OGR Connection string for inputs")]
    pub(crate) in_ogr_conn: String,

    #[structopt(long, short = "l", help = "Layer names for input, use - to use default, and all for everything")]
    pub(crate) in_ogr_layer: String,

    #[structopt(long)]
    pub(crate) snap_raster_path: PathBuf,
    //
    // #[structopt(long)]
    // pub(crate) feature_raster_base_path: PathBuf,

    #[structopt(long)]
    out_ogr_conn: String,

    #[structopt(long)]
    out_ogr_layer: String,
}

pub fn fill_center_building_groups(args: &FillCenterBuildingGroupsArgs) -> Result<()>
{
    //for each input feature, create an in memory raster; snapped to the snap raster, but only big enough

    //some of the grouped multipolygons are too large to pass FGB verification, so we turn verify buffers off
    let input_dataset = Driver::open_vector_static(&args.in_ogr_conn, true, &["VERIFY_BUFFERS=NO".to_string()]).unwrap();
    let input_layer = input_dataset.layer_by_name(&args.in_ogr_layer).unwrap();

    let snap_raster = Raster::read(&args.snap_raster_path, true);
    let snap_stats = &snap_raster.stats;

    let output_driver = Driver::get(Driver::DRIVER_NAME_FLATGEOBUF)?;
    let inmemory_driver = Driver::get(Driver::DRIVER_NAME_MEMORY)?;
    let raster_driver = RasterDriver::get(MEM_DRIVER).unwrap();
    let output_dataset = output_driver.create(&args.out_ogr_conn)?;

    let spatial_ref = input_layer.spatial_reference()?;

    let simple_context = SimpleContextHandle::new();

    let output_layer = output_dataset.create_layer_ext::<&str>(
        &args.out_ogr_layer,
        &spatial_ref,
        OGRwkbGeometryType::wkbMultiPolygon,
        &[
            //"OVERWRITE=YES",
            //"SPATIAL_INDEX=NO"
        ])?;
    let output_layer_def = output_layer.layer_definition();

    for in_feature in input_layer.features() {
        //we need a snapped raster that has 2 extra rows on top & bottom, and 2 extra columns

        let in_geom = in_feature.geometry().as_geom();
        let in_ext = in_geom.envelope();
        let x_left = snap_stats.calc_x(in_ext.MinX) - 1;
        let x_right = snap_stats.calc_x(in_ext.MaxX) + 1;
        let y_top = snap_stats.calc_y(in_ext.MaxY) - 1;
        let y_bottom = snap_stats.calc_y(in_ext.MinY) + 1;

        let mut window_stats = snap_stats.clone();
        window_stats.origin_x = snap_stats.calc_x_coord(x_left);
        window_stats.origin_y = snap_stats.calc_y_coord(y_top);
        window_stats.num_cols = ((x_right - x_left) + 1) as u32;
        window_stats.num_rows = ((y_bottom - y_top) + 1) as u32;
        window_stats.gdal_type = u8::gdal_type();
        window_stats.no_data_value = 0.;

        let mut bv_has_bldgs = BitVec::<Msb0, u8>::new();
        bv_has_bldgs.resize((window_stats.num_rows * window_stats.num_cols) as _, false);

        //which raster squares have buildings, fills in a bit vector representing the 2d grid defined by windows_stats
        set_buildings(&in_geom, &mut bv_has_bldgs, &window_stats);


        let mut bv_filled = BitVec::<Msb0, u8>::new();
        bv_filled.resize((window_stats.num_rows * window_stats.num_cols) as _, true);

        //flood fill the interior, using a bitvec; filled outside to false, which is why the default is true
        flood_fill(&bv_has_bldgs, &mut bv_filled, &window_stats);

        //which squares are on the inside of the polygon
        let mut bv_inner= BitVec::<Msb0, u8>::new();
        bv_inner.resize((window_stats.num_rows * window_stats.num_cols) as _, false);

        find_inner_squares(&mut bv_inner, &bv_filled, &window_stats);

        let mut data = vec![0u8; bv_inner.len()];
        let mut at_least_one = false;
        for (b_idx, b) in bv_inner.iter().enumerate() {
            if *b {
                data[b_idx] = 1;
                at_least_one = true;
            } else {
                data[b_idx] = 0;
            }
        }
        let raster_mem = raster_driver.create_in_memory(window_stats.num_cols as _, window_stats.num_rows as _

        ).unwrap();

        raster_mem.set_geo_transform(&[window_stats.origin_x,
        window_stats.pixel_width, 0.0, window_stats.origin_y, 0.0, window_stats.pixel_height])?;

        raster_mem.set_projection(&window_stats.projection)?;

        raster_mem.add_memory_band(&data);

        // let raster_mem_data:Vec<u8> = raster_mem.rasterband(1).unwrap().read_as((0,0), (window_stats.num_cols as i32, window_stats.num_rows as i32)).unwrap();
        //
        // for idx in 0..data.len() {
        //     assert_eq!(data[idx], raster_mem_data[idx]);
        // }



        // let raster = Raster::read(&raster_subpath, false);
        // raster.band().write((0,0), (window_stats.num_cols as i32, window_stats.num_rows as i32), &data);

        let mem_dataset = inmemory_driver.create("In memory")?;

        let mem_layer = mem_dataset.create_layer_ext::<&str>(
            "lyr",
            &spatial_ref,
            OGRwkbGeometryType::wkbPolygon,
        &[])?;

        raster_mem.rasterband(1).unwrap().polygonize(&mem_layer)?;

        let raster_polygons = mem_layer.count(true);

        assert!(!at_least_one || raster_polygons > 0);

        //any building who all points are surrounded by 9 filled raster squares, remove
        //assert!(in_geom.is_valid());
        let output_geom = prune_inner_buildings(&in_geom, &bv_inner, &window_stats)?;

        let mut ft = Feature::new(&output_layer_def)?;

        if at_least_one {
            let output_geom_geos = convert_from_gdal_to_geos(&output_geom, &simple_context, false)?;

            //assert!(output_geom_geos.is_valid());

            //create one big multipolygon of all the interior squares, this is used to union to the buildings multipolygon
            let mut holes_geom = Geometry::empty(OGRwkbGeometryType::wkbMultiPolygon)?;
            for f in mem_layer.features() {
                let poly = f.geometry().as_geom();
                assert_eq!(poly.geometry_type(), OGRwkbGeometryType::wkbPolygon);
                //To avoid the clone, maybe do a geos multipolygon, and convert the gdal polygons
                holes_geom.add_geometry(poly.clone())?;
            }

            let holes_geom_geos = convert_from_gdal_to_geos(&holes_geom, &simple_context, false)?;

            //assert!(holes_geom_geos.is_valid());
            let mp_union = output_geom_geos.union_unary(&simple_context).unwrap();
            let union = holes_geom_geos.union(&simple_context, &mp_union).unwrap();
            //
            let mut union_gdal = convert_geos_to_gdal(&union).unwrap();
            //
            let geom_type = union_gdal.geometry_type();

            if geom_type == OGRwkbGeometryType::wkbPolygon {
                let mp = union_gdal.to_multi_polygon();
                ft.set_geometry_directly(mp).unwrap();
            } else {
                ft.set_geometry_directly(union_gdal).unwrap();
            }

            if ft.create(&output_layer).is_err() {
                panic!("Problem with {:?}  geom type {:?}", in_feature.field("id"),
                       geometry_type_to_name(geom_type));
            }
        } else {

            ft.set_geometry(output_geom)?;
            ft.create(&output_layer).unwrap();

        }


    }



    //create an actual raster
    //create a multipolygon of the raster surrounded by 9 (or 8 technically), by rasterizing the in memory one
    //union this to the buildings to maintain mp valididy
    Ok(())
}

//Find squares that are completely surrounded by other filled squares
fn find_inner_squares(bv_inner: &mut BitVec::<Msb0, u8>, bv_filled: &BitVec::<Msb0, u8>, window_stats: &RasterStats) {
    for out_x in 1..window_stats.num_cols - 1 {
        for out_y in 1..window_stats.num_rows - 1 {
            let mut is_filled = true;

            for x in out_x - 1..=out_x+1 {
                if !is_filled {
                    break;
                }
                for y in out_y - 1..=out_y+1 {
                    let bv_idx = (x + y * window_stats.num_cols ) as usize;

                    if !bv_filled[bv_idx] {
                        is_filled = false;
                        break;
                    }
                }
            }

            let out_idx = (out_x + out_y * window_stats.num_cols ) as usize;
            bv_inner.set(out_idx, is_filled)
        }
    }
}

fn prune_inner_buildings(input_geom: &Geometry, bv_inner: &BitVec::<Msb0, u8>, window_stats: &RasterStats) -> Result<Geometry> {
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
            let x_coord = window_stats.calc_x(*x);
            let y_coord = window_stats.calc_y(*y);

            let bv_idx = (x_coord + y_coord * window_stats.num_cols as i32) as usize;
            if !bv_inner[bv_idx] {
                skip = false;
                break;
            }

        }

        if skip {
            continue;
        }

        output_geom.add_geometry(poly.clone()).unwrap();
    }

    Ok(output_geom)

}

fn set_buildings(in_geom: &Geometry, bv_has_bldgs: &mut BitVec::<Msb0, u8>, window_stats: &RasterStats) {
    assert_eq!(in_geom.geometry_type(), OGRwkbGeometryType::wkbMultiPolygon);

    let num_polys = in_geom.geometry_count();

    for p_idx in 0..num_polys {
        let poly = in_geom.get_geometry(p_idx);

        //First ring is the 1st geometry
        let outer_ring = poly.get_geometry(0);

        let pt_count = outer_ring.point_count();

        for pt_idx in 0..pt_count {
            let point = outer_ring.get_point(pt_idx as _);


            let x = window_stats.calc_x(point[0]);
            assert!(x >= 0 && x < window_stats.num_cols as _);
            let y = window_stats.calc_y(point[1]);
            assert!(y >= 0 && y < window_stats.num_rows as _);

            let bv_idx = (x + y * window_stats.num_cols as i32) as usize;
            bv_has_bldgs.set(bv_idx, true);
        }
    }
}

//Fills bv_output with false; meaning true is inside the rasterized group building
fn flood_fill(bv_has_bldgs: &BitVec::<Msb0, u8>, bv_output: &mut BitVec::<Msb0, u8>, window_stats: &RasterStats) {
    // let mut bv_output = BitVec::<Msb0, u8>::new();
    // bv_output.resize( (stats.num_rows * stats.num_cols) as usize, true);

    assert_eq!(bv_has_bldgs.len(), (window_stats.num_rows * window_stats.num_cols) as usize);

    let mut bv_seen = BitVec::<Msb0, u8>::new();
    bv_seen.resize(bv_has_bldgs.len(), false);
    //Flood fill this guy, we allow the coordinates to be just outside too, anything we can reach on the outside
    //is false

    let mut deq = VecDeque::new();

    let num_cols = window_stats.num_cols as isize;
    let num_rows = window_stats.num_rows as isize;

    //seed with top row
    for x in 0..num_cols {
        deq.push_back(x);
    }

    while !deq.is_empty() {
        let current_idx = deq.pop_front().unwrap();
        if bv_seen[current_idx as usize] {
            continue;
        }

        bv_seen.set(current_idx as usize, true);

        //If we have buildings, we can't move
        if bv_has_bldgs[current_idx as usize] {
            continue;
        }

        bv_output.set(current_idx as usize, false);

        let y = current_idx / num_cols;
        let x = current_idx % num_cols;

        for dx in -1..=1 {
            let try_x = x + dx;
            if try_x < 0 || try_x >= num_cols {
                continue;
            }
            for dy in -1..=1 {
                let try_y = y + dy;
                if try_y < 0 || try_y >= num_rows {
                    continue;
                }

                let try_index = try_x + try_y * num_cols;
                if bv_seen[try_index as usize] {
                    continue;
                }

                deq.push_back(try_index);
            }
        }
    }
}
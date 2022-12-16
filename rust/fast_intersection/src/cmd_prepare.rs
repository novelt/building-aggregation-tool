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
use std::fmt;
use std::path::PathBuf;
use std::time::Instant;

use anyhow::Result;
use bitvec::prelude::*;
use geo_booleanop::boolean::{BooleanOp};
use geo_types::{Coordinate, LineString, MultiPolygon, Polygon as RustPolygon};
use itertools::Itertools;
use log::{debug,trace};
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use structopt::StructOpt;

use gdal::raster::Driver as RasterDriver;
use gdal::raster::driver::MEM_DRIVER;
use gdal::raster::global_func::rasterize_dataset;
use gdal::raster::types::GdalType;
use gdal::spatial_ref::{ SpatialRef};
use gdal::vector::{Dataset, Driver, Feature, FieldDefinition, Geometry, Layer, LayerDefinition, OGRFieldSubType, OGRFieldType, OGRwkbGeometryType};
use geo_util::raster::{Raster, RasterStats};
use geo_util::util::print_remaining_time;

//extern crate geo;

type DefBitVec = BitVec::<u8, Msb0>;

/*
This slices the shapes into a grid form

Note there can be issues with interior polygons
 */

#[derive(StructOpt)]
pub struct PrepareArgs {
    #[structopt(long, help = "OGR Connection string for inputs")]
    pub(crate) in_ogr_conn: String,

    #[structopt(long, help = "Layer name")]
    pub(crate) in_ogr_layer: String,

    // #[structopt(long, help = "OGR Connection string for inputs")]
    // pub(crate) dbg_out_ogr_conn: String,

    #[structopt(parse(from_os_str), long, help="Reference raster -- should be larger than the input layer")]
    ref_raster: PathBuf,

    #[structopt(parse(from_os_str), long)]
    output_path: PathBuf,

    #[structopt(long, help = "If defined, will use this field instead of the GDAL fid")]
    id_field: Option<String>,
}


/// Takes the input layer and slices it according to the reference raster
///
pub fn prepare(args: &PrepareArgs) -> Result<()> {
    println!("Starting...");

    let now = Instant::now();
    let mut last_output = Instant::now();

    let in_dataset = Dataset::open(&args.in_ogr_conn)?;
    let in_layer = in_dataset.layer_by_name(&args.in_ogr_layer)?;

    //These are used to test the rasterization
    let spatial_ref = in_layer.spatial_reference()?;

    //We use the snap raster just for the stats
    let snap_raster = Raster::read(&args.ref_raster, true);
    let snap_stats = &snap_raster.stats;

    let num_features = in_layer.count(false);

    //let out_dataset = Dataset::open_rw(&args.dbg_out_ogr_conn, false)?;

    let output_driver = Driver::get(Driver::DRIVER_NAME_FLATGEOBUF)?;
    let output_dataset = output_driver.create(&args.output_path.to_str().unwrap())?;

    let mut out_poly_layer = output_dataset.create_layer_ext::<String>(
        args.output_path.file_stem().unwrap().to_str().unwrap(),
        &spatial_ref,
        OGRwkbGeometryType::wkbMultiPolygon,
        &[
            "SPATIAL_INDEX=NO".to_string()
        ],
    )?;

    let field_defn = FieldDefinition::new("grid_index", OGRFieldType::OFTInteger)?;
    field_defn.add_to_layer(&mut out_poly_layer)?;

    let field_defn = FieldDefinition::new("orig_fid", OGRFieldType::OFTInteger)?;
    field_defn.add_to_layer(&mut out_poly_layer)?;

    let out_poly_layer_def = out_poly_layer.layer_definition();

    for (f_idx, feature) in in_layer.features().enumerate() {
        let fid = if let Some(f) = args.id_field.as_ref() {
            feature.field(f).unwrap().into_int().unwrap() as i64
        } else {
            feature.fid()
        };

        let geometry = feature.geometry().as_geom();

        let window_stats = get_window_stats(&geometry, &snap_stats);

        // let gdal_rasterized = rasterize_single_feature(&geometry,
        //                                                &spatial_ref,
        //                                                &window_stats)?;

        //println!("Data: {:?}", data);

        //create a debug dataset

        debug!("Rasterizing polygon {} ", fid);

        let rust_rasterized = rasterize_polygon(&window_stats, &geometry)?;

        debug!("Rasterized polygon {} size {}", fid, rust_rasterized.len());

        // for i in 0..gdal_rasterized.len() {
        //     if gdal_rasterized[i] == rust_rasterized[i] {
        // write_to_debug_layers(&args.dbg_out_ogr_conn,
        //                       &spatial_ref,
        //                       &window_stats,
        //                       &gdal_rasterized, &rust_rasterized);

        grid_slice_multi_polygon(&window_stats, &geometry, &rust_rasterized,
                                 &out_poly_layer_def, &out_poly_layer, fid,
        )?;

        // println!("Problem!!");
        // return Ok(());


        if last_output.elapsed().as_secs() >= 3 {
            last_output = Instant::now();
            print_remaining_time(&now,
                                 f_idx as _,
                                 num_features as _);
        }
    }
    Ok(())
}

#[derive(Debug)]
struct Edge {
    //y_min: f64,     // smallest value of y (when edge enters)
    //y_max: f64,     // largest value of y (when edge leaves)
    raster_x_hit: f64,
    // intersection point (init with x value at yMax)
    m_inv: f64,
    // dx/dy (inverse line increment)
    raster_y_min: u32,
    //min raster row is (-0.5, 0.5)
    raster_y_max: u32,
    //label: String,
}

#[derive(Debug)]
struct Edge2 {
    //In raster coordinates
    pt_from: RasterCoord,
    pt_to: RasterCoord,

    //slope
    delta: RasterCoord,
}


impl Edge2 {
    fn new(pt_from: RasterCoord, pt_to: RasterCoord) -> Self {
        Self {
            pt_to,
            pt_from,
            delta: RasterCoord {
                row: pt_to.row - pt_from.row,
                col: pt_to.col - pt_from.col,
            },
        }
    }

    fn col_at_row(&self, new_row: f64) -> f64 {
        /*
        new_row = from.row + delta.row * k
        new_col = from.col + delta.col * k
         */
        let k = (new_row - self.pt_from.row) / self.delta.row;
        // println!("from row {:.3} col {:.3} to row {:.3} col {:.3} new row {:.3} k={:.6}",
        //          self.pt_from.row,
        //     self.pt_from.col,
        //     self.pt_to.row,
        //     self.pt_to.col,
        //     new_row,
        //          k);
        assert!(k >= 0.0);
        assert!(k <= 1.0);

        self.pt_from.col + self.delta.col * k
    }

    fn row_at_col(&self, new_col: f64) -> f64 {
        /*
        new_row = from.row + delta.row * k
        new_col = from.col + delta.col * k
         */
        let k = (new_col - self.pt_from.col) / self.delta.col;


        trace!("from row {:.3} col {:.3} to row {:.3} col {:.3} new col {:.3} k={:.6}",
                 self.pt_from.row,
            self.pt_from.col,
            self.pt_to.row,
            self.pt_to.col,
            new_col,
                 k);

        assert!(k >= 0.0);
        assert!(k <= 1.0);

        self.pt_from.row + self.delta.row * k
    }

    fn crosses_grid_line(&self) -> bool {
        crosses_int_bounary(self.pt_from.row,
                            self.pt_to.row) || crosses_int_bounary(
            self.pt_from.col, self.pt_to.col,
        )
    }

    //assumes not crosses grid lines
    fn get_grid_row(&self) -> i32 {
        let row = self.pt_from.row.floor();

        if row == self.pt_from.row && self.pt_from.row > self.pt_to.row {
            return (row - 1.0) as _;
        }

        row as _
    }

    fn get_grid_col(&self) -> i32 {
        let col = self.pt_from.col.floor();

        if col == self.pt_from.col && self.pt_from.col > self.pt_to.col {
            return (col - 1.0) as _;
        }

        col as _
    }
}

fn crosses_int_bounary(n1: f64, n2: f64) -> bool {

    if n1==n2 {
        return false;
    }

    if n1 > n2 {
        return crosses_int_bounary(n2, n1);
    }

    assert!(n1 <= n2);

    let n1_f = n1.floor();
    let mut n2_f = n2.floor();

    if n2_f == n2 {
        n2_f -= 1.0;
    }

    return n1_f != n2_f;
}


//x, y ; column, row
#[derive(Debug, Copy, Clone, PartialEq)]
struct RasterCoord {
    row: f64,
    col: f64,
}

#[derive(PartialEq, FromPrimitive, Debug, Copy, Clone)]
enum Side {
    Top,
    Right,
    Bottom,
    Left,
}

fn increment_side(side: Side) -> Side {
    let mut idx = side as i32;
    idx += 1;
    if idx > 3 {
        idx = 0;
    }
    Side::from_i32(idx).unwrap()
}

impl RasterCoord {
    fn on_grid(&self) -> bool {
        self.row.floor() == self.row ||
            self.col.floor() == self.col
    }


    fn to_coordinate(&self, window_stats: &RasterStats) -> (f64, f64) {
        // assert!(window_stats.pixel_width > 0.0);
        // assert!(window_stats.pixel_height < 0.0);
        (
            window_stats.origin_x + window_stats.pixel_width * self.col,
            window_stats.origin_y + window_stats.pixel_height * self.row
        )
    }

    fn get_side(&self, row: i32, col: i32) -> Side {
        if self.row.floor() == self.row {
            if self.row.floor() == row as f64 {
                return Side::Top;
            } else {
                return Side::Bottom;
            }
        } else if self.col.floor() == self.col {
            if self.col.floor() == col as f64 {
                return Side::Left;
            } else {
                return Side::Right;
            }
        } else {
            panic!("oh no {} {} vs {} {}", row, col, self.row, self.col);
        }
    }

    fn is_clockwise(&self, side: Side, other_point: &RasterCoord) -> bool {
        match side {
            Side::Top => other_point.col > self.col,
            Side::Right => other_point.row > self.row,
            Side::Bottom => self.col > other_point.col,
            Side::Left => self.row > other_point.row,
        }
    }
}

impl Edge {
    fn new(top_coord: &RasterCoord, bot_coord: &RasterCoord, //, label: String
    ) -> Self {
        //because 1st row is above the last row
        assert!(top_coord.row < bot_coord.row);

        let dy = top_coord.row - bot_coord.row;
        let dx = top_coord.col - bot_coord.col;

        assert!(top_coord.row >= 0.);
        assert!(bot_coord.row >= 0.);

        //we want y_min to be rounded down and y_max to be rounded up
        //this is because we want the real segment to actually intersect the middle of these raster squares
        //note the 0.5 pixel height is because we want the center
        let raster_y_min = (top_coord.row + 0.5).floor() as u32;
        let raster_y_max = (bot_coord.row + 0.5).floor() as u32;

        // (x0 - x1) / (y0 - y1) = dx / dy
        // x0 - x1 = (dx / dy) * (y0-y1)
        // x0 =  (dx / dy) * (y0-y1) + x1
        let raster_x_hit = top_coord.col + ((raster_y_min as f64 + 0.5) - top_coord.row) * dx / dy;

        Self {
            //x at raster_y_min+0.5
            raster_x_hit,
            m_inv: dx / dy,
            raster_y_min,
            raster_y_max,
            // label: format!("{} raster_x_hit: {} y min/max {}, {}.  What {} + {} - {} * {}/{}",
            //                label, raster_x_hit, raster_y_min, raster_y_max,
            //                top_coord.col, top_coord.row, raster_y_min, dx, dy),
        }
    }
}

// To use the `{}` marker, the trait `fmt::Display` must be implemented
// manually for the type.
impl fmt::Display for Edge2 {
    // This trait requires `fmt` with this exact signature.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Write strictly the first element into the supplied output
        // stream: `f`. Returns `fmt::Result` which indicates whether the
        // operation succeeded or failed. Note that `write!` uses syntax which
        // is very similar to `println!`.
        //write!(f, "{}", self.0)
        write!(f, "from {} to {}",
               self.pt_from,
               self.pt_to,
        )
    }
}

impl fmt::Display for RasterCoord {
    // This trait requires `fmt` with this exact signature.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Write strictly the first element into the supplied output
        // stream: `f`. Returns `fmt::Result` which indicates whether the
        // operation succeeded or failed. Note that `write!` uses syntax which
        // is very similar to `println!`.
        //write!(f, "{}", self.0)
        write!(f, "row {:.3} col {:.3}",
               self.row,
               self.col,
        )
    }
}

fn rasterize_polygon(window_stats: &RasterStats, geometry: &Geometry) -> Result<DefBitVec> {

    //The y projected coordinate of the center of the 1st row
    // let y_start = window_stats.origin_y + 0.5 * window_stats.pixel_height;
    // let y_stop = window_stats.bottom_y_coord() - 0.5 * window_stats.pixel_height;
    //
    // assert!(y_start > y_stop);

    let mut bv_rasterized = DefBitVec::new();
    bv_rasterized.resize((window_stats.num_rows * window_stats.num_cols) as _, false);

    assert_eq!(geometry.geometry_type(), OGRwkbGeometryType::wkbMultiPolygon);

    //Read in all the edges

    let poly_count = geometry.geometry_count();

    let mut edge_list = Vec::new();

    for p in 0..poly_count {
        let poly = geometry.get_geometry(p);
        assert_eq!(poly.geometry_type(), OGRwkbGeometryType::wkbPolygon);

        let ring_count = poly.geometry_count();

        for r in 0..ring_count {
            let ring = poly.get_geometry(r);

            let pt_count = ring.point_count();

            assert!(pt_count > 3);
            assert_eq!(ring.get_point(0), ring.get_point((pt_count - 1) as _));

            let mut raster_coords = Vec::with_capacity(pt_count);

            for pt_idx in 0..pt_count {
                let p1 = ring.get_point(pt_idx as _);

                let r1 = RasterCoord {
                    row: ((p1[1] - window_stats.origin_y) / window_stats.pixel_height),
                    col: ((p1[0] - window_stats.origin_x) / window_stats.pixel_width),
                };

                raster_coords.push(r1);
            }

            for pt_idx in 0..pt_count - 1 {
                let r1 = &raster_coords[pt_idx];
                let r2 = &raster_coords[pt_idx + 1];

                let (top_coord, bot_coord) = if r1.row < r2.row {
                    (&r1, &r2)
                } else {
                    (&r2, &r1)
                };

                //skip horizontal edges
                if r1.row == r2.row {
                    continue;
                }

                let edge =
                    Edge::new(top_coord, bot_coord); //, format!(
                //     "raster (r{}, c{}) to (r{}, c{}) coords {}, {} to {}, {}",
                //     top_coord.row,
                //     top_coord.col,
                //     bot_coord.row,
                //     bot_coord.col,
                //     p1.0, p1.1, p2.0, p2.1
                // ));

                if edge.raster_y_min < edge.raster_y_max {
                    edge_list.push(edge);
                } else {
                    //println!("Ignoring edge {:?}", &edge);
                }
            }
        }
    }

    //println!("Have {} edges", edge_list.len());

    //lowest raster y min last
    edge_list.sort_by(|e1, e2| e2.raster_y_min.cmp(&e1.raster_y_min));

    let mut current_row = 0;

    let mut active_edges = Vec::with_capacity(edge_list.len());

    while (!edge_list.is_empty() || !active_edges.is_empty()) && current_row < window_stats.num_rows {
        //println!("Starting row {}", current_row);

        //consider edges whose y_max >= current_y and y_min <= current_y
        //anything earlier in the edge_list has a y_min that is too high
        //and we stop looking when the y_min

        //Move those edges from the ET to the AET for which holds:
        while !edge_list.is_empty() {
            let last_elem = edge_list.last().unwrap();
            if last_elem.raster_y_min == current_row {
                active_edges.push(edge_list.pop().unwrap());
                continue;
            }

            if last_elem.raster_y_min > current_row {
                break;
            }

            //edge case, first edge is above
            if last_elem.raster_y_min < current_row {
                edge_list.pop().unwrap();
            }
        }

        let mut x_hit_list = active_edges.iter().map(|e| e.raster_x_hit).collect_vec();
        x_hit_list.sort_by(|a, b| a.partial_cmp(b).unwrap());

        // for a in active_edges.iter() {
        //     println!("{}", &a.label);
        // }

        //println!("X intersections: {}", x_hit_list.iter().map(|x| format!("{}", x)).join(", "));

        let mut x_hit_idx = 0;
        let mut parity = 0;

        for col in 0..window_stats.num_cols {
            while x_hit_idx < x_hit_list.len() && x_hit_list[x_hit_idx] < 0.5 + col as f64 {
                x_hit_idx += 1;
                parity = 1 - parity;
            }
            bv_rasterized.set((current_row * window_stats.num_cols + col) as usize, parity == 1);
        }


        //Remove anything in active_edges that no longer applies
        for i in (0..active_edges.len()).rev() {
            if active_edges[i].raster_y_max == 1 + current_row {
                active_edges.swap_remove(i);
            }
        }

        //Increment x intersection
        for a in active_edges.iter_mut() {
            a.raster_x_hit += a.m_inv;
        }

        current_row += 1;
    }

    Ok(bv_rasterized)
}

fn get_window_stats(geometry: &Geometry, snap_stats: &RasterStats) -> RasterStats
{
    let in_ext = geometry.envelope();

    let x_left = snap_stats.calc_x(in_ext.MinX);
    let x_right = snap_stats.calc_x(in_ext.MaxX);
    let y_top = snap_stats.calc_y(in_ext.MaxY);
    let y_bottom = snap_stats.calc_y(in_ext.MinY);

    let mut window_stats = snap_stats.clone();
    window_stats.origin_x = snap_stats.calc_x_coord(x_left);
    window_stats.origin_y = snap_stats.calc_y_coord(y_top);
    window_stats.num_cols = ((x_right - x_left) + 1) as u32;
    window_stats.num_rows = ((y_bottom - y_top) + 1) as u32;
    window_stats.gdal_type = u8::gdal_type();
    window_stats.no_data_value = 0.;

    window_stats
}

//uses gdal and in memory raster to rasterize a single feature
#[allow(dead_code)]
fn rasterize_single_feature(
    geometry: &Geometry,
    spatial_ref: &SpatialRef,
    window_stats: &RasterStats,
) -> Result<DefBitVec> {
    let raster_driver = RasterDriver::get(MEM_DRIVER).unwrap();
    let inmemory_driver = Driver::get(Driver::DRIVER_NAME_MEMORY)?;


    let raster_mem = raster_driver.create_in_memory(window_stats.num_cols as _, window_stats.num_rows as _,
    ).unwrap();

    raster_mem.set_geo_transform(&[window_stats.origin_x,
        window_stats.pixel_width, 0.0, window_stats.origin_y, 0.0, window_stats.pixel_height])?;

    raster_mem.set_projection(&window_stats.projection)?;

    let data = vec![0u8; (window_stats.num_rows * window_stats.num_cols) as usize];

    raster_mem.add_memory_band(&data);

    let mem_dataset = inmemory_driver.create("In memory")?;

    let mem_layer = mem_dataset.create_layer_ext::<&str>(
        "lyr",
        &spatial_ref,
        OGRwkbGeometryType::wkbMultiPolygon,
        &[])?;

    let mem_layer_def = mem_layer.layer_definition();

    let mut mem_feature = Feature::new(&mem_layer_def)?;
    mem_feature.set_geometry_directly(geometry.clone())?;
    mem_feature.create(&mem_layer)?;

    rasterize_dataset::<_>(&mem_dataset, &raster_mem, &["-burn", "1"]).unwrap();

    let mut bv_rasterized = DefBitVec::new();
    bv_rasterized.resize((window_stats.num_rows * window_stats.num_cols) as _, false);

    for (idx, val) in data.iter().enumerate() {
        if *val == 1 {
            bv_rasterized.set(idx, true);
        }
    }

    Ok(bv_rasterized)
}

#[allow(dead_code)]
fn write_to_debug_layers(dbg_out_ogr_conn: &str,
                         spatial_ref: &SpatialRef,
                         window_stats: &RasterStats,
                         gdal_rasterized: &DefBitVec,
                         rust_rasterized: &DefBitVec,
) -> Result<()>
{
    let out_dataset = Dataset::open_rw(dbg_out_ogr_conn, false)?;

    let mut dbg_poly_layer = out_dataset.create_layer_ext(
        "dbg_poly",
        &spatial_ref,
        OGRwkbGeometryType::wkbPolygon,
        &[
            "OVERWRITE=YES",
        ],
    )?;
    let dbg_point_layer = out_dataset.create_layer_ext(
        "dbg_point",
        &spatial_ref,
        OGRwkbGeometryType::wkbPoint,
        &[
            "OVERWRITE=YES",
        ],
    )?;


    let field_defn = FieldDefinition::new("is_on", OGRFieldType::OFTInteger)?;
    field_defn.set_sub_type(OGRFieldSubType::OFSTBoolean);
    field_defn.add_to_layer(&mut dbg_poly_layer)?;

    let field_defn = FieldDefinition::new("is_on_new", OGRFieldType::OFTInteger)?;
    field_defn.set_sub_type(OGRFieldSubType::OFSTBoolean);
    field_defn.add_to_layer(&mut dbg_poly_layer)?;

    let dbg_point_layer_def = dbg_point_layer.layer_definition();
    let dbg_poly_layer_def = dbg_poly_layer.layer_definition();

    for r in 0..window_stats.num_rows {
        let mut row_str = "".to_string();

        for c in 0..window_stats.num_cols {
            let idx = (r * window_stats.num_cols + c) as usize;
            row_str.push_str(if !gdal_rasterized[idx] { "_" } else { "*" });

            let g = Geometry::bbox(
                window_stats.calc_x_coord(c),
                window_stats.calc_y_coord(1 + r),
                window_stats.calc_x_coord(c + 1),
                window_stats.calc_y_coord(r ),
            )?;
            let mut f = Feature::new(&dbg_poly_layer_def)?;
            f.set_geometry_directly(g)?;
            f.set_field_integer_by_index(0, gdal_rasterized[idx] as _)?;
            f.set_field_integer_by_index(1, rust_rasterized[idx] as _)?;

            f.create(&dbg_poly_layer)?;

            let mut f = Feature::new(&dbg_point_layer_def)?;
            let g = Geometry::from_x_y(
                window_stats.calc_x_coord(c) + window_stats.pixel_width * 0.5,
                window_stats.calc_y_coord(r) + window_stats.pixel_height * 0.5,
            )?;
            f.set_geometry_directly(g)?;
            f.create(&dbg_point_layer)?;
        }

        println!("{}", row_str);
    }


    println!(" VS ");

    for r in 0..window_stats.num_rows {
        let mut row_str = "".to_string();

        for c in 0..window_stats.num_cols {
            let idx = (r * window_stats.num_cols + c) as usize;
            row_str.push_str(if rust_rasterized[idx] { "*" } else { "_" });
        }

        println!("{}", row_str);
    }

    Ok(())
}

#[allow(dead_code)]
fn write_edges_to_debug_layers(dbg_out_ogr_conn: &str,
                               spatial_ref: &SpatialRef,
                               window_stats: &RasterStats,
                               edges: &[Edge2],
) -> Result<()>
{
    let out_dataset = Dataset::open_rw(dbg_out_ogr_conn, false)?;

    let mut dbg_poly_layer = out_dataset.create_layer_ext(
        "dbg_lines",
        &spatial_ref,
        OGRwkbGeometryType::wkbLineString,
        &[
            "OVERWRITE=YES",
        ],
    )?;

    let field_defn = FieldDefinition::new("idx", OGRFieldType::OFTInteger)?;
    field_defn.add_to_layer(&mut dbg_poly_layer)?;

    let dbg_poly_layer_def = dbg_poly_layer.layer_definition();

    for (e_idx, edge) in edges.iter().enumerate() {
        let mut g = Geometry::empty(OGRwkbGeometryType::wkbLineString)?;

        g.set_point_2d(0,
                       edge.pt_from.to_coordinate(window_stats),
        );
        g.set_point_2d(1, edge.pt_to.to_coordinate(window_stats));

        let mut f = Feature::new(&dbg_poly_layer_def)?;
        f.set_geometry_directly(g)?;
        f.set_field_integer_by_index(0, e_idx as _)?;

        f.create(&dbg_poly_layer)?;
    }

    Ok(())
}


fn write_slice_polys(layer_def: &LayerDefinition, layer: &Layer,
                     window_stats: &RasterStats,
                     square_mps: &[Vec<PolyRasterCoord>],
                     fid: i64,
) -> Result<()>
{
    for (grid_index, multi_polygon) in square_mps.iter().enumerate() {
        if multi_polygon.is_empty() {
            continue;
        }

        let mut gdal_multi_geom = Geometry::empty(OGRwkbGeometryType::wkbMultiPolygon)?;

        for polygon in multi_polygon.iter() {
            let mut outer_ring = Geometry::empty(OGRwkbGeometryType::wkbLinearRing)?;

            for (pt_idx, point) in polygon.exterior.iter().enumerate() {
                outer_ring.set_point_2d(pt_idx,
                                        point.to_coordinate(window_stats),
                );
            }

            let mut gdal_geom = Geometry::empty(OGRwkbGeometryType::wkbPolygon)?;
            gdal_geom.add_geometry(outer_ring)?;

            for ir in polygon.holes.iter() {
                let mut inner_ring = Geometry::empty(OGRwkbGeometryType::wkbLinearRing)?;

                for (pt_idx, point) in ir.iter().enumerate() {
                    inner_ring.set_point_2d(pt_idx,
                                            point.to_coordinate(window_stats),
                    );
                }

                gdal_geom.add_geometry(inner_ring)?;
            }

            gdal_multi_geom.add_geometry(gdal_geom)?;
        }

        let mut f = Feature::new(layer_def)?;

        //let center = gdal_multi_geom.centroid().unwrap();
        //let (center_x, center_y, _) = center.get_point(0);

        // let grid_row = grid_index / window_stats.num_cols as usize;
        // let grid_col = grid_index % window_stats.num_cols as usize;

        // let check_col = window_stats.calc_x(center_x);
        // let check_row = window_stats.calc_y(center_y);

        // if check_row != grid_row as i32 {
        //     for (grid_index, multi_polygon) in square_mps.iter().enumerate() {
        //         println!("Grid Index: {}", grid_index);
        //         println!("Center x {} center y {}", check_col, check_row);
        //         for (p_idx, p) in multi_polygon.iter().enumerate() {
        //             for c in p.iter() {
        //                 println!("Polygon {} Coordinate {}", p_idx, c);
        //             }
        //         }
        //     }
        // }
        //
        // assert_eq!(grid_row as i32, check_row);
        // assert_eq!(grid_col as i32, check_col);

        f.set_geometry_directly(gdal_multi_geom)?;
        f.set_field_integer_by_index(0, grid_index as _)?;
        f.set_field_integer_by_index(1, fid as _)?;

        f.create(layer)?;
    }

    Ok(())
}

//returns all polygons in 1st list and all holes in 2nd
fn read_edges(window_stats: &RasterStats, geometry: &Geometry) ->
(Vec<Vec<RasterCoord>>,
 Vec<Vec<RasterCoord>>)
{
    assert_eq!(geometry.geometry_type(), OGRwkbGeometryType::wkbMultiPolygon);

    let poly_count = geometry.geometry_count();

    let mut poly_edge_list = Vec::new();
    let mut hole_edge_list = Vec::new();

    for p in 0..poly_count {
        let poly = geometry.get_geometry(p);
        assert_eq!(poly.geometry_type(), OGRwkbGeometryType::wkbPolygon);
        let ring_count = poly.geometry_count();

        for r in 0..ring_count {
            let ring = poly.get_geometry(r);

            let pt_count = ring.point_count();

            assert!(pt_count > 3);
            assert_eq!(ring.get_point(0), ring.get_point((pt_count - 1) as _));

            let mut raster_coords = Vec::with_capacity(pt_count);

            for pt_idx in 0..pt_count {
                let p1 = ring.get_point(pt_idx as _);

                let r1 = RasterCoord {
                    row: ((p1[1] - window_stats.origin_y) / window_stats.pixel_height),
                    col: ((p1[0] - window_stats.origin_x) / window_stats.pixel_width),
                };

                raster_coords.push(r1);
            }

            //close it
            raster_coords.push(raster_coords[0].clone());


            if r == 0 {
                poly_edge_list.push(raster_coords);
            } else {
                //we want holes to be clockwise, not counter clockwise
                raster_coords.reverse();
                hole_edge_list.push(raster_coords);
            }
        }
    }

    (poly_edge_list, hole_edge_list)
}

fn grid_slice_single_polygon(window_stats: &RasterStats,
                             //edges to a single polygon
                             edge_list: Vec<RasterCoord>,
) -> Result<Vec<Vec<Vec<RasterCoord>>>>
{
    //Store the polygons in a grid
    //let mut grid_polygons = vec![None; (window_stats.num_cols * window_stats.num_rows) as usize];

    //need to store the edges in each cell, we also need to store multipolygons
    //need to distinguish polygons that are closed, so each grid can have many
    let mut grid_line_segs: Vec<Vec<Vec<RasterCoord>>> = Vec::with_capacity((window_stats.num_cols * window_stats.num_rows) as usize);
    for _ in 0..window_stats.num_cols * window_stats.num_rows {
        //let v: Vec<RasterCoord> = Vec::new();
        grid_line_segs.push(vec![]);
    }

    let new_edge_list = create_nocross_edges(edge_list);

    //println!("WHTHSNTHEUSNHE");

    // write_edges_to_debug_layers(
    //     dbg_out_ogr_conn, spatial_ref, window_stats,
    //     &new_edge_list.as_slices().0)?;

    //We need to handle an edge case where the polygon never crosses the grid.  In this case we have nothing to do
    if !handle_polygon_not_touching_grid_square(&new_edge_list, &mut grid_line_segs, window_stats.num_cols)? {
        return Ok(grid_line_segs);
    }

    let grid_edge_sequences = build_edge_sequences(new_edge_list, grid_line_segs.len(), window_stats.num_cols)?;


    // now we make polygons !
    // Start with the beginning of an edge, follow it around to make a simple ploygon

    let mut loop_check = 0;

    //println!("Starting edge sequences");
    for (grid_index, mut edge_sequences) in grid_edge_sequences.into_iter().enumerate() {

        //println!("Grid {} edge sequences {}", grid_index, edge_sequences.len());

        //since we are not crossing edges, this is fine
        let grid_row = (grid_index / window_stats.num_cols as usize) as i32;
        let grid_col = (grid_index % window_stats.num_cols as usize) as i32;

        while !edge_sequences.is_empty() {

            //start with any one
            let mut cur_polygon = edge_sequences.pop().unwrap();

            loop {
                loop_check += 1;
                if loop_check > 100_000_000 {
                    break;
                }

                //what is current 'grid edge distance' to close the polygon?
                //this is the clockwise distance, around the edge of the square

                let current_close_distance = grid_boundary_clockwise_distance(&cur_polygon.last().unwrap(),
                                                                              &cur_polygon[0], grid_row, grid_col);

                let mut cur_min_distance = current_close_distance;
                let mut cur_closest_idx = edge_sequences.len();

                //now we need to find if any other edges are between, so with less polar/grid_edge distance
                for idx in 0..edge_sequences.len() {
                    let test_distance = grid_boundary_clockwise_distance(&cur_polygon.last().unwrap(),
                                                                         &edge_sequences[idx][0], grid_row, grid_col);

                    if test_distance < cur_min_distance {
                        cur_min_distance = test_distance;
                        cur_closest_idx = idx;
                    }
                }

                //We close the current polygon
                let last_point = cur_polygon.last().unwrap().clone();

                if cur_closest_idx == edge_sequences.len() {
                    let first_point = cur_polygon[0].clone();

                    add_corners(
                        &last_point,
                        &first_point,
                        grid_row, grid_col, &mut cur_polygon);

                    //now close the polygon
                    cur_polygon.push(cur_polygon[0].clone());

                    grid_line_segs[grid_index].push(cur_polygon);

                    break;
                } else {

                    //we found another point, we need to add the corners beteew the current end and the new start of the edge sequence
                    add_corners(
                        &last_point,
                        &edge_sequences[cur_closest_idx][0],
                        grid_row, grid_col, &mut cur_polygon);

                    //now add all the points
                    cur_polygon.extend(
                        edge_sequences.swap_remove(cur_closest_idx)
                    );
                }
            }
        }
    }

    Ok(grid_line_segs)
}

//Returns sequences per grid square
fn build_edge_sequences(
    mut new_edge_list: VecDeque<Edge2>, num_squares: usize, num_columns: u32) -> Result<Vec<Vec<Vec<RasterCoord>>>> {
    let mut edge_sequences: Vec<Vec<Vec<RasterCoord>>> = Vec::with_capacity(num_squares);
    for _ in 0..num_squares {
        edge_sequences.push(vec![]);
    }

    let mut loop_check = 0;

    //Move edges until we are starting on the grid
    //this maintains the clockwise
    while !new_edge_list[0].pt_from.on_grid() {
        loop_check += 1;

        let edge = new_edge_list.pop_front().unwrap();
        new_edge_list.push_back(edge);

        // if loop_check > new_edge_list.len() {
        //     println!("Printing list for error");
        //     for e in new_edge_list.iter() {
        //         println!("Edge {}", e);
        //     }
        // }
        assert!(loop_check <= new_edge_list.len());
    }

    while !new_edge_list.is_empty() {
        let mut to_index = 0;

        let grid_row = new_edge_list[0].get_grid_row();
        let grid_col = new_edge_list[0].get_grid_col();

        let grid_index = (grid_col + grid_row * num_columns as i32) as usize;

        //find until we are on the grid line again.  This means we have edges starting and stoping on
        //the grid square line
        while !new_edge_list[to_index].pt_to.on_grid() {
            //sanity check to make sure we aren't leaving the grid
            let check_row = new_edge_list[to_index].get_grid_row();
            let check_col = new_edge_list[to_index].get_grid_col();

            // if grid_row != check_row || grid_col != check_col {
            //     for e in new_edge_list.iter() {
            //         println!("Remaining edge: {}", e);
            //     }
            // }
            assert_eq!(grid_row, check_row);
            assert_eq!(grid_col, check_col);

            to_index += 1;
            assert!(to_index < new_edge_list.len());
        }

        let last_point = new_edge_list[to_index].pt_to.clone();
        let seq = new_edge_list.drain(0..=to_index);

        let mut point_list = Vec::with_capacity(to_index + 1);
        for edge in seq {
            point_list.push(edge.pt_from.clone());
        }
        point_list.push(last_point);

        edge_sequences[grid_index].push(point_list);
    }

    Ok(edge_sequences)
}

fn handle_polygon_not_touching_grid_square(
    new_edge_list: &VecDeque<Edge2>,
    grid_line_segs: &mut Vec<Vec<Vec<RasterCoord>>>,
    num_columns: u32,
) -> Result<bool>
{
    let mut any_starting_on_grid = false;

    for e in new_edge_list.iter() {
        //println!("Edge: {} is {}", e_idx, e);
        assert!(!e.crosses_grid_line());

        if e.pt_from.on_grid() {
            any_starting_on_grid = true;
        }
    }

    //Edge case of entire polygon is within a grid square
    if !any_starting_on_grid {
        let grid_col = new_edge_list[0].get_grid_col();
        let grid_row = new_edge_list[0].get_grid_row();

        let grid_index = (grid_col + grid_row * num_columns as i32) as usize;

        let mut line = new_edge_list.into_iter().map(|e| e.pt_from).collect_vec();
        line.push(line[0].clone());
        grid_line_segs[grid_index].push(line);
    }

    return Ok(any_starting_on_grid);
}

fn get_starting_hole(mp_for_square: &Vec<PolyRasterCoord>,
                     grid_row: f64,
                     grid_col: f64,
) -> MultiPolygon<f64> {
    if mp_for_square.is_empty() {
        debug!("Creating a square to start subtracting holes from");
        let mut square_ls: Vec<Coordinate<f64>> = Vec::with_capacity(5);
        square_ls.push(Coordinate { y: grid_row, x: grid_col });
        square_ls.push(Coordinate { y: grid_row + 1., x: grid_col });
        square_ls.push(Coordinate { y: grid_row + 1., x: grid_col + 1. });
        square_ls.push(Coordinate { y: grid_row, x: grid_col + 1. });
        square_ls.push(Coordinate { y: grid_row, x: grid_col });
        let square = RustPolygon::new(LineString(square_ls), vec![]);
        MultiPolygon(vec![square])
    } else {
        //create a multipolygon of the added polygons as a starting point
        //We can assume there are no holes
        let polygons = mp_for_square.iter().map(|s| {
            assert!(s.holes.is_empty());

            let coord_vec = s.exterior.iter().map(|c| {
                if !c.on_grid() {
                    assert_eq!(c.col.floor(), grid_col);
                    assert_eq!(c.row.floor(), grid_row);
                }
                Coordinate {
                    x: c.col,
                    y: c.row,
                }
            }).collect_vec();
            RustPolygon::new(LineString(coord_vec), vec![])
        }).collect_vec();

        debug!("Creating a multipolygon of size {} to start subtracting holes from", polygons.len());

        MultiPolygon(polygons)
    }
}

//Slices a polygon along the given grid
fn grid_slice_multi_polygon(window_stats: &RasterStats, geometry: &Geometry,
                            rasterized: &DefBitVec,
                            layer_def: &LayerDefinition, layer: &Layer,
                            fid: i64,
) -> Result<()>
{
    //we need to intersect all grid lines with the geometry

    assert_eq!(geometry.geometry_type(), OGRwkbGeometryType::wkbMultiPolygon);

    //We need to distinguish between polygons and their holes

    //Read in all the edges
    let (poly_edge_list, hole_edge_list) = read_edges(window_stats, geometry);
    debug!("Have {} polygons {} holes", poly_edge_list.len(), hole_edge_list.len());

    //These 3d vectors are lists of multipolygons, with no interior holes
    //This is because these are split into 2 and dealt with seperately

    //The size is the total number of raster squares in the reference raster slice needed for this geometry

    let mut grid_multi_polygons = edge_list_to_polygons(poly_edge_list, window_stats);

    let mut grid_multi_polygon_holes = edge_list_to_polygons(hole_edge_list, window_stats);

    debug!("Merging polygons and holes");

    for grid_index in 0..grid_multi_polygons.len() {
        let grid_row = (grid_index / window_stats.num_cols as usize) as f64;
        let grid_col = (grid_index % window_stats.num_cols as usize) as f64;

        //debug!("Grid index {}", grid_index);

        let has_holes = !grid_multi_polygon_holes[grid_index].is_empty();

        if has_holes {
            //Now we need to process the holes.  Either we start with a square, or we start with
            //the above
            let mut hole_mp = get_starting_hole(&grid_multi_polygons[grid_index],                
                grid_row,
                grid_col
            );

            for hole_lines in grid_multi_polygon_holes[grid_index].drain(..) {
                //we need to calculate the inverse of this
                //we can assume the initial holes have no holes, it is only when we calc differences that we have them
                assert!(hole_lines.holes.is_empty());
                let hole_coords = hole_lines.exterior.into_iter().map(|rc|
                    Coordinate { x: rc.col, y: rc.row }).collect_vec();

                let hole_ls = LineString(hole_coords);

                let hole = RustPolygon::new(hole_ls, Vec::new());

                hole_mp = hole_mp.difference(&hole);
            }

            //Since the holes were subtracted from the polygons, we clear what was added above
            grid_multi_polygons[grid_index].clear();

            for p in hole_mp.0 {

                let hole_exterior = p.exterior().0.iter().map(|c| RasterCoord {
                    col: c.x,
                    row: c.y,
                }).collect_vec();

                //this is a line
                if hole_exterior.len() < 4 {
                    // for h in hole.iter() {
                    //     println!("Hole point: {}", h);
                    // }
                    continue;
                }

                assert!(hole_exterior.len() >= 4);
                assert!(hole_exterior[0] == hole_exterior[hole_exterior.len() - 1]);

                //holes can have holes :)  Since the holes are treated like normal polygons here
                let mut hole_interiors = Vec::new();

                for i in p.interiors().iter() {
                    let hole_interior = i.0.iter().map(|c| RasterCoord {
                        col: c.x,
                        row: c.y,
                    }).collect_vec();

                    hole_interiors.push(hole_interior);
                }

                grid_multi_polygons[grid_index].push(PolyRasterCoord {
                    exterior: hole_exterior,
                    holes: hole_interiors
                });

            }
        }

        let mp_is_empty = grid_multi_polygons[grid_index].is_empty();
        //if this is part of the raster, we need to draw a square
        if mp_is_empty && rasterized[grid_index as usize] {
            let mut line = Vec::new();
            line.push(RasterCoord { row: grid_row, col: grid_col });
            line.push(RasterCoord { row: grid_row + 1., col: grid_col });
            line.push(RasterCoord { row: grid_row + 1., col: grid_col + 1. });
            line.push(RasterCoord { row: grid_row, col: grid_col + 1. });
            line.push(RasterCoord { row: grid_row, col: grid_col });

            grid_multi_polygons[grid_index].push(
                PolyRasterCoord {
                    exterior: line,
                    holes: vec![]
                }
                );
        }

    }

    debug!("Writing everything to database");

    write_slice_polys(
        layer_def, layer,        
        window_stats,
        &grid_multi_polygons.as_slice(),
        fid,
    )?;

    Ok(())
}

struct PolyRasterCoord {
    exterior: Vec<RasterCoord>,
    holes: Vec<Vec<RasterCoord>>,
}

fn edge_list_to_polygons(edge_list: Vec<Vec<RasterCoord>>,
    window_stats: &RasterStats
) -> Vec<Vec<PolyRasterCoord>> {
    let mut multi_polygon_list: Vec<Vec<PolyRasterCoord>> = Vec::with_capacity((window_stats.num_cols * window_stats.num_rows) as usize);

    for _ in 0..(window_stats.num_cols * window_stats.num_rows) as usize {
        multi_polygon_list.push(Vec::new());
    }

    for pe in edge_list.into_iter() {

        //While not fully optimal, for convenience, these returns
        let mut pe_grid = grid_slice_single_polygon(window_stats, pe).unwrap();

        for grid_index in 0..multi_polygon_list.len() {
            //drain is used to recover memory and to efficiently transfer from one vec to another
            for poly_lines in pe_grid[grid_index].drain(..) {
                multi_polygon_list[grid_index].push(
                    PolyRasterCoord{
                        exterior: poly_lines,
                        holes: vec![]
                    }
                    );
            }
        }
    }

    multi_polygon_list
}

//edges that do not cross grid lines
fn create_nocross_edges(cord_list: Vec<RasterCoord>) -> VecDeque<Edge2> {
    let mut edge_list = cord_list.windows(2).map(|pt_slice| {
        Edge2::new(pt_slice[0].clone(),
                   pt_slice[1].clone())
    }).collect_vec();

    //since we process from the end
    edge_list.reverse();

    let mut new_edge_list = VecDeque::with_capacity(edge_list.capacity());

    assert!(edge_list.len() >= 3);
    assert_eq!(edge_list.len(), cord_list.len() - 1);

    while !edge_list.is_empty() {
        let next_edge = edge_list.pop().unwrap();
        
        let from_row = next_edge.pt_from.row.floor();
        let from_col = next_edge.pt_from.col.floor();
        
        let to_row = next_edge.pt_to.row.floor();
        let to_col = next_edge.pt_to.col.floor();
        
        //skip edges that are exactly on the grid lines
        if from_col == to_col && from_col == next_edge.pt_from.col &&
            to_col == next_edge.pt_to.col {
            continue;
        }
        if from_row == to_row && from_row == next_edge.pt_from.row &&
            to_row == next_edge.pt_to.row {
            continue;
        }

        if crosses_int_bounary(next_edge.pt_from.row, next_edge.pt_to.row) {
            
            //let to_row = next_edge.pt_to.row.floor();

            let new_row = if next_edge.pt_from.row < next_edge.pt_to.row {
                //so like row 2.4 => 2.0 to row 3.0 because to_row is >= 3.0 (increasing)
                from_row + 1.0
            } else if next_edge.pt_from.row == from_row {
                //like row 3.0 => something lower so 2.0
                from_row - 1.0
            } else {
                //like row 2.4 => row 2 (decreasing row from => to)
                from_row
            };

            let new_col = next_edge.col_at_row(new_row);
            let new_to = Edge2::new(
                RasterCoord { row: new_row, col: new_col },
                next_edge.pt_to.clone(),
            );
            let new_from = Edge2::new(next_edge.pt_from.clone(),
                                      RasterCoord { row: new_row, col: new_col },
            );
            //println!("Pushing ROW edge {} remaining {}", &new_from, &new_to);

            edge_list.push(new_to);
            edge_list.push(new_from);

            //new_edge_list.push_back(new_from);
        } else if crosses_int_bounary(next_edge.pt_from.col,
                                      next_edge.pt_to.col) {

            // from < to
            // 2.0 < 3.0
            // 2.1 < 3.1
            // new col is from from.floor + 1
            // 3.0 > 1.x if ==, then - 1
            // 2.3 > 1.3 just floor
            
            //let to_col = next_edge.pt_to.col.floor();

            let new_col = if next_edge.pt_from.col < next_edge.pt_to.col {
                from_col + 1.0
            } else if next_edge.pt_from.col == from_col {
                from_col - 1.0
            } else {
                from_col
            };
            let new_row = next_edge.row_at_col(new_col);
            let new_to = Edge2::new(
                RasterCoord { row: new_row, col: new_col },
                next_edge.pt_to.clone(),
            );
            let new_from = Edge2::new(next_edge.pt_from.clone(),
                                      RasterCoord { row: new_row, col: new_col },
            );
            //println!("Pushing COL edge {} remaining {}", &new_from, &new_to);

            edge_list.push(new_to);
            new_edge_list.push_back(new_from);
        } else {
            //println!("Pushing inside edge {}", &next_edge);
            new_edge_list.push_back(next_edge);
        }
    }

    new_edge_list.make_contiguous();

    new_edge_list
}

fn add_corners(
    start_point: &RasterCoord, stop_point: &RasterCoord,
    grid_row: i32, grid_col: i32,
    point_list: &mut Vec<RasterCoord>,
)
{
    let start_side = start_point.get_side(grid_row, grid_col);
    let stop_side = stop_point.get_side(grid_row, grid_col);
    // println!("Adding corners from {} to {}, sides {:?} to {:?}",
    //     start_point, stop_point, start_side, stop_side
    // );

    let mut side = start_side;

    //steps top to top 4
    //top to right 3
    let mut num_steps = 4 - (4 + start_side as i32 - stop_side as i32) % 4;

    //one correction, if the same side, and it's clockwise, we do 0 steps
    if start_side == stop_side && start_point.is_clockwise(
        start_side,
        &stop_point) {
        num_steps = 0;
    }

    // println!("Adding sides for polygon from {:?} to {:?}.  Number steps: {}",
    //           start_side, stop_side, num_steps);

    for _ in 0..num_steps {
        //clockwise corner from side
        let point = match side {
            Side::Top => (grid_row, grid_col + 1),
            //lower right corner
            Side::Right => (grid_row + 1, grid_col + 1),
            //lower left corner
            Side::Bottom => (grid_row + 1, grid_col),
            //upper left corner
            Side::Left => (grid_row, grid_col),
        };
        point_list.push(RasterCoord {
            row: point.0 as f64,
            col: point.1 as f64,
        });

        side = increment_side(side);
    }
}

fn grid_boundary_clockwise_distance(
    start: &RasterCoord,
    stop: &RasterCoord,
    grid_row: i32,
    grid_col: i32,
) -> f64 {
    let pol_start = get_polar(start, grid_row, grid_col);
    let pol_stop = get_polar(stop, grid_row, grid_col);

    clockwise_distance(pol_start, pol_stop)
}

fn clockwise_distance(pos1: f64, pos2: f64) -> f64 {
    if pos2 > pos1 {
        return pos2 - pos1;
    }

//pos 1 is greater, so finish the loop and go to pos2
    return 4.0 - pos1 + pos2;
}

fn get_polar(pt: &RasterCoord, grid_row: i32, grid_col: i32) -> f64
{
//use convention, starting from upper left
//0 to 1; 1 to 2 right side; 2 to 3 bottom, 3 to 4 left

    let side = pt.get_side(grid_row, grid_col);
    let mut ans = side as i32 as f64;

    assert!(pt.col >= grid_col as f64);
    assert!(pt.row >= grid_row as f64);
    if pt.col > 1.0 + grid_col as f64 {
        println!("{} {} vs {} {}",
                 pt.row, pt.col, grid_row, grid_col
        )
    }
    assert!(pt.col <= 1.0 + grid_col as f64);
    assert!(pt.row <= 1.0 + grid_row as f64);

    ans += match side {
        Side::Top => pt.col - grid_col as f64,
        Side::Right => pt.row - grid_row as f64,
        Side::Bottom => 1.0 + grid_col as f64 - pt.col,
        Side::Left => 1.0 + grid_row as f64 - pt.row,
    };

    ans
}

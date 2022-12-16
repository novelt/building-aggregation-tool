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
use std::collections::{HashMap, VecDeque};
use std::time::Instant;

use anyhow::Result;
use itertools::Itertools;
use partitions::PartitionVec;
//Counts points or centroids and outputs a raster
use structopt::StructOpt;

use geo_util::pq::PgConnection;
use geo_util::raster::{Raster, RasterStats};
use geo_util::util::print_remaining_time;
use geos::{SimpleContextHandle, SimpleCoordinateSequence, SimpleGeometry};

use crate::contour::main::{Contours, Field, march};
//use crate::contour::simply::simplify;


#[derive(StructOpt)]
pub struct ContourArgs {
    //This raster is assumed to be in 4326
    #[structopt(parse(from_os_str), long)]
    pub input_raster: std::path::PathBuf,

    #[structopt(long, default_value = "data_work")]
    schema_name: String,

    #[structopt(long)]
    line_layer_name: String,

    #[structopt(long)]
    pg_conn_str: String,

    #[structopt(long)]
    contour_value: f64,

}


//const SRID: i32 = 42580;
const SRID: i32 = 4326;

type Point = (f64, f64);

struct RasterWindow {
    num_cols: usize,
    num_rows: usize,
    data: Vec<f64>,
}

impl Field for RasterWindow {
    fn dimensions(&self) -> (usize, usize) {
        (self.num_cols, self.num_rows)
    }

    fn z_at(&self, x: usize, y: usize) -> f64 {
        let index = y * self.num_cols + x;
        self.data[index]
    }
}

pub(crate) fn main_contour_lines(args: &ContourArgs) -> Result<()> {
    let input_raster = Raster::read(&args.input_raster, true);

    //let extent = GdalGeometry::bbox(args.x_min, args.y_min, args.x_max, args.y_max)?;

    //let mut dataset = Driver::open_vector_static(&args.ogr_conn_str, false, &vec![])?;


    //Get the extent into the rasters coordinate system
    let raster_sr = input_raster.dataset.spatial_reference()?;
    //raster_sr.auto_identify_epsg();
    println!("Proj string: {}", raster_sr.to_proj4()?);
    println!("WKT string: {}", raster_sr.to_pretty_wkt()?);


    let offset = (0,0);
    let size = (input_raster.stats.num_cols as i32, input_raster.stats.num_rows as i32);

    let band = input_raster.dataset.rasterband(1)?;


    let data = band.read_as::<f64, i32>(
        offset.clone(),
        size,
    ).unwrap();

    let mut vv_data = Vec::with_capacity(size.1 as usize);

    let height = size.1 as usize;
    let width = size.0 as usize;

    for row in 0..height {
        let start_idx = row * width;
        let end_idx = start_idx + width;
        let row_data = (&data[start_idx..end_idx]).to_vec();
        vv_data.push(row_data);
    }

    //For performance, we won't reproject the contour lines, they will be in the input rasters spatial projection


    create_contour_table(&args)?;


    //println!("Min height: {:.2} Max Height: {:.2}", min_height, max_height);


    let start = Instant::now();
    let mut last_output = Instant::now();


    let pg_conn_polygon = PgConnection::new(&args.pg_conn_str)?;



    let copy_sql_poly = format!("
COPY {SCHEMA}.{TABLE}_polygons (is_hole, shape)
    FROM STDIN  BINARY
", SCHEMA = args.schema_name, TABLE = args.line_layer_name);

    // let field = Field {
    //     dimensions: (width, height),
    //     top_left: (0.0, 0.0),
    //     pixel_size: (1.0, 1.0),
    //     values: &vv_data,
    // };




    pg_conn_polygon.copy_start(&copy_sql_poly)?;

    //let lines = field.get_contours(args.contour_value as _);

    let raster_window = RasterWindow {
        num_cols: size.0 as usize,
        num_rows: size.1 as usize,
        data,
    };

    let contours = march(&raster_window, args.contour_value)
                .into_iter()
                //.map(|p| simplify(&p))
                .collect::<Vec<_>>();

    let stats = &input_raster.stats;

    let mut deq = process_contours(
        contours, args, &stats).unwrap();

    let context = SimpleContextHandle::new();

    // //TODO
    // deq.clear();

    let mut num_processed = 0;
    let num_steps = deq.len();

    while !deq.is_empty() {
        let v = deq.pop_front().unwrap();

        if v.len() <= 3 {
            continue;
        }

        //println!("Queue len {}", deq.len());

        num_processed += 1;

        // if num_processed > 17000 {
        //     break;
        // }

        assert!(check_point_line(&v));

        // let mut exterior_ring = GdalGeometry::empty(OGRwkbGeometryType::wkbLineString)?;

        //the line string may intersect, so we want to add polygons
        //clockwise means its a hole
        //counter clockwise means its a shell

        let dbl = find_doubles(&v);

        if let Some(dbl) = dbl {

            //This is the case where the entire linestring is the same point
            if dbl[0] <= dbl[1] && (1 + dbl[1] - dbl[0]) == v.len() {
                //println!("Skipping all doubles");
                continue;
            }

            // println!("Doubles found! @ {:?} with values {:?} and {:?}.  Total num points: {}  queue len {}", dbl,
            //          countour_point_to_coords(&offset, stats, &v[dbl[0]]),
            //         countour_point_to_coords(&offset, stats, &v[dbl[0]]),
            //     v.len(),
            //     deq.len()
            // );

            //now we cut
            if dbl[0] <= dbl[1] {
                assert_ne!(dbl[0], dbl[1]);

                //cut out dbl[0] to dbl[1]
                //and replace with dbl[0] in the other

                //add 2 linestrings

                //No need to process consequtive doubles
                if dbl[1] != dbl[0] + 1 {
                    let cut = v[dbl[0]..=dbl[1]].to_vec();

                    if !check_point_line(&cut) {
                        println!("Invalid cut");
                        print_points(&v, &offset, stats);
                        print_points(&cut, &offset, stats);
                        break;
                    }

                    deq.push_back(cut);
                }

                //now we need 0 to dbl[0] - 1 ; cut[0] ; dbl[1] to end

                let mut cut2 = Vec::new();
                if dbl[0] > 0 {
                    cut2.extend_from_slice(&v[0..dbl[0]]);
                }

                cut2.push(v[dbl[0]].clone());

                if dbl[1] < v.len() - 1 {
                    cut2.extend_from_slice(&v[dbl[1] + 1..v.len()]);
                }

                if !check_point_line(&cut2) {
                    println!("Invalid cut");
                    print_points(&v, &offset, stats);
                    print_points(&cut2, &offset, stats);
                    break;
                }

                deq.push_back(cut2);
            } else {
                //This means we have more than 2 doubles that include the beggining of the line string
                //note we need to make sure that we still start and end on the same point
                //so we can automatically throw out the ends
                assert!(dbl[1] + 1 < dbl[0]);
                let len_check = v.len() - dbl[0] + dbl[1] + 1;
                assert!(len_check > 2 && len_check < v.len());

                let mut cut = Vec::new();
                cut.push(v[dbl[1]].clone());
                cut.extend_from_slice(&v[dbl[1] + 1..dbl[0]]);
                cut.push(v[dbl[0]].clone());

                if !check_point_line(&cut) {
                    println!("Invalid cut 1b");
                    print_points(&v, &offset, stats);
                    print_points(&cut, &offset, stats);
                    break;
                }

                deq.push_back(cut);

                let mut cut2 = Vec::new();

                cut2.extend_from_slice(&v[dbl[0]..v.len()]);
                cut2.extend_from_slice(&v[0..=dbl[1]]);

                if !check_point_line(&cut2) {
                    println!("Invalid cut 2b");
                    print_points(&v, &offset, stats);
                    print_points(&cut2, &offset, stats);
                    break;
                }

                deq.push_back(cut2);
            }

            continue;
        }

        //clockwise is out
        //counter clockwise is in

        // add_line_string(num_processed + lines_num, &v, &offset,
        //                 stats, &context, &pg_conn_line, &pg_conn_point, &pg_conn_polygon, true, "filtered").unwrap();

        add_poly( &v, &offset,
                        stats, &context,  &pg_conn_polygon, ).unwrap();


        if last_output.elapsed().as_secs() >= 3 {
            last_output = Instant::now();
            print_remaining_time(&start,
                                 num_processed as u32,
                                 num_steps as _);
        }
    }

    let copy_result = pg_conn_polygon.copy_end()?;

    println!("Result: {} {}", copy_result.status_str(), copy_result.error_message());

    copy_result.is_ok().unwrap();

    Ok(())
}

fn process_contours(contours: Contours, args: &ContourArgs, stats: &RasterStats) -> Result<VecDeque<Vec<(f64,f64)>>>
{
    //This will also save the points and lines for debugging
    let context = SimpleContextHandle::new();
    let pg_conn_line = PgConnection::new(&args.pg_conn_str)?;
    let pg_conn_point = PgConnection::new(&args.pg_conn_str)?;

    let copy_sql_line = format!("
COPY {SCHEMA}.{TABLE}_lines (id, comment, shape)
    FROM STDIN  BINARY
", SCHEMA = args.schema_name, TABLE = args.line_layer_name);
//
    let copy_sql_point = format!("
COPY {SCHEMA}.{TABLE}_points (line_id, point_num, comment, shape)
    FROM STDIN  BINARY
", SCHEMA = args.schema_name, TABLE = args.line_layer_name);


    pg_conn_line.copy_start(&copy_sql_line)?;
    pg_conn_point.copy_start(&copy_sql_point)?;


    let offset = (0,0);
    let mut deq = VecDeque::new();

    //let lines_num = lines.len();

    for (idx, v) in contours.into_iter().enumerate() {
    //for (idx, v) in lines.into_iter().enumerate() {
        add_points(idx, &v, &offset,
                        stats, &context, &pg_conn_point, "orig").unwrap();
        add_line_string(idx, &v, &offset,
                        stats, &context, &pg_conn_line,  "ls").unwrap();



        if v.len() <= 2 {
            continue;
        }
        //println!("Index: {}", idx);
        // print_points(&v.points, &offset, stats);
        //
        if has_non_consecutive_doubles(&v) {
            print_points(&v, &offset, stats);
        }
        //
        // assert!(check_point_line(&v.points));

        deq.push_back(v);
    }

    let copy_result = pg_conn_line.copy_end()?;

    println!("Result: {} {}", copy_result.status_str(), copy_result.error_message());

    copy_result.is_ok().unwrap();

    let copy_result = pg_conn_point.copy_end()?;

    println!("Result: {} {}", copy_result.status_str(), copy_result.error_message());

    copy_result.is_ok().unwrap();

    Ok(deq)
}

fn create_contour_table(args: &ContourArgs) -> Result<()> {
    let pg_conn = PgConnection::new(&args.pg_conn_str).unwrap();

    let query = format!("
/*DELETE FROM spatial_ref_sys
WHERE srid = SRID;

INSERT INTO spatial_ref_sys (srid, auth_name, auth_srid, proj4text, srtext)
VALUES (
    SRID, 'epsg', SRID, 'PROJ', 'WKT'
);*/

DROP TABLE IF EXISTS {SCHEMA_DATA_WORK}.{TABLE}_lines;
DROP TABLE IF EXISTS {SCHEMA_DATA_WORK}.{TABLE}_points;
DROP TABLE IF EXISTS {SCHEMA_DATA_WORK}.{TABLE}_polygons;

CREATE UNLOGGED TABLE {SCHEMA_DATA_WORK}.{TABLE}_lines
(
    id serial PRIMARY KEY,
    shape Geometry(LineString, {SRID}) NOT NULL,
    comment text
);

CREATE UNLOGGED TABLE {SCHEMA_DATA_WORK}.{TABLE}_points
(
    id serial PRIMARY KEY,
    line_id int NOT NULL,
    point_num int NOT NULL,
    comment text,
    shape Geometry(Point, {SRID}) NOT NULL
);

CREATE UNLOGGED TABLE {SCHEMA_DATA_WORK}.{TABLE}_polygons
(
    id serial PRIMARY KEY,
    is_hole boolean NOT NULL,
    shape Geometry(Polygon, {SRID}) NOT NULL
);

", SCHEMA_DATA_WORK = args.schema_name,
                        TABLE = args.line_layer_name,
                       // PROJ = raster_sr.to_proj4()?,
                       // WKT = raster_sr.to_wkt()?,
                        SRID = SRID
    );

    let r = pg_conn.execute(&query);

    r.is_ok()
}


fn order_indicies(i1: usize, i2: usize, num_points: usize) -> ([usize; 2], usize) {
    assert!(i2 > i1);
    let forward = (i2 - i1) + 1;
    let backward = 2 + num_points - forward;

    return if forward <= backward {
        ([i1, i2], forward)
    } else {
        ([i2, i1], backward)
    };
    //0 1 2 3 4 5 6
    // 1 to 6 == 6
    // 6 to 1 == 3
    // (2-0) + 1 + (6-5) + 1
    // np - i2 + i1 + 1
    // 2+np - i2+i1 - 1
    // np - i2 + i1 + 1

    // 0 + 1 + 7 - 2 + 1 == 5

    // i2 - i1 + 1 + i1 - i2 + np + 3
}

//returns list of point indicies, in reverse order if needs to go backwards to be smallest loop


//First and last point excepted to be the same
fn find_doubles(points: &Vec<Point>) -> Option<[usize; 2]>
{
    //let mut retSet = HashSet::new();
    //let mut ret = Vec::new();

    let mut smallest_size = points.len() * 2;
    let mut ret = None;

    let mut map: HashMap<[i64; 2], usize> = HashMap::new();

    let mut pvec = PartitionVec::with_capacity(points.len());

    for (pidx, p) in points.iter().take(points.len()).enumerate() {
        let key = [convert_f64_to_int(p.0), convert_f64_to_int(p.1)];

        let idx = map.entry(key).or_insert(pidx);

        pvec.push(pidx);

        if *idx != pidx {
            pvec.union(pidx, *idx);
        }
    }

    //This should be a closed line string
    assert!(pvec.same_set(0, points.len() - 1));

    //Now go through and see if we have any doubles
    //don't check the last point
    for pidx in 0..points.len() - 1 {
        let doubles = pvec.len_of_set(pidx);

        if doubles <= 1 {
            continue;
        }

        if doubles == points.len() {
            //println!("Everything is a double");
            return Some([0, points.len() - 1]);
        }

        let mut dup_points = pvec.set(pidx).map(|(i, _v)| i).collect_vec();
        dup_points.sort();

        if doubles == 2 && dup_points[0] == 0 && dup_points[1] == points.len() - 1 {
            //normal beginning and end
            continue;
        }

        if doubles > 2 {

            // println!("Debug points: {}, Total num points: {}",
            //          dup_points.len(),
            //          points.len());
            //
            // for p in dup_points.iter() {
            //     println!("debug Double: {} value: {:?}", p, points[*p]);
            // }

            //If these are all consequtive, return the entire one

            //If we dont have the first index, we can check directly
            if dup_points[0] != 0 {
                if dup_points[dup_points.len() - 1] - dup_points[0] == dup_points.len() - 1 {

                    //short circuit return this
                    //println!("Return conseq list doubles");
                    return Some([dup_points[0], dup_points[dup_points.len() - 1]]);
                }
            } else {

                //check both ends
                let mut left_idx = 0;
                let mut right_idx = dup_points.len() - 1;

                while left_idx < dup_points.len() - 1 {
                    if dup_points[left_idx] + 1 != dup_points[left_idx + 1] {
                        break;
                    }
                    left_idx += 1;
                }

                while right_idx > 0 {
                    if dup_points[right_idx - 1] + 1 != dup_points[right_idx] {
                        break;
                    }
                    right_idx -= 1;
                }

                if right_idx == left_idx + 1 {
                    //println!("Return conseq list doubles 2");
                    return Some([dup_points[right_idx], dup_points[left_idx]]);
                }
            }

            // println!("Problem!  Too many non consequcitive doubles: {}, Total num points: {}",
            //          dup_points.len(),
            //          points.len());
            //
            // for p in dup_points.iter() {
            //     println!("Double: {} value: {:?}", p, points[*p]);
            // }

            //See docs/begin_end.png, it can be ok to have many doubles
        }

        let (points_ordered, len_sub_loop) = order_indicies(dup_points[0], dup_points[1], points.len());

        if len_sub_loop < smallest_size {
            smallest_size = len_sub_loop;
            ret = Some(points_ordered);
        }
    }

    return ret;
}

fn has_non_consecutive_doubles(points: &Vec<Point>) -> bool
{
    //let mut retSet = HashSet::new();
    //let mut ret = Vec::new();

    let mut map: HashMap<[i64; 2], usize> = HashMap::new();

    let mut pvec = PartitionVec::with_capacity(points.len());

    for (pidx, p) in points.iter().take(points.len()).enumerate() {
        let key = [convert_f64_to_int(p.0), convert_f64_to_int(p.1)];

        let idx = map.entry(key).or_insert(pidx);

        pvec.push(pidx);

        if *idx != pidx {
            pvec.union(pidx, *idx);
        }
    }

    //This should be a closed line string
    assert!(pvec.same_set(0, points.len() - 1));

    //Now go through and see if we have any doubles
    //don't check the last point
    for pidx in 0..points.len() - 1 {
        let doubles = pvec.len_of_set(pidx);

        if doubles <= 1 {
            continue;
        }

        if doubles == points.len() {
            //println!("Everything is a double");
            return false;
        }

        let mut dup_points = pvec.set(pidx).map(|(i, _v)| i).collect_vec();
        dup_points.sort();

        //ok if we include the end, see docs/begin_end.png
        if dup_points[0] == 0 {
            continue;
        }

        if doubles == 2 && dup_points[0] == 0 && dup_points[1] == points.len() - 1 {
            //normal beginning and end
            continue;
        }

        if doubles > 2 {


            //If these are all consequtive, return the entire one

            //If we dont have the first index, we can check directly
            if dup_points[0] != 0 {
                if dup_points[dup_points.len() - 1] - dup_points[0] == dup_points.len() - 1 {

                    //short circuit return this
                    //println!("Return conseq list doubles");
                    return false;
                }
            } else {

                //check both ends
                let mut left_idx = 0;
                let mut right_idx = dup_points.len() - 1;

                while left_idx < dup_points.len() - 1 {
                    if dup_points[left_idx] + 1 != dup_points[left_idx + 1] {
                        break;
                    }
                    left_idx += 1;
                }

                while right_idx > 0 {
                    if dup_points[right_idx - 1] + 1 != dup_points[right_idx] {
                        break;
                    }
                    right_idx -= 1;
                }

                if right_idx == left_idx + 1 {
                    //println!("Return conseq list doubles 2");
                    return false;
                }
            }

            println!("Problem!  Too many non consecutive doubles: {}, Total num points: {}",
                     dup_points.len(),
                     points.len());

            for p in dup_points.iter() {
                println!("Double: {} value: {:?} key {:?}", p, points[*p], [convert_f64_to_int(points[*p].0), convert_f64_to_int(points[*p].1)]);
            }

            return true;
        }
    }

    return false;
}

// const PRECISION: f64 = 100000.;
// #[inline]
// fn convert_float_to_int(f: f64) -> i32 {
//     return (f * PRECISION) as i32;
// }

const PRECISION_FLOAT: f64 = 100000.;

#[inline]
fn convert_f64_to_int(f: f64) -> i64 {
    return (f * PRECISION_FLOAT) as i64;
}

fn countour_point_to_coords(offset: &(i32, i32),
                            stats: &RasterStats, pt: &Point) -> [f64; 2] {
    let pt_x = stats.origin_x + stats.pixel_width * (pt.0 as f64 + offset.0 as f64 + 0.5);
    let pt_y = stats.origin_y + stats.pixel_height * (pt.1 as f64 + offset.1 as f64 + 0.5);

    [pt_x, pt_y]
}


fn add_points(
    index: usize,
    p: &Vec<Point>, offset: &(i32, i32),
    stats: &RasterStats,
    context: &SimpleContextHandle,
    pg_conn_point: &PgConnection,
    comment: &str
    ) -> Result<()> {


    for (idx, pt) in p.iter().enumerate() {
        //these points are raster coords

        let [pt_x, pt_y] = countour_point_to_coords(offset, stats, pt);

        let geom_point = SimpleGeometry::create_point_xy(&context, pt_x, pt_y).unwrap();
        geom_point.set_srid(SRID);
        pg_conn_point.copy_field_count(4)?;
        pg_conn_point.copy_int(index as _)?;
        pg_conn_point.copy_int((1 + idx) as _)?;
        pg_conn_point.copy_str(comment)?;

        let ewkb = geom_point.ewkb()?;
        pg_conn_point.copy_bytes(ewkb.as_ref())?;
    }


    Ok(())
}

fn add_line_string(
    index: usize,
    p: &Vec<Point>, offset: &(i32, i32),
    stats: &RasterStats,
    context: &SimpleContextHandle,
    pg_conn_line: &PgConnection,
    comment: &str
    ) -> Result<()> {
    let mut coord_seq = SimpleCoordinateSequence::new(p.len() as _,
                                                      &context)?;

    for (idx, pt) in p.iter().enumerate() {
        //these points are raster coords

        let [pt_x, pt_y] = countour_point_to_coords(offset, stats, pt);

        //exterior_ring.add_point(pt_x, pt_y);
        coord_seq.set_x(idx as _, pt_x)?;
        coord_seq.set_y(idx as _, pt_y)?;
    }


    let line_string = SimpleGeometry::create_line_string(coord_seq)?;

    //Number of fields
    pg_conn_line.copy_field_count(3)?;

    pg_conn_line.copy_int(index as _)?;

    pg_conn_line.copy_str(comment)?;

    line_string.set_srid(SRID);
    let ewkb = line_string.ewkb()?;

    pg_conn_line.copy_bytes(ewkb.as_ref())?;

    return Ok(());

}


fn add_poly(
    p: &Vec<Point>, offset: &(i32, i32),
    stats: &RasterStats,
    context: &SimpleContextHandle,
    pg_conn_poly: &PgConnection,
    ) -> Result<()> {
    let mut coord_seq = SimpleCoordinateSequence::new(p.len() as _,
                                                      &context)?;

    for (idx, pt) in p.iter().enumerate() {
        //these points are raster coords

        let [pt_x, pt_y] = countour_point_to_coords(offset, stats, pt);

        //exterior_ring.add_point(pt_x, pt_y);
        coord_seq.set_x(idx as _, pt_x)?;
        coord_seq.set_y(idx as _, pt_y)?;

    }

    pg_conn_poly.copy_field_count(2)?;

    //clockwise means its a hole
    let is_ccw = coord_seq.is_ccw().unwrap();

    let line_ring = SimpleGeometry::create_linear_ring(coord_seq)?;

    let poly = SimpleGeometry::create_polygon(line_ring, vec![])?;

    pg_conn_poly.copy_boolean(!is_ccw)?;

    poly.set_srid(SRID);
    let ewkb = poly.ewkb()?;

    pg_conn_poly.copy_bytes(ewkb.as_ref())?;

    Ok(())
}

fn print_points(points: &Vec<Point>, offset: &(i32, i32), stats: &RasterStats) {
    println!("Total num points: {}",
             points.len());

    for (pidx, p) in points.iter().enumerate() {
        println!("# {}: {:?}.  {:?}", pidx, p, countour_point_to_coords(&offset, stats, p), );
    }
}

fn check_point_line(_points: &Vec<Point>) -> bool {
    /*
    if points.len() < 2 {
        return false;
    }

    if points[0] != points[points.len() - 1] {
        return false;
    }

    if has_non_consecutive_doubles(points) {
        return false;
    }*/

    return true;
}

#[cfg(test)]
mod cmd_contour_test {
    use super::*;

    #[test]
    fn test_find_doubles() {
        let a = find_doubles(&vec![
            (3.2, 4.5),
            (3.0, 4.5),
            (3.4, 4.5),
            (3.2, 4.5),
            (3.2, 4.5),
        ]);
        //3 doubles in a row
        assert_eq!(a, Some([3, 0]));

        //this case we have non consecutive doubles, which normally
        //shouldn't happen
        let y = 3.;
        let a = find_doubles(&vec![
            (3.2, y),
            (3.0, y),
            (3.2, y),
            (3.5, y),
            (3.4, y),
            (3.2, y),
        ]);
        assert_eq!(a, Some([0, 2]));

        let a = find_doubles(&vec![
            (1.0, 4.5),
            (2.0, 4.5),
            (2.0, 4.5),
            (2.0, 4.5),
            (3.0, 4.5),
            (4.0, 4.5),
            (1.0, 4.5),
        ]);

        assert_eq!(a, Some([1, 3]));

        let a = find_doubles(&vec![
            (1.0, 4.5),
            (1.0, 4.5),
            (5.0, 4.5),
            //Point{x: 4.0, y: 4.5},
            //Point{x: 3.0, y: 4.5},
            //Point{x: 3.0, y: 4.5},

            (1.0, 4.5),
            (1.0, 4.5),
            (1.0, 4.5),
        ]);

        assert_eq!(a, Some([3, 1]));

        //normal doubles that go reverse
        let y = 4.;
        let a = find_doubles(&vec![
            (1.0, y),
            (2.0, y),
            (3.0, y),
            (4.0, y),
            (5.0, y),
            (6.0, y),
            (2.0, y),
            (1.0, y),
        ]);
        assert_eq!(a, Some([6, 1]));
    }
}
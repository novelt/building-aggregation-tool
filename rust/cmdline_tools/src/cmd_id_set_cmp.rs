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
use std::collections::{HashMap, HashSet};

use std::path::PathBuf;
use std::time::Instant;
use gdal::spatial_ref::{SpatialRef};

use anyhow::{bail, Result};
use structopt::StructOpt;

use gdal::vector::{Driver};
use geo_util::util::{print_remaining_time, RasterChunkIterator};
use log::{info};
use geo_util::pq::PgConnection;
use geo_util::raster::{Raster, RasterStats};
use geos::{SimpleContextHandle, SimpleGeometry};

///
/// Groups polygons/multipolygons within horizonal/vertical distance of x meters together
#[derive(StructOpt)]
pub struct IdSetCmpArgs {

    #[structopt(long, parse(from_os_str))]
    pub (crate) year1_raster: PathBuf,

    #[structopt(long, parse(from_os_str))]
    pub (crate) year2_raster: PathBuf,

    #[structopt(long)]
    pg_conn_str: String,

    #[structopt(long)]
    schema: String,

    #[structopt(long)]
    y1_layer_name: String,

    #[structopt(long)]
    y1_ogr_conn_str: String,

    #[structopt(long)]
    y1_id_field: String,

    #[structopt(long)]
    y2_layer_name: String,

    #[structopt(long)]
    y2_ogr_conn_str: String,

    #[structopt(long)]
    y2_id_field: String,
}


pub(crate) fn id_set_cmp(args: &IdSetCmpArgs) -> Result<()> {

    
    let year1_raster = Raster::read(&args.year1_raster, true);
    let year2_raster = Raster::read(&args.year2_raster, true);

    year1_raster.stats.assert_equals_except_no_data(&year2_raster.stats);

    let pg_conn = PgConnection::new(&args.pg_conn_str)?;

    let stats = &year1_raster.stats;
    let lat_lon = SpatialRef::from_wkt(&stats.projection)?;

    let seen_y1_ids = get_ids_from_raster(&year1_raster)?;
    let seen_y2_ids = get_ids_from_raster(&year2_raster)?;

    let missing_y1_ids = get_missing_ids(&args.y1_id_field,
    &args.y1_layer_name, &args.y1_ogr_conn_str, &seen_y1_ids, stats)?;

    let missing_y2_ids = get_missing_ids(&args.y2_id_field,
    &args.y2_layer_name, &args.y2_ogr_conn_str, &seen_y2_ids, stats)?;

    info!("Number year 1 ids: {} Number ids not in raster: {}", seen_y1_ids.len(), missing_y1_ids.len());
    info!("Number year 2 ids: {} Number ids not in raster: {}", seen_y2_ids.len(), missing_y2_ids.len());

    assert_eq!(4326, lat_lon.auth_code()?);
    assert_eq!("EPSG", lat_lon.auth_name()?);

    let query = format!("
CREATE SCHEMA IF NOT EXISTS {SCHEMA};

DROP TABLE IF EXISTS {SCHEMA}.squares;

CREATE TABLE {SCHEMA}.squares
(
    raster_x int NOT NULL,
    raster_y int NOT NULL,
    center Geometry(Point, 4326) NOT NULL,
    year1_id int,
    year2_id int,
    PRIMARY KEY(raster_x, raster_y)
);


", SCHEMA = args.schema,

    );

    let r = pg_conn.execute(&query);
    r.is_ok().unwrap();

    let copy_sql = format!("
COPY {SCHEMA}.squares (
    raster_x, raster_y,
    center, year1_id,
    year2_id
    )
    FROM STDIN  BINARY
", SCHEMA = args.schema);


    let num_steps = stats.num_cols * stats.num_rows;
    let mut num_processed = 0;
    let mut last_output = Instant::now();
    let start = Instant::now();

    let number_of_chunks = 10;
    let context = SimpleContextHandle::new();

    for raster_window in RasterChunkIterator::<i32>::new(
        stats.num_rows as _,
        stats.num_cols as _,number_of_chunks as _)
    {
        pg_conn.copy_start(&copy_sql)?;
        //println!("Combining {:?} and {:?}", window_offset, window_size);

        let year1_data: Vec<i32> = year1_raster.band().read_as(
            raster_window.window_offset, raster_window.window_size
        )?;
        let year2_data: Vec<i32> = year2_raster.band().read_as(
            raster_window.window_offset, raster_window.window_size
        )?;

        for idx in 0..year1_data.len() {

            if last_output.elapsed().as_secs() >= 3 {
                last_output = Instant::now();
                print_remaining_time(&start,
                                     num_processed as u32,
                                     num_steps as _);
            }

            num_processed+=1;


            let idx_i32 = idx as i32;
            let offset_x = idx_i32 % raster_window.window_size.0;
            let offset_y = idx_i32 / raster_window.window_size.0;
            let raster_x = raster_window.window_offset.0 + offset_x;
            let raster_y = raster_window.window_offset.1 + offset_y;

            let year1_id = get_id(year1_data[idx],
                &missing_y1_ids,
                raster_x, raster_y
            );
            let year2_id = get_id(year2_data[idx],
                &missing_y2_ids,
                raster_x, raster_y
            );

            if year1_id.is_none() && year2_id.is_none() {
               continue;
            }

            let coord_xy = stats.calc_center((raster_x, raster_y));

            if Some(-1) == year1_id || Some(-1) == year2_id {
                bail!("Problem at {}, {} ; {}, {}",
                    raster_x, raster_y,
                    coord_xy[0], coord_xy[1],
                )
            }



            pg_conn.copy_field_count(5)?;
            pg_conn.copy_int(raster_x)?;
            pg_conn.copy_int(raster_y)?;

            let geom_point = SimpleGeometry::create_point_xy(&context,
                                                             coord_xy[0],
                                                             coord_xy[1]).unwrap();
            geom_point.set_srid(4326);

            let ewkb = geom_point.ewkb()?;
            pg_conn.copy_bytes(ewkb.as_ref())?;


            pg_conn.copy_opt_int(year1_id)?;
            pg_conn.copy_opt_int(year2_id)?;

        }

        pg_conn.copy_end()?.is_ok().unwrap();
    }

    let query = format!("

CREATE INDEX idx_year1_id ON {SCHEMA}.squares (year1_id) ;
CREATE INDEX idx_year2_id ON {SCHEMA}.squares (year2_id) ;
CREATE INDEX idx_center ON {SCHEMA}.squares USING GIST (center) ;

", SCHEMA = args.schema,

    );

    pg_conn.execute(&query).is_ok().unwrap();

    //Now we need to account for any ids that are not represented

    Ok(())
}

fn get_ids_from_raster(raster: &Raster) -> Result<HashSet<i32>>
{
    let number_of_chunks = 10;

    let stats = &raster.stats;

    let mut seen_ids: HashSet<i32> = HashSet::new();

    for raster_window in RasterChunkIterator::<i32>::new(
        stats.num_rows as _,
        stats.num_cols as _,number_of_chunks as _)
    {
        let id_data: Vec<i32> = raster.band().read_as(
            raster_window.window_offset, raster_window.window_size
        )?;

        for id in id_data {
            seen_ids.insert(id);
        }

    }

    Ok(seen_ids)
}


///Returns a map from raster_x, raster_y to an id
///This raster_x, raster_y is the centroid of the missing settlement
fn get_missing_ids(
    id_field: &str, layer_name: &str,
    conn_str: &str,
    // These are the ids that are contained in a raster
    seen_ids: &HashSet<i32>,
    stats: &RasterStats,
) -> Result<HashMap<(i32, i32), i32>>
{
    let input_dataset = Driver::open_vector_static(conn_str, true, &["VERIFY_BUFFERS=NO".to_string()]).unwrap();
    let input_layer = input_dataset.layer_by_name(layer_name).unwrap();

    let col_idx_id = input_layer.layer_definition().get_field_index(id_field);

    let mut ret = HashMap::new();

    for f in input_layer.features() {
        let id = if let Ok(ci) = col_idx_id {
            f.get_field_as_int(ci)
        } else {
            f.fid() as i32
        };

        if seen_ids.contains(&id) {
            continue;
        }

        //We need the raster square closest to the centroid
        let centroid = f.geometry().as_geom().centroid()?;

        let xy = centroid.get_point(0);

        let raster_x = stats.calc_x(xy[0]);
        let raster_y = stats.calc_y(xy[1]);

        ret.insert((raster_x, raster_y), id);
    }

    Ok(ret)
}

fn get_id(
    //Id from year1 or year2 raster
    id_from_data: i32,
    //Map from raster_x,raster_y representing centroid
    //to the missing id
    missing_id_map: &HashMap<(i32, i32), i32>,
    raster_x: i32,
    raster_y: i32
) -> Option<i32> {

    let is_nodata = id_from_data < 0;

    //The raster_x and raster_y are chosen by the raster square containing the centroid
    //of settlements who don't appear in the raster.  This takes priority over any
    //other id (which in rare cases can not be no data such as when 2 settlements are
    //very close, eg. raster center is in one settlement, but the centroid of another settlement is too
    let missing_id = missing_id_map.get(&(raster_x, raster_y));

    if let Some(v) = missing_id {
        return Some(*v);
    }

    if !is_nodata {
        return Some(id_from_data);
    }

    None

}
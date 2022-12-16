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
use std::path::PathBuf;
use std::time::Instant;
use gdal::spatial_ref::{CoordTransform, SpatialRef};

use anyhow::Result;
use rstar::{AABB, Envelope, PointDistance, RTree, RTreeObject};
//Counts points or centroids and outputs a raster
use structopt::StructOpt;

use gdal::vector::{Dataset, OGRwkbGeometryType};
use geo_util::util::{print_remaining_time, RasterChunkIterator};
use log::{debug};
use uuid::Uuid;
use geo_util::pq::PgConnection;
use geo_util::raster::{Raster, RasterStats};
use geos::{SimpleContextHandle, SimpleGeometry};
use geo::{Point as GeoPoint};
use geo::geodesic_distance::GeodesicDistance;

///
/// Groups polygons/multipolygons within horizonal/vertical distance of x meters together
#[derive(StructOpt)]
pub struct RaHfCsvArgs {

    #[structopt(long, parse(from_os_str))]
    pub (crate) pop_raster: PathBuf,

    #[structopt(long, parse(from_os_str))]
    pub (crate) hf: PathBuf,

    #[structopt(long, parse(from_os_str))]
    pub (crate) set_level_raster: PathBuf,

    #[structopt(long)]
    pg_conn_str: String,

    #[structopt(long)]
    schema: String,


}


pub(crate) fn ra_hf_csv(args: &RaHfCsvArgs) -> Result<()> {

    //debug!("Min height: {:.2} Max Height: {:.2}", min_height, max_height);

    let pop_raster = Raster::read(&args.pop_raster, true);
    let set_level_raster = Raster::read(&args.set_level_raster, true);

    assert!(pop_raster.stats.is_aligned(&set_level_raster.stats));

    set_level_raster.stats.assert_equals_except_no_data(&pop_raster.stats);

    //info!("Ref raster: {} Pop raster: {} Offsets: {:?}", &ref_raster.stats, &pop_raster.stats, &offsets);

    //create a RTree for the health facilites
    let pg_conn = PgConnection::new(&args.pg_conn_str)?;

    let stats = &pop_raster.stats;
    let lat_lon = SpatialRef::from_wkt(&stats.projection)?;

    assert_eq!(4326, lat_lon.auth_code()?);
    assert_eq!("EPSG", lat_lon.auth_name()?);
    let meters_proj = get_laea_spatial_ref(stats)?;

    let hf_rtree = serialize_hf_rtree(args, &meters_proj)?;

    let query = format!("
CREATE SCHEMA IF NOT EXISTS {SCHEMA};

DROP TABLE IF EXISTS {SCHEMA}.squares;

CREATE TABLE {SCHEMA}.squares
(
    square_id serial PRIMARY KEY,
    raster_x int NOT NULL,
    raster_y int NOT NULL,
    center Geometry(Point, 4326) NOT NULL,
    pop double precision NULL,
    set_level smallint NULL,
    nearest_dist double precision NOT NULL,
    hf_id int NOT NULL REFERENCES {SCHEMA}.hf (hf_id)

);


", SCHEMA = args.schema,

    );

    pg_conn.execute(&query).is_ok().unwrap();

    let copy_sql = format!("
COPY {SCHEMA}.squares (
    raster_x, raster_y,
    center, pop,
    set_level,
    nearest_dist, hf_id
    )
    FROM STDIN  BINARY
", SCHEMA = args.schema);


    let num_steps = stats.num_cols * stats.num_rows;
    let mut num_processed = 0;
    let mut last_output = Instant::now();
    let start = Instant::now();

    let number_of_chunks = 10;
    let context = SimpleContextHandle::new();

    let x_form = CoordTransform::new(&lat_lon, &meters_proj)?;

    for raster_window in RasterChunkIterator::<i32>::new( pop_raster.stats.num_rows as _,
                                                                              pop_raster.stats.num_cols as _,number_of_chunks as _)
    {
        pg_conn.copy_start(&copy_sql)?;
        //println!("Combining {:?} and {:?}", window_offset, window_size);

        let pop_data: Vec<f64> = pop_raster.band().read_as(
            raster_window.window_offset, raster_window.window_size
        )?;
        let set_level_data: Vec<i16> = set_level_raster.band().read_as(
            raster_window.window_offset, raster_window.window_size
        )?;

        for idx in 0..pop_data.len() {

            if last_output.elapsed().as_secs() >= 3 {
                last_output = Instant::now();
                print_remaining_time(&start,
                                     num_processed as u32,
                                     num_steps as _);
            }

            num_processed+=1;
            let pop = pop_data[idx];
            let sl = set_level_data[idx];

            let pop_is_nodata = pop_raster.stats.is_nodata(pop);
            let sl_is_nodata = sl < 0;
            if sl_is_nodata && pop_is_nodata  {
               continue;
            }

            let idx_i32 = idx as i32;
            let offset_x = idx_i32 % raster_window.window_size.0;
            let offset_y = idx_i32 / raster_window.window_size.0;
            let raster_x = raster_window.window_offset.0 + offset_x;
            let raster_y = raster_window.window_offset.1 + offset_y;
            let coord_xy = stats.calc_center((raster_x, raster_y));

            let coord_xy_meters = x_form.transform_point(&coord_xy)?;

            let coord_point: GeoPoint<f64> = coord_xy.into();

            let nn = hf_rtree.nearest_neighbor(&coord_xy_meters).unwrap();


            pg_conn.copy_field_count(7)?;
            pg_conn.copy_int(raster_x)?;
            pg_conn.copy_int(raster_y)?;

            let geom_point = SimpleGeometry::create_point_xy(&context,
                                                             coord_xy[0],
                                                             coord_xy[1]).unwrap();
            geom_point.set_srid(4326);

            let ewkb = geom_point.ewkb()?;
            pg_conn.copy_bytes(ewkb.as_ref())?;

            if pop_is_nodata {
                pg_conn.copy_null()?;
            } else {
                pg_conn.copy_f64(pop)?;
            }

            if sl_is_nodata {
                pg_conn.copy_null()?;
            } else {
                pg_conn.copy_smallint(sl)?;
            }

            let dist = nn.geo_point.geodesic_distance(&coord_point);
            pg_conn.copy_f64(dist)?;
            pg_conn.copy_int(nn.fidx as _)?;

        }

        pg_conn.copy_end()?.is_ok().unwrap();
    }

    let query = format!("

CREATE INDEX idx_raster_x ON {SCHEMA}.squares (raster_x) ;
CREATE INDEX idx_raster_y ON {SCHEMA}.squares (raster_y) ;
CREATE INDEX idx_center ON {SCHEMA}.squares USING GIST (center) ;
CREATE INDEX idx_nearest_dist ON {SCHEMA}.squares (nearest_dist) ;

", SCHEMA = args.schema,

    );

    pg_conn.execute(&query).is_ok().unwrap();

    Ok(())
}

pub fn serialize_hf_rtree(args: &RaHfCsvArgs, meters_proj: &SpatialRef) -> Result<RTree<RTreeIndexObjectHf>> {
    let pg_conn = PgConnection::new(&args.pg_conn_str)?;

    let query = format!("
CREATE SCHEMA IF NOT EXISTS {SCHEMA};

DROP TABLE IF EXISTS {SCHEMA}.hf CASCADE;

CREATE TABLE {SCHEMA}.hf
(
    hf_id int PRIMARY KEY,
    global_id uuid NOT NULL UNIQUE,
    name text,
    center Geometry(Point, 4326) NOT NULL,
    proj_x double precision,
    proj_y double precision
);


", SCHEMA = args.schema,

    );

    let r = pg_conn.execute(&query);

    r.is_ok()?;

    let copy_sql = format!("
COPY {SCHEMA}.hf (hf_id, global_id, name, center, proj_x, proj_y)
    FROM STDIN  BINARY
", SCHEMA = args.schema);

    pg_conn.copy_start(&copy_sql)?;

    let mut last_output = Instant::now();

    let context = SimpleContextHandle::new();

    let mut rio_list = Vec::new();

    debug!("Reading input set");

    let input_ds = Dataset::open(args.hf.to_str().unwrap())?;
    let input_lyr = input_ds.layer(0)?;
    let input_lyr_def = input_lyr.layer_definition();

    let num_steps = input_lyr.count(false);
    let mut num_processed = 0;
    let start = Instant::now();

    let name_field_idx = input_lyr_def.get_field_index("name")?;
    let global_id_field_idx = input_lyr_def.get_field_index("global_id")?;

    let lat_lon = input_lyr.spatial_reference()?;
    assert_eq!(4326, lat_lon.auth_code()?);
    assert_eq!("EPSG", lat_lon.auth_name()?);

    let x_form = CoordTransform::new(&lat_lon, meters_proj)?;

    for (fidx, in_feature) in input_lyr.features().enumerate() {

        let fidx = fidx as u32;
        num_processed += 1;

        let g = in_feature.geometry().as_geom();
        assert_eq!(g.geometry_type(), OGRwkbGeometryType::wkbPoint);

        let lat_lon_pt = g.get_point(0);

        let meters_pt = x_form.transform_point(&lat_lon_pt )?;
        let envelope = AABB::from_corners(meters_pt, meters_pt);

        let name = in_feature.get_field_as_string(name_field_idx);
        let global_id_str = in_feature.get_field_as_string(global_id_field_idx);
        let global_id = Uuid::parse_str(&global_id_str)?;

        pg_conn.copy_field_count(6)?;
        pg_conn.copy_int(fidx as _)?;
        pg_conn.copy_bytes(global_id.as_bytes())?;
        pg_conn.copy_str(&name)?;

        let geom_point = SimpleGeometry::create_point_xy(&context,
                                                             lat_lon_pt[0],
                                                             lat_lon_pt[1]).unwrap();
        geom_point.set_srid(4326);

        let ewkb = geom_point.ewkb()?;
        pg_conn.copy_bytes(ewkb.as_ref())?;

        pg_conn.copy_f64(meters_pt[0] )?;
        pg_conn.copy_f64(meters_pt[1] )?;

        let rio = RTreeIndexObjectHf {
            fidx,
            envelope,
            geo_point: lat_lon_pt.into()
        };
        rio_list.push(rio);

        if last_output.elapsed().as_secs() >= 3 {
            last_output = Instant::now();
            print_remaining_time(&start,
                                 num_processed as u32,
                                 num_steps as _);
        }
    }

    let copy_result = pg_conn.copy_end()?;
    debug!("Result: {} {}", copy_result.status_str(), copy_result.error_message());

    copy_result.is_ok().unwrap();

    let rtree: RTree<RTreeIndexObjectHf> = RTree::bulk_load(rio_list);


    Ok(rtree)

}

fn get_laea_spatial_ref(stats: &RasterStats) -> Result<SpatialRef> {

    let center_x = (stats.right_x_coord() + stats.origin_x) / 2.0;
    let center_y = (stats.origin_y + stats.bottom_y_coord()) / 2.0;

    let laea = SpatialRef::from_proj4(&format!(
        "+proj=laea +lat_0={} +lon_0={} +ellps=GRS80 +towgs84=0,0,0,0,0,0,0 +units=m +no_defs",
        center_y, center_x
    ))?;

    Ok(laea)
}

pub type Coord = f64;


#[derive(Clone)]
pub struct RTreeIndexObjectHf {
    pub fidx: u32,
    pub geo_point: GeoPoint<f64>,
    pub envelope: AABB<[Coord; 2]>,
}

/// Implement this to support nearest neighbor calculations
impl PointDistance for RTreeIndexObjectHf {
    /// For speed, use the distance of the center of the envelope to the point
    fn distance_2(
        &self,
        rhs: &[Coord; 2]) -> Coord {
        let center = self.envelope.center();

        // Vector distance in lat/lon
        return center.distance_2(rhs);
    }

    // This implementation is not required but more efficient since it
    // omits the calculation of a square root
    fn contains_point(&self, point: &[Coord; 2]) -> bool
    {
        self.envelope.contains_point(point)
    }
}

/// Rstar requires this implementation to know how to index it
impl RTreeObject for RTreeIndexObjectHf {
    type Envelope = AABB<[Coord; 2]>;

    fn envelope(&self) -> Self::Envelope {
        self.envelope
    }
}

impl PartialEq for RTreeIndexObjectHf {
    fn eq(&self, other: &Self) -> bool {
        self.fidx == other.fidx
    }
}

impl Eq for RTreeIndexObjectHf {}

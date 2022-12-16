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
use std::collections::HashSet;
use std::fs::File;
use std::io::BufWriter;
use std::path::PathBuf;
use std::time::Instant;
use anyhow::{Result};
use csv::WriterBuilder;
use itertools::Itertools;
use gdal::vector::{Driver, OGRwkbGeometryType};
use structopt::StructOpt;
use rstar::{AABB, PointDistance, Envelope, RTreeObject, RTree};
//use serde::{Deserialize, Serialize};
use geo_util::convert::convert_from_gdal_to_geos;
use log::{debug, info};

use geo_util::raster::{Raster};
use geo_util::util::print_remaining_time_msg;
use geos::{PreparedGeometry, SimpleContextHandle, SimpleGeometry};
use crate::raster_center_dto::{RasterCenterDto, Corners};

#[derive(StructOpt)]
pub struct CheckRasterCenterArgs {
    #[structopt(long, parse(from_os_str), help="Split Settlement FGB Path")]
    pub(crate) settlement_fgb: Vec<PathBuf>,

    #[structopt(long, parse(from_os_str), help="Building Centroid FGB Path")]
    pub(crate) bldg_centroid_fgb: PathBuf,

    #[structopt(long, parse(from_os_str), help="CSV output")]
    pub(crate) out_csv: PathBuf,

    #[structopt(long, parse(from_os_str), help="Binary output")]
    pub(crate) out_bin: PathBuf,

    #[structopt(long, parse(from_os_str), help="Building count raster")]
    pub(crate) bldg_count_raster: PathBuf,

}

pub fn check_raster_center(args: &CheckRasterCenterArgs) -> Result<()>
{
    info!("Starting check_raster_center...");

    //
    // # Create a CSV with settlement fid, raster x, raster y, 4326 coords, corner
    // We need to check the buildings because a settlement can intersect a raster square
    // without it actually having a building in it

    let fgb_driver = Driver::get(Driver::DRIVER_NAME_FLATGEOBUF)?;

    let ds_bc = fgb_driver.open(args.bldg_centroid_fgb.to_str().unwrap(), true ).unwrap();
    let lyr_bc = ds_bc.layer(0)?;

    let bldg_count_raster = Raster::read(&args.bldg_count_raster, true);
    let stats = &bldg_count_raster.stats;
    let band = bldg_count_raster.band();

    let mut building_count_data = vec![-1i32; 1];

    debug!("Creating csv at {:?}", &args.out_csv);
    //let csv_f = File::create(&args.out_csv).expect("Unable to create file");
    //let mut csv_f = Writer::new(csv_f);
    let mut csv_f = WriterBuilder::new()
        .has_headers(true)
        .from_path(&args.out_csv)?;

    //csv_f.write("settlement fid,settlement_level,raster x,raster y,longitude,latitude,grid_index,corner\n".as_bytes())?;

    let now  = Instant::now();
    let total_to_process = lyr_bc.count(false);
    let mut last_output = Instant::now();

    /*
    create building centroid layers

    for each building centroid

    find settlement slice that matches that
    find raster square

    check if that raster square center is in any of the settlement slices of that settlement

     */

    //Build an rtree of the settlements
    let simple_context = SimpleContextHandle::new();
    debug!("Building rtree");
    let rtree_settlements = build_settlement_rtree(args, &simple_context)?;

    let mut seen = HashSet::new();

    let mut records = Vec::new();

    debug!("Looping through buildings");
    for (f_idx, f) in lyr_bc.features().enumerate() {

        if last_output.elapsed().as_secs() >= 3 {
            last_output = Instant::now();
            print_remaining_time_msg(&now, f_idx as _, total_to_process as u32,
                ""
            );
        }

        let g = f.geometry().as_geom();
        assert_eq!(g.geometry_type(), OGRwkbGeometryType::wkbPoint);

        let bldg_centroid = g.get_point(0);

        //Find which settlement
        let inter_settlements = rtree_settlements.locate_all_at_point(&bldg_centroid).collect_vec();

        if inter_settlements.len() < 1 {
            println!("Cannot find settlement at {} {}", bldg_centroid[0], bldg_centroid[1]);
        }

        assert!(inter_settlements.len() > 0);

        //Can have 2 different settlements in same grid square
        let settlement_slice = if inter_settlements.len() > 1 {
            let sg = SimpleGeometry::create_point_xy(&simple_context, bldg_centroid[0], bldg_centroid[1])?;
            let mut found_idx = inter_settlements.len();
            for (idx, is) in inter_settlements.iter().enumerate() {
                if is.prepared_geom.intersects(&sg)? {
                    found_idx = idx;
                    break;
                }
            }

            assert_ne!(found_idx, inter_settlements.len());
            inter_settlements[found_idx]
        } else {
            inter_settlements[0]
        };


        //check if we have done this square + settlement
        let raster_x = stats.calc_x(bldg_centroid[0]);
        let raster_y = stats.calc_y(bldg_centroid[1]);
        let raster_index = raster_x + raster_y * stats.num_cols as i32;

        assert!(raster_x >= 0 && raster_x < stats.num_cols as i32);
        assert!(raster_y >= 0 && raster_y < stats.num_rows as i32);

        //now check, do we intersect the center?
        let seen_key = (raster_index, settlement_slice.fid, settlement_slice.settlement_level);

        //insert will return true if we have not seen the key yet
        let already_seen = !seen.insert(seen_key);

        if already_seen {
            continue;
        }

        //Sanity check...
        band.read_into_vec((raster_x, raster_y),(1,1), &mut building_count_data).unwrap();
        assert!(building_count_data[0] > 0);

        let center = stats.calc_center((raster_x, raster_y));
        let center_point = SimpleGeometry::create_point_xy(&simple_context, center[0], center[1])?;

        debug!("Intersecting with {} {}", center[0], center[1]);
        let intersects = settlement_slice.prepared_geom.intersects(&center_point)?;
        debug!("Intersecting with {} {}, result: {}", center[0], center[1], intersects);

        if intersects {
            continue;
        }

        //Now we need to write the csv
        // let settlement_fid = f.get_field_as_int(orid_fid_col_index);

        let square_x_min = stats.calc_x_coord(raster_x);
        let square_x_max = stats.calc_x_coord(raster_x+1);
        let square_y_min = stats.calc_y_coord(raster_y+1);
        let square_y_max = stats.calc_y_coord(raster_y);

        assert!(square_y_max > square_y_min);

        // corners
        // 3 0
        // 2 1

        //Find settlement slice centroid, see which corner its closest to
        let settlement_centroid = settlement_slice.geometry.centroid()?.get_xy()?;

        assert!(settlement_centroid.0 >= square_x_min);
        assert!(settlement_centroid.0 <= square_x_max);
        assert!(settlement_centroid.1 >= square_y_min);
        assert!(settlement_centroid.1 <= square_y_max);

        let is_right = (square_x_max - settlement_centroid.0) < (settlement_centroid.0 - square_x_min);
        let is_top = (square_y_max - settlement_centroid.1) < (settlement_centroid.1 - square_y_min);

        let corner = if is_right && is_top {
            Corners::NorthEast
        } else if is_right && !is_top {
            Corners::SouthEast
        } else if !is_right && !is_top {
            Corners::SouthWest
        } else {
            Corners::NorthWest
        };

        let record = RasterCenterDto {
            settlement_fid: settlement_slice.fid,
            settlement_level:  settlement_slice.settlement_level,
            raster_x,
            raster_y,
            long: center[0],
            lat: center[1],
            grid_index: raster_index,
            corner
        };

        csv_f.serialize(&record)?;
        records.push(record);
    }

    let mut f = BufWriter::new(File::create(&args.out_bin).unwrap());
    bincode::serialize_into(&mut f, &records).unwrap();

    Ok(())
}

fn build_settlement_rtree<'a>(args: &CheckRasterCenterArgs, context_handle: &'a SimpleContextHandle) -> Result<RTree<RTreeIndexObject<'a>>> {
    let fgb_driver = Driver::get(Driver::DRIVER_NAME_FLATGEOBUF)?;

    let mut rio_list = Vec::new();

    for (settlement_level, settlement_fgb) in args.settlement_fgb.iter().enumerate()
    {
        let settpath_fgb_str = settlement_fgb.to_str().unwrap();
        let ds_settlements = fgb_driver.open(settpath_fgb_str, true).expect(settpath_fgb_str);
        let lyr_settlements = ds_settlements.layer(0)?;

        debug!("Adding {} settlements to rtree from level {}", lyr_settlements.count(false), settlement_level);

        let orid_fid_col_index = lyr_settlements.layer_definition().get_field_index("orig_fid").unwrap();

        for feature in lyr_settlements.features() {
            let geos_geom = convert_from_gdal_to_geos(&feature.geometry().as_geom(),
                                                      &context_handle, false
            )?;

            let bbox = geos_geom.envelope()?.bbox()?;
            let envelope_aabb = AABB::from_corners([bbox[0], bbox[1]], [bbox[2], bbox[3]]);

            let prepared_geom = PreparedGeometry::new(&geos_geom).unwrap();

            let fid = feature.get_field_as_int(orid_fid_col_index);

            let rio = RTreeIndexObject {
                fid,
                settlement_level: settlement_level as u8,
                envelope: envelope_aabb,
                geometry: geos_geom,
                prepared_geom
            };
            rio_list.push(rio);
        }
    }

    let rtree: RTree<RTreeIndexObject>  = RTree::bulk_load(rio_list);

    debug!("Settlement rtree count: {}", rtree.size());

    Ok(rtree)
}

pub type Coord  = f64;

//#[derive(Deserialize, Serialize)]
pub struct RTreeIndexObject<'a> {
    pub fid: i32,
    pub settlement_level: u8,
    pub envelope: AABB<[Coord; 2]>,
    pub geometry: SimpleGeometry<'a>,
    pub prepared_geom: PreparedGeometry<'a>
}

/// Implement this to support nearest neighbor calculations
impl <'a> PointDistance for RTreeIndexObject<'a> {
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
impl <'a> RTreeObject for RTreeIndexObject<'a> {
    type Envelope = AABB<[Coord; 2]>;

    fn envelope(&self) -> Self::Envelope {
        self.envelope
    }
}

impl <'a> PartialEq for RTreeIndexObject<'a> {
  fn eq(&self, other: &Self) -> bool {
      self.fid == other.fid && self.settlement_level == other.settlement_level
  }
}
impl <'a> Eq for RTreeIndexObject<'a> {}
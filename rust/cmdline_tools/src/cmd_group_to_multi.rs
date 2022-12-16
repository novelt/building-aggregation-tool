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
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::time::Instant;
use gdal::spatial_ref::{OSRAxisMappingStrategy, SpatialRef};

use anyhow::Result;
use partitions::PartitionVec;
use rstar::{AABB, Envelope, PointDistance, RTree, RTreeObject};
use serde::{Deserialize, Serialize};
//Counts points or centroids and outputs a raster
use structopt::StructOpt;

use gdal::vector::{Dataset, Driver, Feature, FieldDefinition, Geometry, OGRFieldType, OGRwkbGeometryType};
use geo_util::io::get_sub_dir;
use geo_util::util::print_remaining_time;
use log::{debug};

///
/// Groups polygons/multipolygons within horizonal/vertical distance of x meters together
#[derive(StructOpt)]
pub struct GroupToMultiArgs {
    #[structopt(long, help = "OGR Connection string for inputs")]
    pub(crate) in_ogr_conn: Vec<String>,

    #[structopt(long, help = "Layer names for input, use - to use default, and all for everything")]
    pub(crate) in_ogr_layer: Vec<String>,

    #[structopt(long)]
    pub(crate) out_ogr_conn: String,

    #[structopt(long)]
    pub(crate) out_ogr_layer: String,

    #[structopt(long)]
    pub(crate) width: f64,

    #[structopt(long, parse(from_os_str))]
    pub (crate) temp_dir: PathBuf,

}

pub(crate) fn group_to_multi(args: &GroupToMultiArgs) -> Result<()> {

    //debug!("Min height: {:.2} Max Height: {:.2}", min_height, max_height);



    let mut last_output = Instant::now();


    //Loop through input, creating rtree of the extents

    //Go through partition vec extents, create mp for each



    let mut pvec = PartitionVec::with_capacity(70_000_000 );
    let mut rio_list = Vec::new();

    //Also serialize
    let dat_file_path = get_sub_dir(&args.temp_dir,
                                        "group_to_multi.dat");

    let mut dat_file = BufWriter::new(File::create(&dat_file_path)?);
    let mut dat_file_bytes_written = 0;

    let mut offset_length: Vec<(u64,usize)> = Vec::new();

    debug!("Reading input set");

    let mut fidx = 0;


    for input_idx in 0..args.in_ogr_conn.len() {
        let input_ds = Dataset::open(&args.in_ogr_conn[input_idx])?;
        let input_lyr = input_ds.layer_by_name(&args.in_ogr_layer[input_idx])?;

        let num_steps = input_lyr.count(false);
        let mut num_processed = 0;
        let start = Instant::now();

        for in_feature in input_lyr.features() {

            num_processed += 1;

            let g = in_feature.geometry().as_geom();
            let env = g.envelope();
            let envelope = AABB::from_corners([env.MinX, env.MinY], [env.MaxX, env.MaxY]);

            let rio = RTreeIndexObject {
                fidx,
                envelope,
            };
            rio_list.push(rio);

            pvec.push(fidx);

            let geom_bytes = g.ewkb_bytes_raw()?;
            offset_length.push((dat_file_bytes_written, geom_bytes.len()));
            dat_file_bytes_written += geom_bytes.len() as u64;
            let bytes_written = dat_file.write(&geom_bytes)?;
            assert_eq!(bytes_written, geom_bytes.len());

            fidx += 1;

            if last_output.elapsed().as_secs() >= 3 {
                last_output = Instant::now();
                print_remaining_time(&start,
                                     num_processed as u32,
                                     num_steps as _);
            }
        }
    }
    dat_file.flush().unwrap();

    let num_steps = rio_list.len();

    let rtree: RTree<RTreeIndexObject> = RTree::bulk_load(rio_list);

    debug!("RTree processing");

    let mut num_processed = 0;
    let start = Instant::now();

    for f in rtree.iter() {
        num_processed += 1;
        let query_envelope = AABB::from_corners(
            [f.envelope.lower()[0] - args.width, f.envelope.lower()[1] - args.width],
            [f.envelope.upper()[0] + args.width, f.envelope.upper()[1] + args.width],
        );
        for r_int in rtree.locate_in_envelope_intersecting(&query_envelope) {
            pvec.union(f.fidx as _, r_int.fidx as _);
        }

        if last_output.elapsed().as_secs() >= 3 {
            last_output = Instant::now();
            print_remaining_time(&start,
                                 num_processed as u32,
                                 num_steps as _);
        }
    }


    let mut num_processed = 0;
    let start = Instant::now();

    let drv = Driver::get(Driver::DRIVER_NAME_FLATGEOBUF)?;
    let ds = drv.create(&args.out_ogr_conn)?;

    let mut spatial_ref = SpatialRef::from_epsg(4326)?;
    spatial_ref.set_axis_mapping_strategy(OSRAxisMappingStrategy::OAMS_TRADITIONAL_GIS_ORDER);

    let mut output_lyr = ds.create_layer_ext::<String>(
        &args.out_ogr_layer,
        &spatial_ref,
        OGRwkbGeometryType::wkbMultiPolygon,
        &vec![
            "SPATIAL_INDEX=NO".to_string()
        ],
    )?;

    let field_defn = FieldDefinition::new("poly_count", OGRFieldType::OFTInteger)?;
    field_defn.add_to_layer(&mut output_lyr)?;

    let field_defn = FieldDefinition::new("id", OGRFieldType::OFTInteger)?;
    field_defn.add_to_layer(&mut output_lyr)?;

    let num_steps = rtree.size();
    let output_layer_def = output_lyr.layer_definition();

    let mut dat_file = BufReader::new(File::open(&dat_file_path)?);

    for (id_idx, set) in pvec.all_sets().enumerate() {

        let mut poly_list = Vec::new();

        for (index, _v) in set {
            num_processed += 1;

            //maybe use serialized version
            //let input_feature = input_lyr.get_feature_by_id(value)?;

            //debug!("Dealing with index {} fid {}", index, fid);
            let off_len = &offset_length[index];

            //debug!("offset {} bytes {}", off_len.0, off_len.1);

            //3 192 576
            //3 191 112 bytes 182

            dat_file.seek(SeekFrom::Start(off_len.0)).unwrap();

            let mut buf = vec![0u8; off_len.1];
            dat_file.read_exact(&mut buf)?;

            let mut gdal_geom = Geometry::empty(OGRwkbGeometryType::wkbMultiPolygon)?;

            // debug!("Importing bytes {}", buf.len());
            gdal_geom.import_ewkb_bytes_raw(&buf).unwrap();
            //debug!("Done Importing bytes {}", buf.len());

            let poly_count = gdal_geom.geometry_count();
            //debug!("Polygon count {}", poly_count);
            for p in 0..poly_count {
                let poly = gdal_geom.get_geometry(p);
                poly_list.push(poly.clone());
            }
            //debug!("Polygon count {} done", poly_count);

            if last_output.elapsed().as_secs() >= 3 {
                last_output = Instant::now();
                print_remaining_time(&start,
                                     num_processed as u32,
                                     num_steps as _);
            }

        }

        let mut ft = Feature::new(&output_layer_def)?;

        if poly_list.len() > 100_000 {
            debug!("Polygon count: {}", poly_list.len());
        }
        ft.set_field_integer_by_index(0, poly_list.len() as _)?;
        ft.set_field_integer_by_index(1, id_idx as _)?;

        let mut grouped_geom = Geometry::empty(OGRwkbGeometryType::wkbMultiPolygon)?;
        //debug!("Building geometry");
        for polygon in poly_list {
            grouped_geom.add_geometry(polygon)?;
        }


        // It is extremely slow to check validity
        // The polygons are valid normally ,but there can be internal intersection
        // let input_is_valid = grouped_geom.is_valid();
        //
        // // Next make sure the geometry is valid
        // if !input_is_valid {
        //     debug!("Non valid geometry found");
        //
        //     let valid_grouped_geom = grouped_geom.make_valid();
        //     ft.set_geometry(valid_grouped_geom)?;
        // } else {
            //debug!("Setting geometry");
        ft.set_geometry(grouped_geom)?;
        //}
        ft.create(&output_lyr)?;
    }


    Ok(())
}


pub type Coord = f64;

#[derive(Deserialize, Serialize, Clone)]
pub struct RTreeIndexObject {
    pub fidx: u32,
    pub envelope: AABB<[Coord; 2]>,
}

/// Implement this to support nearest neighbor calculations
impl PointDistance for RTreeIndexObject {
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
impl RTreeObject for RTreeIndexObject {
    type Envelope = AABB<[Coord; 2]>;

    fn envelope(&self) -> Self::Envelope {
        self.envelope
    }
}

impl PartialEq for RTreeIndexObject {
    fn eq(&self, other: &Self) -> bool {
        self.fidx == other.fidx
    }
}

impl Eq for RTreeIndexObject {}


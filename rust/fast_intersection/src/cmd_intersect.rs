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
use std::collections::HashMap;
use std::fs::{create_dir_all, File};
use std::io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::time::Instant;
use anyhow::{bail, Result};
use structopt::StructOpt;
use log::{debug, info, trace};
use itertools::Itertools;
use rstar::{AABB, Envelope, PointDistance, RTree, RTreeObject};
use geo_util::io::{get_index_width_len, get_sub_dir};
use serde::{Deserialize, Serialize};
use gdal::spatial_ref::{OSRAxisMappingStrategy, SpatialRef};
use gdal::vector::{Driver, Feature, FieldDefinition, OGRFieldType, OGRwkbGeometryType};
use geo_util::convert::{convert_from_gdal_to_geos, convert_geos_to_gdal};
use geo_util::util::print_remaining_time;
use geos::{PreparedGeometry, SimpleContextHandle, SimpleGeometry, WKBReader};


#[derive(StructOpt)]
pub struct IntersectArgs {
    #[structopt(long, help = "Building FGB")]
    in_path: PathBuf,

    #[structopt(long, help = "Chunk number")]
    in_chunk_num: isize,

    #[structopt( long, )]
    pub chunk_rows: isize,

    #[structopt( long, )]
    pub chunk_cols: isize,

    #[structopt(parse(from_os_str), long, help = "Output building FGB")]
    output_path: PathBuf,

    #[structopt(long, help = "Which field to copy for intersected chunks")]
    id_field: Vec<String>,

    #[structopt(long, help = "Which field to set if there is an intersection")]
    out_field: Vec<String>,

    // #[structopt(long="level", help = "If intersects, what settlement level to put")]
    // level_field_value: Vec<u8>,

    #[structopt(long, parse(from_os_str), help = "Directory containing chunks")]
    int_chunk_dir: Vec<PathBuf>,

    #[structopt(long, parse(from_os_str), help = "Saving in geometries")]
    common_work_path: PathBuf,

    #[structopt(long, help = "Initial serialization?")]
    mode_int_prep: bool,
}

// offsets and orig fids
const FILE_NAME_GEOM_META_DATA: &str = "geom_meta_data.dat";
const FILE_NAME_GEOM_GEOS_DATA: &str = "geom_geos_data.dat";
//const FILE_NAME_RTREE_DATA: &str = "rtree.dat";

/// Takes a building fgb that has been chunked and intersects it
/// with N number of other chunked geometry and copies the intersected id
/// Will take the 1st one it intersects (so assumes settlements don't intersect)
pub fn intersect(args: &IntersectArgs) -> Result<()> {

    //Get the surrounding chunk rows/cols/chunk index
    let surrounding_chunk_row_col = get_surrounding_chunk_row_col(args).unwrap();

    debug!("Surrounding:\n{}", surrounding_chunk_row_col.iter().map(|src| format!("Row:  {} Col: {} Index: {}", src.0, src.1, src.2)).join(",\n"));

    //Serialize the input geometries
    if args.mode_int_prep {
        serialize_intersection_data(args)?;
        return Ok(());
    }

    //For each intersection settlement, create a prepared geometry and then intersect it with the buildings
    //that are within its extent

    //So...we need a in memory building rtree
    //a list of building geos geometries

    let simple_context = SimpleContextHandle::new();

    let (building_rtree, building_geos) = read_input_data(args, &simple_context).unwrap();

    //Now we want to fill a vec of the buildings with 0 or 1 intersected ids
    let mut building_intersections = Vec::with_capacity(building_geos.len());

    for _ in 0..building_geos.len() {
        //initialize to how many intersections we are doing
        building_intersections.push(vec![-1; args.int_chunk_dir.len()]);
    }

    //let layer_index_padding = get_index_width_len((args.chunk_cols * args.chunk_rows) as _);

    let geos_reader = WKBReader::new(&simple_context).unwrap();

    //Now go through each input
    for input_index in 0..args.int_chunk_dir.len() {

        for (chunk_row, chunk_col, chunk_index) in surrounding_chunk_row_col.iter() {
            let work_base_path = get_sub_dir(&args.common_work_path,
                                             format!("{}_{}", input_index, chunk_index));

            //load rtree and the offsets
            // let rtree: RTree<RTreeIndexObject> = {
            //     let rtree_path = get_sub_dir(&work_base_path, FILE_NAME_RTREE_DATA);
            //     let mut reader = BufReader::new(File::open(rtree_path)?);
            //     bincode::deserialize_from(&mut reader)?
            // };

            let offsets_and_orig_fids: Vec< (u64, i32) > = {
                let byte_offset_path = get_sub_dir(&work_base_path, FILE_NAME_GEOM_META_DATA);
                debug!("Opening {:?}", &byte_offset_path );
                let mut reader = BufReader::new(File::open(byte_offset_path).unwrap());
                bincode::deserialize_from(&mut reader).unwrap()
            };

            let geom_dat_file_path = get_sub_dir(&work_base_path, FILE_NAME_GEOM_GEOS_DATA);
            let mut geom_dat_file = BufReader::new(File::open(&geom_dat_file_path).unwrap());

            debug!("Loop through each input file of input # {} for chunk {} ; chunk row {}, chunk col {} Opening geom in {:?}",
                input_index, chunk_index, chunk_row, chunk_col,
                &geom_dat_file_path
                );

            //last one is just the last offset == total size of file
            for int_feature_index in 0..offsets_and_orig_fids.len()-1 {

                trace!("Processing input index {}, feature # {}, offset: {}, orig_fid: {}",
                    input_index,
                    int_feature_index,
                    offsets_and_orig_fids[int_feature_index].0,
                    offsets_and_orig_fids[int_feature_index].1
                );

                geom_dat_file.seek(SeekFrom::Start(offsets_and_orig_fids[int_feature_index].0)).unwrap();

                let num_bytes_to_read = offsets_and_orig_fids[int_feature_index+1].0 - offsets_and_orig_fids[int_feature_index].0 ;

                let mut buf = vec![0u8; num_bytes_to_read as usize];
                geom_dat_file.read_exact(&mut buf).unwrap();

                let geos_geom = geos_reader.read_wkb(&buf).unwrap();

                let prepared_geom = PreparedGeometry::new(&geos_geom).unwrap();

                //Now we consider all buildings (the inputs) in the extent of the feature to intersect

                let bbox = geos_geom.envelope()?.bbox()?;
                let envelope_aabb = AABB::from_corners( [bbox[0], bbox[1]], [bbox[2], bbox[3]] );

                for b in building_rtree.locate_in_envelope_intersecting(&envelope_aabb) {
                    let building_geos_shape = &building_geos[b.f_idx as usize];
                    //do we actually intersect?
                    if prepared_geom.intersects(building_geos_shape).unwrap() {
                        building_intersections[b.f_idx as usize][input_index] = offsets_and_orig_fids[int_feature_index].1
                    }
                }
            }
        }
    }


    //Now we write the output
    write_output(args, building_geos, building_intersections).unwrap();

    Ok(())
}

fn write_output(args: &IntersectArgs,
                building_geos: Vec<SimpleGeometry>,
    building_intersections: Vec<Vec<i32>>
) -> Result<()>
{
    let now =  Instant::now();
    let mut last_output = Instant::now();

    let drv = Driver::get(Driver::DRIVER_NAME_FLATGEOBUF)?;

    let mut n_processed = 0;

    info!("Writing output {:?}", &args.output_path);

    create_dir_all(args.output_path.parent().unwrap()).unwrap();

    let ds = drv.create(args.output_path.to_str().unwrap())?;

    let mut target_sr = SpatialRef::from_epsg(4326).unwrap();

    //x, y ; long/lat
    target_sr.set_axis_mapping_strategy(OSRAxisMappingStrategy::OAMS_TRADITIONAL_GIS_ORDER);

    let mut output_lyr = ds.create_layer_ext::<String>(
        args.output_path.file_stem().unwrap().to_str().unwrap(),
        &target_sr,
        OGRwkbGeometryType::wkbMultiPolygon,
        &vec![],
    )?;

    let unique_out_fields = {
        let mut u = HashMap::new();

        for of in args.out_field.iter() {
            let cur_len = u.len();
            let index = u.entry(of.to_string()).or_insert(cur_len);

            if *index == cur_len {
                //we just inserted
                let field_defn = FieldDefinition::new(of, OGRFieldType::OFTInteger)?;
                field_defn.add_to_layer(&mut output_lyr)?;

                //And the settlement level
                // let field_defn = FieldDefinition::new(&format!("{}_level",of), OGRFieldType::OFTInteger)?;
                // field_defn.add_to_layer(&mut output_lyr)?;
            }
        }

        u
    };

    let output_layer_def = output_lyr.layer_definition();

    let total_to_process = building_geos.len();



    for (b_idx, sg) in building_geos.into_iter().enumerate()
    {
        if last_output.elapsed().as_secs() >= 3 {
            last_output = Instant::now();
            print_remaining_time(&now, n_processed, total_to_process as u32);
        }

        n_processed += 1;

        let mut geom = convert_geos_to_gdal(&sg).unwrap();

        //assert!(geom.is_valid());

        let geometry_type = geom.geometry_type();

        let mut ft = Feature::new(&output_layer_def)?;

        match geometry_type {
            OGRwkbGeometryType::wkbPolygon => {
                let mp = geom.to_multi_polygon();
                ft.set_geometry_directly(mp)?;
            }
            OGRwkbGeometryType::wkbMultiPolygon => {
                    ft.set_geometry_directly(geom)?;
            }
            _ => {
                bail!("Problem");
            }
        }

        for (input_index, intersection_id) in building_intersections[b_idx].iter().enumerate() {
            if *intersection_id < 0 {
                continue;
            }

            let field_index = 2*unique_out_fields.get(&args.out_field[input_index]).unwrap();
            ft.set_field_integer_by_index(field_index as _, *intersection_id)?;

            //settlement level
            //ft.set_field_integer_by_index((1+field_index) as _, args.level_field_value[input_index] as _)?;
        }



        ft.create(&output_lyr)?;
    }

    Ok(())
}


fn get_surrounding_chunk_row_col(args: &IntersectArgs) -> Result<Vec<(isize, isize, isize)>>
{
    //let snap_raster = Raster::read(&args.ref_raster, true);
    //let snap_raster_stats = snap_raster.stats;

    // let raster_width_coords = snap_raster_stats.right_x_coord() - snap_raster_stats.origin_x;
    // let raster_height_coords = snap_raster_stats.bottom_y_coord() - snap_raster_stats.origin_y;

    // let chunk_width = raster_width_coords / args.chunk_cols as f64;
    // let chunk_height = raster_height_coords / args.chunk_rows as f64;

    let chunk_row = args.in_chunk_num / args.chunk_cols;
    let chunk_col = args.in_chunk_num % args.chunk_cols;

    debug!("Chunk row: {} Chunk col: {}", chunk_row, chunk_col);

    //Get the surrounding chunk rows/cols/chunk index
    let mut surrounding_chunk_row_col = Vec::new();

    for col_delta in -1..=1 {
        for row_delta in -1..=1 {
            let s_chunk_row = chunk_row + row_delta;
            let s_chunk_col = chunk_col + col_delta;

            if 0 <= s_chunk_row && s_chunk_row < args.chunk_rows &&
                0 <= s_chunk_col && s_chunk_col < args.chunk_cols {
                surrounding_chunk_row_col.push( (s_chunk_row, s_chunk_col, s_chunk_col+s_chunk_row*args.chunk_cols));
            }
        }
    }

    Ok(surrounding_chunk_row_col)
}

fn serialize_intersection_data(args: &IntersectArgs) -> Result<()>
{


    let layer_index_padding = get_index_width_len((args.chunk_cols * args.chunk_rows) as _);

    //let simple_context = SimpleContextHandle::new();

    // let mut geos_writer = WKBWriter::new(&simple_context).unwrap();
    // geos_writer.set_wkb_byte_order(LittleEndian);

    //let geos_reader = WKBReader::new(&simple_context).unwrap();

    for input_index in 0..args.int_chunk_dir.len() {

        let work_base_path = get_sub_dir(&args.common_work_path,
                                         format!("{}_{}", input_index, args.in_chunk_num));

        if work_base_path.exists() {
            debug!("{:?} already exists, skipping", &work_base_path);
            continue;
        }

        create_dir_all(&work_base_path).unwrap();

        let intersect_path = get_sub_dir(&args.int_chunk_dir[input_index],
                                      format!("chunk_{:0width$}.fgb", args.in_chunk_num, width = layer_index_padding));

        // create 3 files
        // An RTree, index object contains 0 based id and id field value
        // Serialized geometries in GEOS format
        // A byte offset array

        debug!("Serializing {:?}", &intersect_path);

        let mut meta_data: Vec<(u64, i32)> = Vec::new();

        let dat_file_path = get_sub_dir(&work_base_path, FILE_NAME_GEOM_GEOS_DATA);
        let mut dat_file = BufWriter::new(File::create(&dat_file_path)?);
        let mut dat_file_bytes_written = 0;

        let int_dataset = Driver::open_vector_static(intersect_path.to_str().unwrap(), true, &["VERIFY_BUFFERS=NO".to_string()]).unwrap();
        let int_layer = int_dataset.layer(0).unwrap();

        let start = Instant::now();
        let mut last_output = Instant::now();

        let num_features = int_layer.count(false);

        //let mut rtree_index_entries = Vec::new();

        let id_field_index = int_layer.layer_definition().get_field_index( &args.id_field[input_index])?;

        //Read the input geometries
        for (f_idx, int_feature) in int_layer.features().enumerate() {

            //We want the geos geometry
            let g = int_feature.geometry().as_geom();
            //let geos_g_test = convert_from_gdal_to_geos(&g, &simple_context, true)?;

            let geom_bytes = g.ewkb_bytes_raw().unwrap();

            // let geos_g = geos_reader.read_wkb(&geom_bytes)?;
            //
            // debug!("Feature {} area {} area {}",f_idx,
            //     geos_g.area()?,
            //     geos_g_test.area().unwrap());

            //Get the bytes
            //let bytes = geos_writer.write_wkb(&geos_g).unwrap();

            //let env = g.envelope();
            //let envelope = AABB::from_corners([env.MinX, env.MinY], [env.MaxX, env.MaxY]);

            let orig_fid = int_feature.get_field_as_int(id_field_index);

            // let rio = RTreeIndexObject {
            //     f_idx: f_idx as u32,
            //     orig_fid,
            //     envelope
            // };
            // rtree_index_entries.push(rio);

            meta_data.push((dat_file_bytes_written, orig_fid) );

            dat_file_bytes_written += geom_bytes.len() as u64;
            let bytes_written = dat_file.write(&geom_bytes)?;
            assert_eq!(bytes_written, geom_bytes.len());

            if last_output.elapsed().as_secs() >= 3 {
                last_output = Instant::now();
                print_remaining_time(&start,
                                     f_idx as _,
                                     num_features as _);
            }
        }

        //Add the last one of the offsets to calculate size of each entry
        meta_data.push((dat_file_bytes_written, -1) );

        let byte_offset_path = get_sub_dir(&work_base_path, FILE_NAME_GEOM_META_DATA);
        let mut f = BufWriter::new(File::create(byte_offset_path).unwrap());
        bincode::serialize_into(&mut f, &meta_data).unwrap();

        //let tree = RTree::bulk_load(rtree_index_entries);

        // println!("Serializing full rtree");
        // let rtree_path = get_sub_dir(&work_base_path, FILE_NAME_RTREE_DATA);
        // let mut rtree_writer = BufWriter::new(File::create(rtree_path).unwrap());
        // bincode::serialize_into(&mut rtree_writer, &tree).unwrap();
        // rtree_writer.flush().unwrap();
    }

    Ok(())
}

/// This is the building data
fn read_input_data<'c>(
    args: &IntersectArgs,
    simple_context: &'c SimpleContextHandle
)
-> Result< (RTree<RTreeIndexObject>, Vec<SimpleGeometry<'c>>) >
{
    let i_dataset = Driver::open_vector_static(
        args.in_path.to_str().unwrap(), true,
                                               &["VERIFY_BUFFERS=NO".to_string()]).unwrap();
    let i_layer = i_dataset.layer(0).unwrap();

    let mut geos_geometries = Vec::new();

    let mut rtree_index_entries = Vec::new();

    for (i_idx, i_feature) in i_layer.features().enumerate()
    {
        let g = i_feature.geometry().as_geom();

        let geos_g = convert_from_gdal_to_geos(&g, &simple_context, true).unwrap();

        geos_geometries.push(geos_g);

        let env = g.envelope();
        let envelope = AABB::from_corners([env.MinX, env.MinY], [env.MaxX, env.MaxY]);

        rtree_index_entries.push({
            RTreeIndexObject{
                f_idx: i_idx as u32,
                orig_fid: i_idx as i32,
                envelope
            }
        })
    }

    let rtree = RTree::bulk_load(rtree_index_entries);



    Ok( (rtree, geos_geometries ))

}


pub type Coord = f64;

#[derive(Deserialize, Serialize, Clone)]
pub struct RTreeIndexObject {
    pub f_idx: u32,
    pub orig_fid: i32,
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
        self.f_idx == other.f_idx
    }
}

impl Eq for RTreeIndexObject {}
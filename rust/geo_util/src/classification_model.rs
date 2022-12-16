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

use geo_util::gdm::constants;
use geo_util::util::RunError;
use geo_util::gdm::paths::{get_nested_file_path_with_ext, get_path_with_ext, get_file_path_with_ext, get_base_path};
use serde::{Deserialize, Serialize};
use glob::glob;
use csv::{Trim};
use std::io::{BufReader};
use std::fs::{File, create_dir_all, remove_dir_all};

use itertools::Itertools;
use std::path::PathBuf;
use geo::algorithm::intersects::Intersects;
use geo_util::io::{ReaderContext, get_sub_dir, open_rtree};

/// Read CSVs D:\git\pop_model\country_specific\CMR\working\rust_data\buildings\raster_data
/// Produce a 2nd set of CSVs in .../classified_raster_data


#[derive(Debug, Deserialize)]
struct CsvFieldsSrc {
    building_id: u32,
    building_file: u8,
    center_coord_x:  f64,
    center_coord_y: f64,
    raster_x: u32,
    raster_y: u32,
    avg_building_area_m2: f64,
    avg_closest_bldg_m: f64,
    buffer_bldg_count: u32,
    neighborhood_type_id: u8,
    pop_zone_id: u32,
    pop_zone_pop: f64,
    settlement_id: u32,
    settlement_level: u8
}


#[derive(Debug, Serialize)]
struct CsvFieldsDest {
    building_id: u32,
    building_file: u8,
    center_coord_x:  f64,
    center_coord_y: f64,
    raster_x: u32,
    raster_y: u32,
    avg_building_area_m2: f64,
    avg_closest_bldg_m: f64,
    buffer_bldg_count: u32,
    neighborhood_type_id: u8,
    pop_zone_id: u32,
    pop_zone_pop: f64,
    settlement_id: u32,
    settlement_level: u8,
    classify_perc: f64,
    classify_is_inhabited: u8,
}

/// Generate a 2nd set of CSVs that use the urban v rural classification
/// This 2nd set is done to not change the original code, this is a POC for now
fn run() -> Result<(), RunError>
{
    // Read in the pydat files and convert them to an rtree index + a dat file with the polygons
    //TODO
    //create_rtree_files(constants::PATH_CLASSIFIED);

    let src_csv_dir = get_nested_file_path_with_ext(
                constants::PATH_BUILDINGS,
                constants::PATH_RASTER_DATA,
                "*",
                constants::EXT_CSV
            );

    let target_csv_dir = get_nested_file_path_with_ext(
                constants::PATH_CLASSIFIED,
                constants::PATH_RASTER_DATA,
                "*",
                constants::EXT_CSV
            ).parent().unwrap().to_path_buf();

    println!("Creating CSVs from {:?} to {:?}", src_csv_dir, target_csv_dir);

    let base_path = get_base_path();

    let mut building_readers =
        ReaderContext::new( &get_sub_dir(&base_path, constants::PATH_BUILDINGS))?;

    let mut classified_readers = ReaderContext::new(
        &get_sub_dir(&base_path, constants::PATH_CLASSIFIED))?;

    //clean out the target csv directory
    if target_csv_dir.exists() {
        remove_dir_all(&target_csv_dir)?;
    }

    create_dir_all(&target_csv_dir)?;

    //Reading csv data


    //Get some headers

    let field_idx_is_inhabited = classified_readers.readers[0].field_header_to_idx["clssfct"];
    let field_idx_is_inhabited_perc = classified_readers.readers[0].field_header_to_idx["clssfc_"];

    let rtree = open_rtree(&get_sub_dir(&base_path, constants::PATH_CLASSIFIED))?;
    let mut classified_rtree_dat_reader = BufReader::new(File::open(get_path_with_ext(constants::PATH_CLASSIFIED, constants::EXT_DAT)).unwrap());

    for csv_path in glob(src_csv_dir.to_str().unwrap())?.filter_map(|e| e.ok()) {

        println!("Reading {:?}", csv_path);

        let target_csv_path: PathBuf = {
            let mut pb = PathBuf::from(target_csv_dir.clone());
            pb.push(csv_path.file_name().unwrap().to_str().unwrap());
            pb
        };

        let mut csv_writer = csv::WriterBuilder::new()
            .has_headers(true)
            .from_path(target_csv_path).unwrap();


        let mut rdr = csv::ReaderBuilder::new()
            .trim(Trim::All)
            .from_path(csv_path).expect("Cannot read CSV");

        for result in rdr.deserialize() {
            // Notice that we need to provide a type hint for automatic
            // deserialization.
            let record: CsvFieldsSrc = result.expect("Could not parse record");



            let reader = &mut building_readers.readers[record.building_file as usize];

            let mp = reader.read_mp(record.building_id);

            //search the classified buildings to find something that intersects the building extent
            let classified_buildings_result : Vec<_> = rtree.locate_all_at_point(&[record.center_coord_x, record.center_coord_y]).collect_vec();

            let mut num_matches = 0;

            for classified_rtree_idx in classified_buildings_result {
                let c_mp: MultiPolygonWithFID = classified_rtree_idx.read_data(&mut classified_rtree_dat_reader);

                //first we need to do a intersection check since the rtree uses envelopers
                if !c_mp.intersects(&mp) {
                    continue;
                }

                num_matches += 1;

                if num_matches > 1 {
                    println!("{:?}", record);
                    panic!("Too many matches")
                }

                //Look up the classified building fields
                let c_reader = &mut classified_readers.readers[classified_rtree_idx.path_index as usize];
                let c_fields = c_reader.read_fields(c_mp.fid);

                let target_record = CsvFieldsDest {
                    building_id: record.building_id,
                    building_file: record.building_file,
                    center_coord_x: record.center_coord_x,
                    center_coord_y: record.center_coord_y,
                    raster_x: record.raster_x,
                    raster_y: record.raster_y,
                    avg_building_area_m2: record.avg_building_area_m2,
                    avg_closest_bldg_m: record.avg_closest_bldg_m,
                    buffer_bldg_count: record.buffer_bldg_count,
                    neighborhood_type_id: record.neighborhood_type_id,
                    pop_zone_id: record.pop_zone_id,
                    pop_zone_pop: record.pop_zone_pop,
                    settlement_id: record.settlement_id,
                    settlement_level: record.settlement_level,
                    classify_perc: c_fields[field_idx_is_inhabited_perc].parse()?,
                    classify_is_inhabited: c_fields[field_idx_is_inhabited].parse()?
                };

                csv_writer.serialize(target_record).expect("CSV writing failed");
            }

            assert_eq!(1, num_matches);
        }

    }



    Ok(())
}

fn main() {
    run().unwrap();
}
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
use std::path::{PathBuf};
use anyhow::{Result, bail};
use gdal::spatial_ref::{SpatialRef, OSRAxisMappingStrategy, CoordTransform};
use gdal::vector::{Driver, Feature, OGRFieldType, OGRwkbGeometryType,
                   FieldDefinition, Dataset,
    field_type_to_name
};
use structopt::StructOpt;

use std::time::Instant;
use geo_util::io::{InputOgrLayer,};
use geo_util::util::print_remaining_time;
use std::fs::{create_dir_all};
use itertools::Itertools;
use std::collections::HashMap;
use std::fs;
//use geo_util::vector::get_fixed_geom;

/*

merges layers, ignoring overlapping shapes

cd /rust
cargo run --release --bin cmdline_tools -- merge-vector \
--ogr-conn-str /country_specific/BLDG/input/building_classification/gis_osm_buildings_a_free_13v.shp \
--layer-name all \
--ogr-conn-str /country_specific/BLDG/input/building_classification/vselectDISJOINT32733v.shp \
--layer-name all \
--work-dir /country_specific/BLDG/work \
--output-path /country_specific/BLDG/output.fgb \
--output-driver FlatGeobuf

cd /rust
cargo run --release --bin cmdline_tools -- merge-vector \
--ogr-conn-str /country_specific/BLDG/input/building_classification/gis_osm_buildings_a_free_13v.shp \
--layer-name all \
--ogr-conn-str /country_specific/BLDG/input/building_classification/vselectDISJOINT32733v.shp \
--layer-name all \
--work-dir /country_specific/BLDG/work \
--output-path /country_specific/BLDG/output.shp \
--output-driver 'ESRI Shapefile'
 */

#[derive(StructOpt)]
pub struct MergeVectorFastArgs {

    //Note this isn't generic with respect to the fields, so for now keeping osm & dg
    #[structopt(long, short="c", help="OGR Connection string for inputs")]
    pub(crate) in_ogr_conn: Vec<String>,

    #[structopt(long, short="l", help="Layer names for input, use - to use default, and all for everything")]
    pub(crate) in_ogr_layer: Vec<String>,

    #[structopt(long, short="l", help="")]
    pub(crate) in_dir: Vec<String>,

    #[structopt(parse(from_os_str), long, help="Output is a file, according to which output format")]
    pub(crate) output_file_path: PathBuf,

    #[structopt(long, help="EPSG code of output projection")]
    pub(crate) output_projection: Option<u32>,

    #[structopt(long, help="What is the output format")]
    pub(crate) output_driver: Option<String>,


    #[structopt(long)]
    pub(crate) skip_merge_cols: bool


}

fn build_input_ogr_layers(args: &MergeVectorFastArgs) -> Result<Vec<InputOgrLayer>>
{
    assert_eq!(args.in_ogr_layer.len(), args.in_ogr_conn.len());

    //assert!(args.in_ogr_conn.len() >= 1);

    let mut ret = Vec::new();

    for (i, ocs) in args.in_ogr_conn.iter().enumerate() {

        //open it to inspect the layers
        let dataset = Dataset::open(&ocs )?;

        let input_layer_name = &args.in_ogr_layer[i];

        let mut layer_names: Vec<String> = Vec::new();

        let layer_count = dataset.count();

        //assert!(!input_layer.layer_name.is_empty(), "Call try_set_empty_layer_name to set empty layer name");
        let mut at_least_one = false;
        for layer_index in 0..layer_count {
            let layer = dataset.layer(layer_index)?;
            let layer_name = layer.name();
            layer_names.push(layer.name());

            if !( (layer_index == 0 && input_layer_name == "-") ||
                (input_layer_name == "all") ||
                (input_layer_name == &layer_name) ) {
                continue;
            }

            at_least_one = true;

            ret.push(InputOgrLayer {
                    name: layer_name.clone(),
                    layer_name,
                    ogr_conn_str: ocs.clone(),
                    attribute_filter: None,
                    layer_creation_option: vec![],
                    ogr_format: None
                });

        }

        if !at_least_one {
            let layer_names = layer_names.iter().sorted().join(", ");
            bail!("No layer name found {:?} in dataset: {}.  Found layers: {}",
                                   input_layer_name, ocs, layer_names);
        }

    }

    for dir_name in args.in_dir.iter() {
        for entry in fs::read_dir(dir_name)? {
            let entry = entry?;
            if "fgb" != entry.path().extension().unwrap().to_str().unwrap() {
                continue;
            }
            let layer_name = entry.path().file_stem().unwrap().to_str().unwrap().to_string();
            ret.push(InputOgrLayer {
                name: layer_name.clone(),
                layer_name,
                ogr_conn_str: entry.path().to_str().unwrap().to_string(),
                attribute_filter: None,
                    layer_creation_option: vec![],
                    ogr_format: None
            });
        }
    }

    Ok(ret)
}

#[derive(Clone)]
struct ColumnInfo {
    ogr_type: OGRFieldType::Type,
    name: String,
    index: usize
}

fn get_combined_column_names(inputs: &Vec<InputOgrLayer>) -> Result<(Vec<ColumnInfo>, HashMap<String, ColumnInfo>)>
{
    let mut name_to_idx: HashMap<String, ColumnInfo> = HashMap::new();
    let mut list : Vec<ColumnInfo> = Vec::new();

    for input in inputs.iter() {
        let dataset = Dataset::open(&input.ogr_conn_str)?;

        let layer = dataset.layer_by_name(&input.layer_name)?;

        let layer_def = layer.layer_definition();

        for field in layer_def.fields() {
            let name_to_idx_len = name_to_idx.len();

            if field.name().is_empty() {
                continue;
            }

            let ci = name_to_idx.entry(field.name()).or_insert_with(|| {
                ColumnInfo {
                    ogr_type: field.field_type(),
                    index: name_to_idx_len,
                    name: field.name()
                }
            });

            if ci.index == list.len() {
                list.push(ci.clone());
            }

            if ci.ogr_type != field.field_type() {
                bail!("Conflicting types !  We have 2 column names with same name {} but different types: {:?} and {:?}",
                    field.name(), field_type_to_name(ci.ogr_type), field_type_to_name(field.field_type())
                );
            }
        }
    }

    assert_eq!(list.len(), name_to_idx.len());

    Ok((list, name_to_idx))
}

//We need to know the field indexes of this input to the output
fn get_mapping(
    mapping: &HashMap<String, ColumnInfo>,
    input: &InputOgrLayer
) -> Result<Vec<usize>>
{
    //open dataset to get field names
    let ds = Dataset::open(&input.ogr_conn_str)?;
    let lyr = ds.layer_by_name(&input.layer_name)?;

    Ok(lyr.layer_definition().fields().filter(|field| !field.name().is_empty()).map( |field| {
        mapping[&field.name()].index
    }).collect_vec())
}

/// Combines building layers, in order of priority, not including ones that intersect an already existing building
pub fn merge_vectors_fast(args: &MergeVectorFastArgs) -> Result<()> {
    println!("Starting merging vector layers");

    let now  = Instant::now();
    let mut last_output = Instant::now();

    if args.output_file_path.exists() {
        println!("{:?} already exists, doing nothing", &args.output_file_path);
        return Ok(());
    }

    create_dir_all(&args.output_file_path.parent().unwrap())?;

    let mut target_sr = SpatialRef::from_epsg(args.output_projection.unwrap_or(4326)).unwrap();

    target_sr.set_axis_mapping_strategy(OSRAxisMappingStrategy::OAMS_TRADITIONAL_GIS_ORDER);

    let ogr_inputs = build_input_ogr_layers(args)?;

    let output_driver_name = args.output_driver.as_ref().map_or(
        Driver::DRIVER_NAME_FLATGEOBUF, |od| &od );

    let drv = Driver::get(output_driver_name)?;
    let output_spatial_ref = SpatialRef::from_epsg(args.output_projection.unwrap_or(4326))?;

    let mut n_processed = 0;

    let ds = drv.create(args.output_file_path.to_str().unwrap())?;

    let mut output_lyr = ds.create_layer_ext::<String>(
    args.output_file_path.file_stem().unwrap().to_str().unwrap(),
        &output_spatial_ref,
        OGRwkbGeometryType::wkbMultiPolygon,
        &vec![]
    )?;

    //Build all unique column names
    let (col_info_list, col_info_map) = get_combined_column_names(&ogr_inputs)?;

    let layer_mappings = ogr_inputs.iter().map(|oi| {
        get_mapping(&col_info_map, oi).unwrap()
    }).collect_vec();

    let source_index = col_info_list.len();

    if !args.skip_merge_cols {
        for ci in col_info_list.iter() {
            let field_defn = FieldDefinition::new(&ci.name, ci.ogr_type).unwrap();
            field_defn.add_to_layer(&mut output_lyr).unwrap();
        }

        let source_defn = FieldDefinition::new("source_merge", OGRFieldType::OFTString)?;
        source_defn.add_to_layer(&mut output_lyr)?;
    }
    let output_layer_def = output_lyr.layer_definition();

    let total_to_process = {
        let mut t = 0;
        for input in ogr_inputs.iter() {
            let ds = Dataset::open(&input.ogr_conn_str)?;
            let lyr = ds.layer_by_name(&input.layer_name)?;
            t += lyr.count(false);
        }
        t
    };

    for (input_layer_index, input) in ogr_inputs.iter().enumerate() {
        let ds = Dataset::open(&input.ogr_conn_str)?;
        let lyr = ds.layer_by_name(&input.layer_name)?;

        let source = format!("{}", &input.layer_name);

        let input_crs = lyr.spatial_reference().unwrap();

        // println!("Going to convert\n{}\nto\n{}\n", input_crs.to_wkt().unwrap(),
        //          target_sr.to_wkt().unwrap());

        let transform = CoordTransform::new(&input_crs, &target_sr)?;


        for input_feature in lyr.features()
        {
            if last_output.elapsed().as_secs() >= 3 {
                last_output = Instant::now();
                print_remaining_time(&now, n_processed, total_to_process as u32);
            }

            n_processed += 1;

            let mut geom = input_feature.geometry().as_geom().clone();
            //let mut geom = get_fixed_geom(geom);

            //assert!(geom.is_valid());

            let geom_type = geom.geometry_type();
            assert_eq!(geom_type, OGRwkbGeometryType::wkbMultiPolygon);

            geom.transform_inplace(&transform).unwrap();

            //assert!(geom.is_valid());

            let mut ft = Feature::new(&output_layer_def)?;

            if !args.skip_merge_cols {
                let field_indexs = &layer_mappings[input_layer_index];
                for (idx, f_idx) in field_indexs.iter().enumerate() {
                    let input_data = input_feature.field_from_idx(idx as _)?;
                    ft.set_field_by_index(*f_idx as _, &input_data)?;
                }

                ft.set_field_string_by_index(source_index as _, &source)?;
            }
            ft.set_geometry_directly(geom).unwrap();
            ft.create(&output_lyr)?;

            // if n_processed % 10 == 0 {
            //     break;
            // }
        }
    }
    Ok(())
}


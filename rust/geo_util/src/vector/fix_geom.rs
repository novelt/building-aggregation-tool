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
use gdal::vector::{Driver, Layer,
                   geometry_type_to_name, Feature,
                   OGRwkbGeometryType, FieldDefinition,
                   OGRFieldType, Dataset, Geometry as GdalGeometry};
use anyhow::{bail, Result};

use crate::util::print_remaining_time;
use std::time::Instant;
use log::{debug,warn,info};
use crate::vector::{get_input_column_names, InputColumnInfo, add_columns_to_layer};


/// Creates fixed vector layer if it doesn't exist
pub fn run_fix_geometry(
    orig_ogr_conn_str: &str,
    orig_layer_name: &str,
    fixed_ogr_conn_str: &str,
    fixed_layer_name: &str,
    output_format: &str,
) -> Result<()> {

    if orig_ogr_conn_str.is_empty() {
        bail!("Nothing to adjust from");
    }

    //If corrected layer already exists, skip
    let output_driver = Driver::get(output_format)?;
    let output_dataset = output_driver.open(fixed_ogr_conn_str, true);

    if let Ok(id) = output_dataset {
        if fixed_layer_name.is_empty() || id.layer_by_name(&fixed_layer_name).is_ok() {
            //we can open the dataset, and if we have a layer name, the layer exists so we are done
            return Ok(());
        } else {
            warn!("Not able layer {} in open fixed dataset: {}",
                     fixed_layer_name, fixed_ogr_conn_str);
        }
    }

    debug!("Need to create fixed dataset: {}", fixed_ogr_conn_str);

    let non_adjusted_dataset = Dataset::open(orig_ogr_conn_str)?;

    let (non_adjusted_layer, input_columns) = if orig_layer_name.is_empty() {
        let lyr = non_adjusted_dataset.layer(0)?;
        let lyr_name = lyr.name();
        let ic = get_input_column_names(orig_ogr_conn_str, &lyr_name)?;
        (lyr, ic)
    } else {
        (non_adjusted_dataset.layer_by_name(orig_layer_name)?, get_input_column_names(orig_ogr_conn_str, orig_layer_name)?)
    };

    fix_geometry(
        &non_adjusted_layer,
        fixed_ogr_conn_str,
    fixed_layer_name,
        output_format,
        &input_columns
    )
}

/// Will create a fixed FlatGeobuf based on the input layer
fn fix_geometry(
    input_layer: &Layer,

    output_ogr_str: &str,
    output_layer_name: &str,

    output_format: &str,

    input_columns: &Vec<InputColumnInfo>
) -> Result<()>
{
    //See try_set_empty_layer_name if no layer name

    let now = Instant::now();
    let mut last_output= now;

    let output_driver = Driver::get(output_format)?;

    let output_dataset = output_driver.create(output_ogr_str)?;

    let spatial_ref = input_layer.spatial_reference()?;

    let geom_type = input_layer.layer_definition().get_geometry_type();

    info!("Creating geom with {} at ogr [{:?}] name {} with driver {}",
         geometry_type_to_name(geom_type)?,
         output_ogr_str, output_layer_name,
        output_format
    );

    let mut output_layer = output_dataset.create_layer_ext::<&str>(
        output_layer_name,
            &spatial_ref,
            OGRwkbGeometryType::wkbMultiPolygon,
        &[
            //"OVERWRITE=YES",
            //"SPATIAL_INDEX=NO"
        ])?;

    add_columns_to_layer(&mut output_layer, &input_columns);

    let field_defn = FieldDefinition::new("orig_fid", OGRFieldType::OFTInteger64)?;
    field_defn.add_to_layer(&mut output_layer)?;

    let input_layer_count = input_layer.count(false);
    let mut count = 0;

    let output_layer_def = output_layer.layer_definition();

    for input_feature in input_layer.features() {
        let mut ft = Feature::new(&output_layer_def)?;

        let input_fid = input_feature.fid();

        let geom = input_feature.geometry().as_geom();
        let geom = get_fixed_geom(geom);

        assert!(geom.is_valid());

        let geom_type = geom.geometry_type();

        assert_eq!(geom_type, OGRwkbGeometryType::wkbMultiPolygon);

        ft.set_geometry_directly(geom)?;

        ft.set_fid(input_fid)?;

        // Copy fields over
        for idx in 0..input_columns.len() {
            let input_field_value = input_feature.field_from_idx(idx as _)?;
            ft.set_field_by_index(idx as _, &input_field_value)?;
        }

        ft.set_field_integer64_by_index(input_columns.len() as _, input_fid)?;

        // Add the feature to the layer:
        ft.create(&output_layer)?;

        count += 1;

        if last_output.elapsed().as_secs() >= 3 {
          last_output = Instant::now();
          print_remaining_time(&now, count, input_layer_count as u32);
        }
    }

    Ok(())

}

// Returns a MultiPolygon with the fixed geometry
pub fn get_fixed_geom(mut geom: GdalGeometry) -> GdalGeometry {


    // First make sure we don't have any curves
    let has_curve_geometry = geom.has_curve_geometry(false);
    if has_curve_geometry {
        //debug!("Converting curved to linear for {}", input_fid);
        geom = geom.get_linear_geometry();
    }

    debug_assert!(!geom.has_curve_geometry(true));
    debug_assert!(!geom.has_curve_geometry(false));

    let input_is_valid = geom.is_valid();

    // Next make sure the geometry is valid
    if !input_is_valid {
        //debug!("Non valid geometry found");

        geom = geom.make_valid();

        //debug!("Remove lower dim sub geoms for fid {}", input_fid);

        //geom = geom.remove_lower_dim_sub_geoms();

        //debug!("Make valid done for {}", input_fid);
    }

    debug_assert!(geom.is_valid());

    let geom_type = geom.geometry_type();

    if geom_type == OGRwkbGeometryType::wkbGeometryCollection {
        //many single polygons
        let mut single_polygon_count = 0;
        let mut multi_polygon_count = 0;
        for i in 0..geom.geometry_count() {
            let sub_geom = geom.get_geometry(i);
            let sub_geom_type = sub_geom.geometry_type();

            if sub_geom_type == OGRwkbGeometryType::wkbPolygon {
                single_polygon_count += 1
            } else if sub_geom_type == OGRwkbGeometryType::wkbMultiPolygon {
                multi_polygon_count += 1
            }
            else {
                debug!("Type in geo collection: {}", sub_geom.geometry_name());
            }

        }

        assert_eq!(multi_polygon_count + single_polygon_count, 1);
        //assert!(multi_polygon_count + single_polygon_count > 0);

        //Now that we know this collection only has 1 polygon geometry, we clone it
        for i in 0..geom.geometry_count() {
            let sub_geom = geom.get_geometry(i);
            let sub_geom_type = sub_geom.geometry_type();

            if sub_geom_type == OGRwkbGeometryType::wkbPolygon {
                geom = sub_geom.clone().to_multi_polygon();
            }
            else if sub_geom_type == OGRwkbGeometryType::wkbMultiPolygon {
                geom = sub_geom.clone();
            }
            break;
        }
    }


    // If we have transformed the geometry, it has already been cloned (owned = true)
    // but if not, we need to make sure we copy the geometry before calling set geometry directly
    if !geom.is_owned() {
        //debug!("Cloning non owned geometry {}", input_fid);
        //Since we use set_geometry_directly
        geom = geom.clone();
    } else {
        //debug!("No clone needed for owned geometry {}", input_fid);
    }

    assert!(geom.is_owned());

    // We want multipolygons
    if geom_type == OGRwkbGeometryType::wkbPolygon {
        //debug!("To multi polygon, is owned? {} ", geom.is_owned());
        geom = geom.to_multi_polygon();

        //debug!("Done to multi polygon, is owned? {} for fid {}", geom.is_owned(), input_fid);
    }

    assert!(geom.is_owned());

    return geom;
}
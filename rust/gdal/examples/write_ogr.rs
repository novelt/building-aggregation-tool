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
use gdal::vector::{Driver, FieldDefinition, FieldValue, Geometry, OGRFieldType, Feature};
use std::fs;
use anyhow::Result;

/// Example 1, the detailed way:
fn example_1() -> Result<()> {
    let _ = fs::remove_file("/tmp/output1.geojson");
    let drv = Driver::get("GeoJSON")?;
    let mut ds = drv.create("/tmp/output1.geojson")?;

    let mut lyr = ds.create_layer()?;

    let field_defn = FieldDefinition::new("Name", OGRFieldType::OFTString)?;
    field_defn.set_width(80);
    field_defn.add_to_layer(&mut lyr)?;

    let field_defn = FieldDefinition::new("Value", OGRFieldType::OFTReal)?;
    field_defn.add_to_layer(&mut lyr)?;

    let defn = lyr.layer_definition();
    
    let name_index = 0;
    let value_index = 1;

    // 1st feature:
    let mut ft = Feature::new(&defn)?;
    ft.set_geometry_directly(Geometry::from_wkt("POINT (45.21 21.76)")?)?;
    ft.set_field_string_by_index(name_index, "Feature 1")?;
    ft.set_field_double_by_index(value_index, 45.78)?;
    ft.create(&lyr)?;

    // 2nd feature:
    let mut ft = Feature::new(&defn)?;
    ft.set_field_double_by_index(value_index, 0.789)?;
    ft.set_geometry_directly(Geometry::from_wkt("POINT (46.50 22.50)")?)?;
    ft.set_field_string_by_index(name_index, "Feature 2")?;
    ft.create(&lyr)?;

    // Feature triggering an error due to a wrong field name:
    let mut ft = Feature::new(&defn)?;
    ft.set_geometry_directly(Geometry::from_wkt("POINT (46.50 22.50)")?)?;
    ft.set_field_string_by_index(name_index, "Feature 2")?;
    match ft.set_field_double_by_index(value_index, 0.789) {
        Ok(v) => v,
        Err(err) => println!("{}", err),
    };
    ft.create(&lyr)?;

    Ok(())
}

/// Example 2, same output, shortened way:
fn example_2() -> Result<()> {
    let _ = fs::remove_file("/tmp/output2.geojson");
    let driver = Driver::get("GeoJSON")?;
    let mut ds = driver.create("/tmp/output2.geojson")?;
    let mut layer = ds.create_layer()?;

    layer.create_defn_fields(&[
        ("Name", OGRFieldType::OFTString),
        ("Value", OGRFieldType::OFTReal),
    ])?;

    layer.create_feature_fields(
        Geometry::from_wkt("POINT (45.21 21.76)")?,
        &["Name", "Value"],
        &[
            FieldValue::StringValue("Feature 1".to_string()),
            FieldValue::RealValue(45.78),
        ],
    )?;

    layer.create_feature_fields(
        Geometry::from_wkt("POINT (46.50 22.50)")?,
        &["Name", "Value"],
        &[
            FieldValue::StringValue("Feature 2".to_string()),
            FieldValue::RealValue(0.789),
        ],
    )?;

    // Feature creation triggering an error due to a wrong field name:
    match layer.create_feature_fields(
        Geometry::from_wkt("POINT (46.50 22.50)")?,
        &["Abcd", "Value"],
        &[
            FieldValue::StringValue("Feature 2".to_string()),
            FieldValue::RealValue(0.789),
        ],
    ) {
        Ok(v) => v,
        Err(err) => println!("{}", err),
    };

    Ok(())
}

fn main() {
    example_1().unwrap();
    example_2().unwrap();
}

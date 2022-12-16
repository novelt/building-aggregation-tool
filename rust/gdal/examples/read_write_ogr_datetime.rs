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
//#[cfg(feature = "datetime")]
use chrono::Duration;
use anyhow::Result;
use gdal::vector::*;
use std::ops::Add;

//#[cfg(feature = "datetime")]
fn run() -> Result<()> {
    println!("gdal crate was build with datetime support");

    let dataset_a = Dataset::open("fixtures/points_with_datetime.json")?;
    let layer_a = dataset_a.layer(0)?;

    // Create a new dataset:
    let _ = std::fs::remove_file("/tmp/later.geojson");
    let drv = Driver::get("GeoJSON")?;
    let mut ds = drv.create("/tmp/later.geojson")?;
    let mut lyr = ds.create_layer()?;

    // Copy the origin layer shema to the destination layer:
    for field in layer_a.layer_definition().fields() {
        let field_defn = FieldDefinition::new(&field.name(), field.field_type())?;
        field_defn.set_width(field.width());
        field_defn.add_to_layer(&mut lyr)?;
    }

    // Get the definition to use on each feature:
    let defn = lyr.layer_definition();

    for feature_a in layer_a.features() {
        let mut ft = Feature::new(&defn)?;
        ft.set_geometry_directly(feature_a.geometry().as_geom().clone())?;
        // copy each field value of the feature:
        for field in defn.fields() {
            ft.set_field(
                &field.name(),
                &match feature_a.field(&field.name())? {
                    // add one day to dates
                    FieldValue::DateValue(value) => {
                        println!("{} = {}", field.name(), value);
                        FieldValue::DateValue(value.add(Duration::days(1)))
                    }

                    // add 6 hours to datetimes
                    FieldValue::DateTimeValue(value) => {
                        println!("{} = {}", field.name(), value);
                        FieldValue::DateTimeValue(value.add(Duration::hours(6)))
                    }
                    v => v,
                },
            )?;
        }
        // Add the feature to the layer:
        ft.create(&lyr)?;
    }
    Ok(())
}


fn main() {
    run().unwrap();
}

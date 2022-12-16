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
use gdal::spatial_ref::{CoordTransform, SpatialRef};
use gdal::vector::*;
use std::fs;
use anyhow::Result;

fn run() -> Result<()> {
    let dataset_a = Dataset::open("fixtures/roads.geojson")?;
    let layer_a = dataset_a.layer(0)?;
    let fields_defn = layer_a
        .layer_definition()
        .fields()
        .map(|field| (field.name(), field.field_type(), field.width()))
        .collect::<Vec<_>>();

    // Create a new dataset:
    let _ = fs::remove_file("/tmp/abcde.shp");
    let drv = Driver::get("ESRI Shapefile")?;
    let mut ds = drv.create("/tmp/abcde.shp")?;
    let mut lyr = ds.create_layer()?;

    // Copy the origin layer shema to the destination layer:
    for fd in &fields_defn {
        let field_defn = FieldDefinition::new(&fd.0, fd.1)?;
        field_defn.set_width(fd.2);
        field_defn.add_to_layer(&mut lyr)?;
    }

    // Prepare the origin and destination spatial references objects:
    let spatial_ref_src = SpatialRef::from_epsg(4326)?;
    let spatial_ref_dst = SpatialRef::from_epsg(3025)?;

    // And the feature used to actually transform the geometries:
    let htransform = CoordTransform::new(&spatial_ref_src, &spatial_ref_dst)?;

    // Get the definition to use on each feature:
    let defn = lyr.layer_definition();

    for feature_a in layer_a.features() {
        // Get the original geometry:
        let geom = feature_a.geometry();
        // Get a new transformed geometry:
        let new_geom = geom.as_geom().transform(&htransform)?;
        // Create the new feature, set its geometry:
        let mut ft = Feature::new(&defn)?;
        ft.set_geometry_directly(new_geom)?;
        // copy each field value of the feature:
        for fd in &fields_defn {
            ft.set_field(&fd.0, &feature_a.field(&fd.0)?)?;
        }
        // Add the feature to the layer:
        ft.create(&lyr)?;
    }

    Ok(())
}

fn main() {
    run().unwrap();
}

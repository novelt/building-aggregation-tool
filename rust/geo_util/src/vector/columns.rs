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
use gdal::vector::{Dataset, OGRFieldType, FieldDefinition, Layer};
use anyhow::{Result};

#[derive(Clone)]
pub struct InputColumnInfo {
    pub ogr_type: OGRFieldType::Type,
    pub name: String,
}

pub fn get_input_column_names(in_ogr_conn: &str, in_ogr_layer: &str,) -> Result<Vec<InputColumnInfo>>
{
    let mut list : Vec<InputColumnInfo> = Vec::new();

    let dataset = Dataset::open(in_ogr_conn)?;

    let layer = dataset.layer_by_name(in_ogr_layer)?;

    let layer_def = layer.layer_definition();

    for field in layer_def.fields() {

        let ci = InputColumnInfo {
                ogr_type: field.field_type(),
                name: field.name()
            };

        list.push(ci.clone());

    }

    Ok(list)
}

pub fn add_columns_to_layer(out_lyr: &mut Layer, input_columns: &[InputColumnInfo]) {
    for ci in input_columns.iter() {
        let field_defn = FieldDefinition::new(&ci.name, ci.ogr_type).unwrap();
        field_defn.add_to_layer(out_lyr).unwrap();
    }
}

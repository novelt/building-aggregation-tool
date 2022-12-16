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
use serde::{Deserialize, Serialize};
use std::fmt;
use anyhow::{Result, bail};
use gdal::vector::Dataset;

#[derive(Deserialize, Serialize, Debug, Clone, Default,)]
#[serde(deny_unknown_fields)]
pub struct InputOgrLayer {
    //used to name the files
    #[serde(default)]
    pub name: String,

    //ogr layer name, if empty will be set to layer name if dataset has 1 layer (like shapefiles) with try_set_empty_layer_name
    #[serde(default)]
    pub layer_name: String,

    pub ogr_conn_str: String,
    #[serde(rename="ogr_filter")]
    pub attribute_filter: Option<String>,

    #[serde(default)]
    pub layer_creation_option: Vec<String>,

    pub ogr_format: Option<String>
}

impl InputOgrLayer {
    pub fn try_set_empty_layer_name(&mut self) -> Result<()> {
        if !self.layer_name.is_empty() {
            return Ok(());
        }

        let dataset = Dataset::open(&self.ogr_conn_str)?;

        let layer_count = dataset.count();

        if layer_count != 1 {
            bail!("No layer name and layer count is {} for dataset {}.  Must be 1 to know layer name", layer_count, &self.ogr_conn_str);
        }

        self.layer_name = dataset.layer(0)?.name().clone();

        Ok(())
    }
}



impl fmt::Display for InputOgrLayer {
    // This trait requires `fmt` with this exact signature.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Write strictly the first element into the supplied output
        // stream: `f`. Returns `fmt::Result` which indicates whether the
        // operation succeeded or failed. Note that `write!` uses syntax which
        // is very similar to `println!`.
        write!(f, "Name {:?} Layer: {:?} ogr conn: {}",
               self.name,
               self.layer_name,
               self.ogr_conn_str
        )
    }
}

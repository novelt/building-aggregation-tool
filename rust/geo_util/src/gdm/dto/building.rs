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
use crate::io::SetArea;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct Building {
    //This will be populated by whatever the FID field is, often this is objectid
    #[serde(rename(deserialize = "FID"))]
    pub orig_fid: u32,

    //Special handling in gdal deserializer to skip this
    pub area: f32,

    // pub predictions_probability: String,
    //
    // //#[serde(rename(deserialize = "predictions"))]
    // pub predictions: String,
}

impl SetArea for Building {
    fn set_area(&mut self, area: f32) {
        self.area = area;
    }
}
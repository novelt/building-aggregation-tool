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
//settlement fid,settlement_level,raster x,raster y,longitude,latitude,grid_index,corner
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
#[repr(u8)]
pub enum Corners {
    NorthEast = 0,
    SouthEast = 1,
    SouthWest = 2,
    NorthWest = 3
}

#[derive(Deserialize, Serialize)]
pub struct RasterCenterDto {
    pub settlement_fid: i32,
    pub settlement_level: u8,
    pub raster_x: i32,
    pub raster_y: i32,
    pub long: f64,
    pub lat: f64,
    pub grid_index: i32,
    pub corner: Corners
}
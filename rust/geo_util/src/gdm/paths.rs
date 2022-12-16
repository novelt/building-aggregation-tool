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
use crate::gdm::constants;


pub fn get_raster_csv_name(start_x: u16, stop_x: u16, start_y: u16, stop_y: u16) -> String
{
    format!("raster_sq_x{:0nz$}_{:0nz$}__y{:0nz$}_{:0nz$}.{}", start_x,
                     stop_x, start_y, stop_y, constants::EXT_CSV, nz = 5)
}


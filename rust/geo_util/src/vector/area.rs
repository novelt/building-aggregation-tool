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
use geo::MultiPolygon;
use geo::algorithm::chamberlain_duquette_area::ChamberlainDuquetteArea;

pub fn get_multi_poly_area(polygon: &MultiPolygon<f64>) -> f64 {
    let mut area = 0f64;

    for polygon in polygon.0.iter() {
        area += polygon.chamberlain_duquette_unsigned_area();
    }

    area
}
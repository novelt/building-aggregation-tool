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

pub const PATH_RASTER_DATA: &str = "raster_data";
pub const PATH_CLASSIFIED: &str = "classified";

pub const EXT_DAT: &str = "dat";
pub const EXT_CSV: &str = "csv";
pub const EXT_RTREE: &str = "rtree";
pub const EXT_OFFSET: &str = "offsets";

///https://github.com/openlayers/openlayers/blob/139b048197f705ccd26919813a6728496423b4be/src/ol/sphere.js#L24
/**
 * The mean Earth radius (1/3 * (2a + b)) for the WGS84 ellipsoid.
 * https://en.wikipedia.org/wiki/Earth_radius#Mean_radius
 * in meters
 */
pub const DEFAULT_RADIUS: f64 = 6371008.8;
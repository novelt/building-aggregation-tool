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
use diesel::deserialize::FromSql;
use diesel::{deserialize};
use diesel::pg::Pg;
//use std::io::Write;
//use diesel::serialize::{ToSql, Output, IsNull};
use postgis::ewkb::{ Polygon};
use diesel::*;
use geo::{LineString as GeoLineString, Coordinate, Polygon as GeoPolygon};
use itertools::Itertools;

#[derive(SqlType, QueryId)]
#[postgres(type_name = "geography")]
pub struct Geography;

#[derive(FromSqlRow, Debug)]
pub struct DbPolygon(pub GeoPolygon<f64>);

impl FromSql<Geography, Pg> for DbPolygon {
	//How to read a polygon to a Dbpolygon type
	fn from_sql(bytes: Option<&[u8]>) -> deserialize::Result<Self> {
		use std::io::Cursor;
		use postgis::ewkb::EwkbRead;
		let bytes = not_none!(bytes);
		let mut rdr = Cursor::new(bytes);

        let polygon = Polygon::read_ewkb(&mut rdr)?;

		//Convert to the geo structure
		let mut geo_line_strings = polygon.rings.iter().map( |postgis_ls| {
			let geopoints_vec = postgis_ls.points.iter().map( |postgis_pt| {
				Coordinate{x: postgis_pt.x, y:postgis_pt.y}
			}).collect_vec();

			GeoLineString(geopoints_vec)
		}).collect_vec();

		let exterior = geo_line_strings.remove(0);
		let polygon = GeoPolygon::new(exterior, geo_line_strings);

		Ok(DbPolygon(polygon))
	}
}

/*
impl ToSql<Geography, Pg> for DbPolygon {
	fn to_sql<W: Write>(&self, out: &mut Output<W, Pg>) -> serialize::Result {
		use postgis::ewkb::{AsEwkbPoint, EwkbWrite};
		Point::from(*self).as_ewkb().write_ewkb(out)?;
		Ok(IsNull::No)
	}
}*/
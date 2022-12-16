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
use std::fmt::Debug;
use gdal::vector::{Geometry as GdalGeometry, OGRwkbGeometryType};
use anyhow::{anyhow, Result};
use geo::{Point, Geometry,
          MultiPoint, MultiLineString,
          LineString, GeometryCollection, MultiPolygon, Polygon,
    Line
};

use crate::convert::traits::ToGdal;
use num::Float;

impl<T> ToGdal for Point<T>
where
    T: Float + Debug,
{
    fn to_gdal(&self) -> Result<GdalGeometry> {
        let mut geom = GdalGeometry::empty(OGRwkbGeometryType::wkbPoint)?;
        let &Point(coordinate) = self;
        geom.set_point_2d(
            0,
            (
                coordinate.x.to_f64().ok_or(anyhow!("f64 cast"))?,
                coordinate.y.to_f64().ok_or(anyhow!("f64 cast"))?,
            ),
        );
        Ok(geom)
    }
}

impl<T> ToGdal for MultiPoint<T>
where
    T: Float + Debug,
{
    fn to_gdal(&self) -> Result<GdalGeometry> {
        let mut geom = GdalGeometry::empty(OGRwkbGeometryType::wkbMultiPoint)?;
        let &MultiPoint(ref point_list) = self;
        for point in point_list.iter() {
            geom.add_geometry(point.to_gdal()?)?;
        }
        Ok(geom)
    }
}

fn geometry_with_points<T>(
    wkb_type: OGRwkbGeometryType::Type,
    points: &LineString<T>,
) -> Result<GdalGeometry>
where
    T: Float + Debug,
{
    let mut geom = GdalGeometry::empty(wkb_type)?;
    let &LineString(ref linestring) = points;
    for (i, &coordinate) in linestring.iter().enumerate() {
        geom.set_point_2d(
            i,
            (
                coordinate.x.to_f64().ok_or(anyhow!("f64 cast"))?,
                coordinate.y.to_f64().ok_or(anyhow!("f64 cast"))?,
            ),
        );
    }
    Ok(geom)
}

impl<T> ToGdal for Line<T>
where
    T: Float + Debug,
{
    fn to_gdal(&self) -> Result<GdalGeometry> {
        let mut geom = GdalGeometry::empty(OGRwkbGeometryType::wkbLineString)?;
        geom.set_point_2d(
            0,
            (
                self.start.x.to_f64().ok_or(anyhow!("f64 cast"))?,
                self.start.y.to_f64().ok_or(anyhow!("f64 cast"))?,
            ),
        );
        geom.set_point_2d(
            1,
            (
                self.end.x.to_f64().ok_or(anyhow!("f64 cast"))?,
                self.end.y.to_f64().ok_or(anyhow!("f64 cast"))?,
            ),
        );
        Ok(geom)
    }
}

impl<T> ToGdal for LineString<T>
where
    T: Float + Debug,
{
    fn to_gdal(&self) -> Result<GdalGeometry> {
        geometry_with_points(OGRwkbGeometryType::wkbLineString, self)
    }
}

impl<T> ToGdal for MultiLineString<T>
where
    T: Float + Debug,
{
    fn to_gdal(&self) -> Result<GdalGeometry> {
        let mut geom = GdalGeometry::empty(OGRwkbGeometryType::wkbMultiLineString)?;
        let &MultiLineString(ref point_list) = self;
        for point in point_list.iter() {
            geom.add_geometry(point.to_gdal()?)?;
        }
        Ok(geom)
    }
}

impl<T> ToGdal for Polygon<T>
where
    T: Float + Debug,
{
    fn to_gdal(&self) -> Result<GdalGeometry> {
        let mut geom = GdalGeometry::empty(OGRwkbGeometryType::wkbPolygon)?;
        let exterior = self.exterior();
        let interiors = self.interiors();
        geom.add_geometry(geometry_with_points(
            OGRwkbGeometryType::wkbLinearRing,
            exterior,
        )?)?;
        for ring in interiors.iter() {
            geom.add_geometry(geometry_with_points(
                OGRwkbGeometryType::wkbLinearRing,
                ring,
            )?)?;
        }
        Ok(geom)
    }
}

impl<T> ToGdal for MultiPolygon<T>
where
    T: Float + Debug,
{
    fn to_gdal(&self) -> Result<GdalGeometry> {
        let mut geom = GdalGeometry::empty(OGRwkbGeometryType::wkbMultiPolygon)?;
        let &MultiPolygon(ref polygon_list) = self;
        for polygon in polygon_list.iter() {
            geom.add_geometry(polygon.to_gdal()?)?;
        }
        Ok(geom)
    }
}

impl<T> ToGdal for GeometryCollection<T>
where
    T: Float + Debug,
{
    fn to_gdal(&self) -> Result<GdalGeometry> {
        let mut geom = GdalGeometry::empty(OGRwkbGeometryType::wkbGeometryCollection)?;
        let &GeometryCollection(ref item_list) = self;
        for item in item_list.iter() {
            geom.add_geometry(item.to_gdal()?)?;
        }
        Ok(geom)
    }
}

impl<T> ToGdal for Geometry<T>
where
    T: Float + Debug,
{
    fn to_gdal(&self) -> Result<GdalGeometry> {
        match *self {
            Geometry::Point(ref c) => c.to_gdal(),
            Geometry::Line(ref c) => c.to_gdal(),
            Geometry::LineString(ref c) => c.to_gdal(),
            Geometry::Polygon(ref c) => c.to_gdal(),
            Geometry::MultiPoint(ref c) => c.to_gdal(),
            Geometry::MultiLineString(ref c) => c.to_gdal(),
            Geometry::MultiPolygon(ref c) => c.to_gdal(),
            Geometry::GeometryCollection(ref c) => c.to_gdal(),
            _ => panic!("Unknown type")
        }
    }
}

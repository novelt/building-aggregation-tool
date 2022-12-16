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
use anyhow::{Result, bail};
use geos::{SimpleGeometry, SimpleCoordinateSequence, GeometryTypes};
use gdal::vector::{OGRwkbGeometryType, Geometry as GdalGeometry};

fn geometry_with_points(
    wkb_type: OGRwkbGeometryType::Type,
    points: &SimpleCoordinateSequence,
) -> Result<GdalGeometry>
{
    let mut geom = GdalGeometry::empty(wkb_type)?;

    for (i, pt) in points.points()?.enumerate() {
        //println!("Set point number {}", i);
        geom.set_point_2d(
            i,
            (
                pt[0],
                pt[1]
            ),
        );
    }
    Ok(geom)
}

pub fn convert_geos_to_gdal(geom: &SimpleGeometry) -> Result<GdalGeometry> {
    match geom.geometry_type() {
        GeometryTypes::Point => bail!("not implemented yet"),
        GeometryTypes::LineString => bail!("not implemented yet"),
        GeometryTypes::LinearRing => bail!("not implemented yet"),
        GeometryTypes::Polygon => convert_geos_polygon_to_gdal(geom, true),
        GeometryTypes::MultiPoint => bail!("not implemented yet"),
        GeometryTypes::MultiLineString => bail!("not implemented yet"),
        GeometryTypes::MultiPolygon => {
            let mut gdal_geom = GdalGeometry::empty(OGRwkbGeometryType::wkbMultiPolygon)?;
            let num_geom = geom.get_num_geometries()?;
            for p in 0..num_geom {
                gdal_geom.add_geometry( convert_geos_polygon_to_gdal(&geom.get_geometry_n(p)?, true)? )?;
            }
            Ok(gdal_geom)
        },
        GeometryTypes::GeometryCollection => bail!("not implemented yet"),
        GeometryTypes::__Unknonwn(_) => bail!("not implemented yet"),
    }
}

pub fn convert_geos_to_gdal_no_holes(geom: &SimpleGeometry) -> Result<GdalGeometry> {
    match geom.geometry_type() {
        GeometryTypes::Point => bail!("not implemented yet"),
        GeometryTypes::LineString => bail!("not implemented yet"),
        GeometryTypes::LinearRing => bail!("not implemented yet"),
        GeometryTypes::Polygon => convert_geos_polygon_to_gdal(geom, false),
        GeometryTypes::MultiPoint => bail!("not implemented yet"),
        GeometryTypes::MultiLineString => bail!("not implemented yet"),
        GeometryTypes::MultiPolygon => {
            let mut gdal_geom = GdalGeometry::empty(OGRwkbGeometryType::wkbMultiPolygon)?;
            let num_geom = geom.get_num_geometries()?;
            for p in 0..num_geom {
                gdal_geom.add_geometry( convert_geos_polygon_to_gdal(&geom.get_geometry_n(p)?, false)? )?;
            }
            Ok(gdal_geom)
        },
        GeometryTypes::GeometryCollection => bail!("not implemented yet"),
        GeometryTypes::__Unknonwn(_) => bail!("not implemented yet"),
    }
}

fn convert_geos_polygon_to_gdal(geom: &SimpleGeometry, include_holes: bool) -> Result<GdalGeometry> {

    assert_eq!(geom.geometry_type(), GeometryTypes::Polygon);

    let mut gdal_geom = GdalGeometry::empty(OGRwkbGeometryType::wkbPolygon)?;
    let exterior = geom.get_exterior_ring().unwrap();

    gdal_geom.add_geometry(geometry_with_points(
        OGRwkbGeometryType::wkbLinearRing,
        &exterior.get_coord_sequence()?,
    )?)?;

    if include_holes {
        let n_interior = geom.get_num_interior_rings().unwrap();

        for int_ring in 0..n_interior {
            let interior = geom.get_interior_ring_n(int_ring as _)?;
            gdal_geom.add_geometry(geometry_with_points(
                OGRwkbGeometryType::wkbLinearRing,
                &interior.get_coord_sequence()?,
            )?)?;
        }
    }

    Ok(gdal_geom)
}
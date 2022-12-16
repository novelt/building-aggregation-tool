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
use anyhow::{bail, Result};
use gdal::vector::{OGRwkbGeometryType, Geometry as GdalGeometry};
use geos::{SimpleGeometry, GeometryTypes, SimpleContextHandle, SimpleCoordinateSequence};

//GDAL to Geos

pub fn convert_from_gdal_to_geos<'c>(
    geo: &GdalGeometry, context: &'c SimpleContextHandle, create_rings: bool) -> Result<SimpleGeometry<'c>> {
    let geometry_type = geo.geometry_type();

    Ok(match geometry_type {
        OGRwkbGeometryType::wkbPoint => {
            let [x, y] = geo.get_point(0);
            SimpleGeometry::create_point_xy(context, x, y)?
        }
        OGRwkbGeometryType::wkbMultiPoint => {
            let point_count = geo.geometry_count();
            let coords = (0..point_count)
                .map(|n| convert_from_gdal_to_geos(&geo.get_geometry(n), context, create_rings).unwrap())
                .collect();
            SimpleGeometry::create_multi_geom(context, coords, GeometryTypes::MultiPoint)?
        }
        OGRwkbGeometryType::wkbLineString => {
            let pc = geo.point_count();
            let mut geos_point_vec = Vec::with_capacity(pc);

            for p in 0..pc {
                let gdal_point = geo.get_point(p as _);
                geos_point_vec.push( gdal_point);
            }

            //println!("Making coord seq");
            let coord_seq = SimpleCoordinateSequence::from_slice_pts(&geos_point_vec, context).unwrap();
            //println!("Done Making coord seq");

            if create_rings {
                SimpleGeometry::create_linear_ring(coord_seq)?
            } else {
                SimpleGeometry::create_line_string(coord_seq)?
            }

        }
        OGRwkbGeometryType::wkbMultiLineString => {
            let line_count = geo.geometry_count();
            let coords = (0..line_count)
                .map(|n| convert_from_gdal_to_geos(&geo.get_geometry(n), context, create_rings).unwrap())
                .collect();
            SimpleGeometry::create_multi_geom(context, coords, GeometryTypes::MultiLineString)?
        }
        OGRwkbGeometryType::wkbPolygon => {
            let ring_count = geo.geometry_count();
            let outer = geo.get_geometry(0);
            let geos_ring = convert_from_gdal_to_geos(&outer, context, true)?;
            let holes = (1..ring_count)
                .map(|r| convert_from_gdal_to_geos(&geo.get_geometry(r), context, true).unwrap()).collect();
            SimpleGeometry::create_polygon(geos_ring, holes)?
        }
        OGRwkbGeometryType::wkbMultiPolygon => {
            let poly_count = geo.geometry_count();
            let polygons = (0..poly_count)
                .map(|n| convert_from_gdal_to_geos(&geo.get_geometry(n), context, create_rings).unwrap())
                .collect();
            SimpleGeometry::create_multi_geom(context, polygons, GeometryTypes::MultiPolygon)?
        }
        OGRwkbGeometryType::wkbGeometryCollection => {
            let geom_count = geo.geometry_count();
            let coords = (0..geom_count)
                .map(|n| convert_from_gdal_to_geos(&geo.get_geometry(n), context, create_rings).unwrap())
                .collect();
            SimpleGeometry::create_multi_geom(context, coords, GeometryTypes::GeometryCollection)?
        }
        _ => bail!("Unknown geometry type"),

    })
}


pub fn convert_from_gdal_to_geos_no_holes<'c>(
    geo: &GdalGeometry, context: &'c SimpleContextHandle, create_rings: bool) -> Result<SimpleGeometry<'c>> {
    let geometry_type = geo.geometry_type();

    Ok(match geometry_type {

        OGRwkbGeometryType::wkbPolygon => {
            let outer = geo.get_geometry(0);
            let geos_ring = convert_from_gdal_to_geos(&outer, context, true)?;

            SimpleGeometry::create_polygon(geos_ring, vec![])?
        }
        OGRwkbGeometryType::wkbMultiPolygon => {
            let poly_count = geo.geometry_count();
            let polygons = (0..poly_count)
                .map(|n| convert_from_gdal_to_geos_no_holes(&geo.get_geometry(n), context, create_rings).unwrap())
                .collect();
            SimpleGeometry::create_multi_geom(context, polygons, GeometryTypes::MultiPolygon)?
        }
        _ => {
            convert_from_gdal_to_geos(geo, context, create_rings)?
        }

    })
}

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
use gdal::spatial_ref::{CoordTransform};
use geos::{SimpleGeometry, GeometryTypes, SimpleCoordinateSequence, SimpleContextHandle};
use anyhow::{Result, bail};
use itertools::Itertools;

pub fn transform_geos<'c>(
    context: &'c SimpleContextHandle,
    transform: &CoordTransform,
    shape: &SimpleGeometry) -> Result<SimpleGeometry<'c>>
{
    return match shape.geometry_type()  {
        GeometryTypes::MultiPolygon => transform_geos_multipolygon(context, transform, shape),
        GeometryTypes::Polygon => transform_geos_polygon(context, transform, shape),
        GeometryTypes::Point => {


            let coord_seq = shape.get_coord_sequence()?;
            assert_eq!(1, coord_seq.num_points()?);
            let x = coord_seq.get_x(0)?;
            let y = coord_seq.get_y(0)?;

            let mut xs = [x];
            let mut ys = [y];
            let mut zs = [0.0];

            transform.transform_coords(&mut xs, &mut ys, &mut zs)?;

            SimpleGeometry::create_point_xy(context, xs[0], ys[0])
        },
        _ => {
            bail!("Unrecognized type: {:?}", shape.geometry_type());
        }
    };
}

pub fn transform_geos_multipolygon<'c>(
    context: &'c SimpleContextHandle,
    transform: &CoordTransform,
    multi_polygon: &SimpleGeometry) -> Result<SimpleGeometry<'c>>
{
    //Make sure X, Y order
    /*srs_from
        .set_axis_mapping_strategy(OSRAxisMappingStrategy::OAMS_TRADITIONAL_GIS_ORDER);
    srs_to
        .set_axis_mapping_strategy(OSRAxisMappingStrategy::OAMS_TRADITIONAL_GIS_ORDER);*/

    assert_eq!(multi_polygon.geometry_type(), GeometryTypes::MultiPolygon);

    let num_geometries = multi_polygon.get_num_geometries()?;
    let mut vec_polys = Vec::with_capacity(num_geometries);
    for n in 0..num_geometries {
        let poly = multi_polygon.get_geometry_n(n)?;
        vec_polys.push(
            transform_geos_polygon(context, transform, &poly)?
        );
    }

    SimpleGeometry::create_multi_geom(
        context,
        vec_polys,
        GeometryTypes::MultiPolygon
    )
}

/// Uses a GDAL coordinate transform to transform point by point a polygon
pub fn transform_geos_polygon<'c>(
    context: &'c SimpleContextHandle,
    transform: &CoordTransform,
    polygon: &SimpleGeometry) -> Result<SimpleGeometry<'c>>
{
    //Make sure X, Y order
    /*srs_from
        .set_axis_mapping_strategy(OSRAxisMappingStrategy::OAMS_TRADITIONAL_GIS_ORDER);
    srs_to
        .set_axis_mapping_strategy(OSRAxisMappingStrategy::OAMS_TRADITIONAL_GIS_ORDER);*/

    assert_eq!(polygon.geometry_type(), GeometryTypes::Polygon);

    //first we need all the points in 2 arrays -- x & y
    let exterior = polygon.get_exterior_ring()?;
    let n_interior = polygon.get_num_interior_rings()?;

    let transformed_exterior = transform_linear_ring(
        context, transform, &exterior)?;

    let transformed_interior_rings = (0..n_interior).map(|ir| {
        let interior = polygon.get_interior_ring_n(ir as _).expect("Get interior");
        let transfomed_interior = transform_linear_ring(
            context, transform, &interior).expect("Failed hole transform");
        transfomed_interior
    }).collect_vec();

    SimpleGeometry::create_polygon(transformed_exterior, transformed_interior_rings)
}

fn transform_linear_ring<'c>(
    context: &'c SimpleContextHandle,
    transform: &CoordTransform,
    ring: &SimpleGeometry) -> Result<SimpleGeometry<'c>>
{
    assert_eq!(ring.geometry_type(), GeometryTypes::LinearRing);

    let cs = ring.get_coord_sequence()?;

    let num_points = cs.num_points()?;

    let mut xs = Vec::with_capacity(num_points as _);
    let mut ys = Vec::with_capacity(num_points as _);
    let mut zs = Vec::with_capacity(num_points as _);

    for p in cs.points()?
    {
        xs.push(p[0]);
        ys.push(p[1]);
    }

    transform.transform_coords(&mut xs, &mut ys, &mut zs)?;

    let mut coord_seq = SimpleCoordinateSequence::new(num_points, &context)?;

    for i in 0..num_points {
        coord_seq.set_x(i, xs[i as usize])?;
        coord_seq.set_y(i, ys[i as usize])?;
    }

    SimpleGeometry::create_linear_ring(coord_seq)
}
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
use gdal::vector::{Geometry as GdalGeometry};
use geo::{MultiPoint, Point, Geometry,
          Coordinate, LineString, MultiLineString,
    Polygon, MultiPolygon, GeometryCollection
};
use crate::convert::traits::{ToRustGeo, ToGdal};

#[test]
fn test_import_export_point() {
    let wkt = "POINT (1 2)";
    let coord = Coordinate { x: 1., y: 2. };
    let geo = Geometry::Point(Point(coord));

    assert_eq!(
        GdalGeometry::from_wkt(wkt).unwrap().to_rust_geo(),
        geo
    );
    assert_eq!(geo.to_gdal().unwrap().wkt().unwrap(), wkt);
}

#[test]
fn test_import_export_multipoint() {
    let wkt = "MULTIPOINT (0 0,0 1,1 2)";
    let coord = vec![
        Point(Coordinate { x: 0., y: 0. }),
        Point(Coordinate { x: 0., y: 1. }),
        Point(Coordinate { x: 1., y: 2. }),
    ];
    let geo = Geometry::MultiPoint(MultiPoint(coord));

    assert_eq!(
        GdalGeometry::from_wkt(wkt).unwrap().to_rust_geo(),
        geo
    );
    assert_eq!(geo.to_gdal().unwrap().wkt().unwrap(), wkt);
}

#[test]
fn test_import_export_linestring() {
    let wkt = "LINESTRING (0 0,0 1,1 2)";
    let coord = vec![
        Coordinate { x: 0., y: 0. },
        Coordinate { x: 0., y: 1. },
        Coordinate { x: 1., y: 2. },
    ];
    let geo = Geometry::LineString(LineString(coord));

    assert_eq!(
        GdalGeometry::from_wkt(wkt).unwrap().to_rust_geo(),
        geo
    );
    assert_eq!(geo.to_gdal().unwrap().wkt().unwrap(), wkt);
}

#[test]
fn test_import_export_multilinestring() {
    let wkt = "MULTILINESTRING ((0 0,0 1,1 2),(3 3,3 4,4 5))";
    let strings = vec![
        LineString(vec![
            Coordinate { x: 0., y: 0. },
            Coordinate { x: 0., y: 1. },
            Coordinate { x: 1., y: 2. },
        ]),
        LineString(vec![
            Coordinate { x: 3., y: 3. },
            Coordinate { x: 3., y: 4. },
            Coordinate { x: 4., y: 5. },
        ]),
    ];
    let geo = Geometry::MultiLineString(MultiLineString(strings));

    assert_eq!(
        GdalGeometry::from_wkt(wkt).unwrap().to_rust_geo(),
        geo
    );
    assert_eq!(geo.to_gdal().unwrap().wkt().unwrap(), wkt);
}

fn square(x0: isize, y0: isize, x1: isize, y1: isize) -> LineString<f64> {
    LineString(vec![
        Coordinate {
            x: x0 as f64,
            y: y0 as f64,
        },
        Coordinate {
            x: x0 as f64,
            y: y1 as f64,
        },
        Coordinate {
            x: x1 as f64,
            y: y1 as f64,
        },
        Coordinate {
            x: x1 as f64,
            y: y0 as f64,
        },
        Coordinate {
            x: x0 as f64,
            y: y0 as f64,
        },
    ])
}

#[test]
fn test_import_export_polygon() {
    let wkt = "POLYGON ((0 0,0 5,5 5,5 0,0 0),\
               (1 1,1 2,2 2,2 1,1 1),\
               (3 3,3 4,4 4,4 3,3 3))";
    let outer = square(0, 0, 5, 5);
    let holes = vec![square(1, 1, 2, 2), square(3, 3, 4, 4)];
    let geo = Geometry::Polygon(Polygon::new(outer, holes));

    assert_eq!(
        GdalGeometry::from_wkt(wkt).unwrap().to_rust_geo(),
        geo
    );
    assert_eq!(geo.to_gdal().unwrap().wkt().unwrap(), wkt);
}

#[test]
fn test_import_export_multipolygon() {
    let wkt = "MULTIPOLYGON (\
               ((0 0,0 5,5 5,5 0,0 0),\
               (1 1,1 2,2 2,2 1,1 1),\
               (3 3,3 4,4 4,4 3,3 3)),\
               ((4 4,4 9,9 9,9 4,4 4),\
               (5 5,5 6,6 6,6 5,5 5),\
               (7 7,7 8,8 8,8 7,7 7))\
               )";
    let multipolygon = MultiPolygon(vec![
        Polygon::new(
            square(0, 0, 5, 5),
            vec![square(1, 1, 2, 2), square(3, 3, 4, 4)],
        ),
        Polygon::new(
            square(4, 4, 9, 9),
            vec![square(5, 5, 6, 6), square(7, 7, 8, 8)],
        ),
    ]);
    let geo = Geometry::MultiPolygon(multipolygon);

    assert_eq!(
        GdalGeometry::from_wkt(wkt).unwrap().to_rust_geo(),
        geo
    );
    assert_eq!(geo.to_gdal().unwrap().wkt().unwrap(), wkt);
}

#[test]
fn test_import_export_geometrycollection() {
    let wkt = "GEOMETRYCOLLECTION (POINT (1 2),LINESTRING (0 0,0 1,1 2))";
    let coord = Coordinate { x: 1., y: 2. };
    let point = Geometry::Point(Point(coord));
    let coords = vec![
        Coordinate { x: 0., y: 0. },
        Coordinate { x: 0., y: 1. },
        Coordinate { x: 1., y: 2. },
    ];
    let linestring = Geometry::LineString(LineString(coords));
    let collection = GeometryCollection(vec![point, linestring]);
    let geo = Geometry::GeometryCollection(collection);

    assert_eq!(
        GdalGeometry::from_wkt(wkt).unwrap().to_rust_geo(),
        geo
    );
    assert_eq!(geo.to_gdal().unwrap().wkt().unwrap(), wkt);
}

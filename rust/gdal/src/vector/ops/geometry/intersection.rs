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
use crate::vector::Geometry;
use gdal_sys::OGR_G_Intersection;

/// An intersection between Geometry/Geometry returning the same type.
pub trait Intersection
where
    Self: Sized,
{
    /// Compute intersection.
    ///
    /// Generates a new geometry which is the region of intersection of
    /// the two geometries operated on. Call intersects (Not yet implemented)
    /// to check if there is a region of intersection.
    /// Geometry validity is not checked. In case you are unsure of the
    /// validity of the input geometries, call IsValid() before,
    /// otherwise the result might be wrong.
    ///
    /// # Returns
    /// Some(Geometry) if both Geometries contain pointers
    /// None if either geometry is missing the gdal pointer, or there is an error.
    fn intersection(&self, other: &Self) -> Option<Self>;
}

impl Intersection for Geometry {
    fn intersection(&self, other: &Self) -> Option<Self> {

        unsafe {
            let ogr_geom = OGR_G_Intersection(self.c_geometry, other.c_geometry);
            if ogr_geom.is_null() {
                return None;
            }
            Some(Geometry::with_c_geometry(ogr_geom, true))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gdal_sys::OGRwkbGeometryType;

    #[test]
    fn test_intersection_success() {
        let geom =
            Geometry::from_wkt("POLYGON ((0.0 10.0, 0.0 0.0, 10.0 0.0, 10.0 10.0, 0.0 10.0))")
                .unwrap();
        let other =
            Geometry::from_wkt("POLYGON ((0.0 5.0, 0.0 0.0, 5.0 0.0, 5.0 5.0, 0.0 5.0))").unwrap();

        let inter = geom.intersection(&other);

        assert!(inter.is_some());

        let inter = inter.unwrap();

        assert_eq!(inter.area(), 25.0);
    }

    #[test]
    fn test_intersection_no_gdal_ptr() {
        let geom =
            Geometry::from_wkt("POLYGON ((0.0 10.0, 0.0 0.0, 10.0 0.0, 10.0 10.0, 0.0 10.0))")
                .unwrap();
        let other = Geometry::empty(OGRwkbGeometryType::wkbPoint).unwrap();

        let inter = geom.intersection(&other);

        assert!(inter.is_some());

        assert_eq!(inter.unwrap().area(), 0.0);
    }

    #[test]
    fn test_intersection_no_intersects() {
        let geom =
            Geometry::from_wkt("POLYGON ((0.0 5.0, 0.0 0.0, 5.0 0.0, 5.0 5.0, 0.0 5.0))").unwrap();

        let other =
            Geometry::from_wkt("POLYGON ((15.0 15.0, 15.0 20.0, 20.0 20.0, 20.0 15.0, 15.0 15.0))")
                .unwrap();

        let inter = geom.intersection(&other);

        assert!(inter.is_some());

        assert_eq!(inter.unwrap().area(), 0.0);
    }
}

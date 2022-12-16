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
use geos_sys::*;
use crate::SimpleContextHandle;
use ::{GeometryTypes, SimpleCoordinateSequence};
use anyhow::{bail, Result};
use simple_string::simple_managed_string;
use ByteOrder;
use c_vec::CVec;

pub struct SimpleGeometry<'c>
{
    pub(crate) c_handle: *mut GEOSGeometry,
    //is false always ConstGeosGeometry?
    pub(crate) owned: bool,
    pub(crate) context_handle: &'c SimpleContextHandle
}

impl <'c> SimpleGeometry<'c> {
    pub fn create_point_xy(context: &'c SimpleContextHandle, x: f64, y: f64) -> Result<Self>
    {
        unsafe {
            let ptr = GEOSGeom_createPointFromXY_r(context.c_handle, x, y);

            if ptr.is_null() {
                bail!("GEOSGeom_createPointFromXY_r");
            }

            Ok(SimpleGeometry {
                c_handle: ptr,
                owned: true,
                context_handle: context
            })
        }
    }

    pub fn create_multi_geom(
        context: &'c SimpleContextHandle,
        mut geoms: Vec<SimpleGeometry<'c>>,
        output_type: GeometryTypes,
    ) -> Result<SimpleGeometry<'c>> {
        let nb_geoms = geoms.len();

        assert!(
            output_type == GeometryTypes::GeometryCollection ||
            output_type == GeometryTypes::MultiPoint ||
            output_type == GeometryTypes::MultiPolygon ||
            output_type == GeometryTypes::MultiLineString
        );

        //also remove ownership
        let mut geoms: Vec<*mut GEOSGeometry> = geoms.iter_mut().map(|g| {
            g.owned = false;
            g.c_handle
        }).collect();
        unsafe {
            //from the geos_c comment, we no longer delete the geometries in vec, but they will be deleted as part of the
            //returned geometry

            let ptr = GEOSGeom_createCollection_r(
                context.c_handle,
                output_type.into(),
                geoms.as_mut_ptr() as *mut *mut GEOSGeometry,
                nb_geoms as _,
            );

            if ptr.is_null() {
                bail!("GEOSGeom_createCollection_r");
            }

            //println!("Created multi geom: {:?} of {:?}", ptr, output_type);

            Ok(SimpleGeometry {
                c_handle: ptr,
                owned: true,
                context_handle: context
            })
        }
    }

    pub fn create_line_string(mut s: SimpleCoordinateSequence<'c>) -> Result<SimpleGeometry<'c>> {
        unsafe {
            let ptr = GEOSGeom_createLineString_r(s.context_handle.c_handle, s.c_handle);
            if ptr.is_null() {
                bail!("GEOSGeom_createLineString_r");
            }
            //coord sequence now owned by line string
            s.owned = false;

            Ok(SimpleGeometry {
                c_handle: ptr,
                owned: true,
                context_handle: s.context_handle
            })
        }
    }

    pub fn create_linear_ring(mut s: SimpleCoordinateSequence<'c>) -> Result<SimpleGeometry<'c>> {
        unsafe {
            let ptr = GEOSGeom_createLinearRing_r(s.context_handle.c_handle, s.c_handle);
            if ptr.is_null() {
                bail!("GEOSGeom_createLineString_r");
            }
            //coord sequence now owned by line string
            s.owned = false;

            //println!("Created linear ring: {:?}", ptr);

            Ok(SimpleGeometry {
                c_handle: ptr,
                owned: true,
                context_handle: s.context_handle
            })
        }
    }

    pub fn create_polygon(
        mut exterior: SimpleGeometry<'c>,
        mut interiors: Vec<SimpleGeometry<'c>>,
    ) -> Result<SimpleGeometry<'c>> {

        if exterior.geometry_type() != GeometryTypes::LinearRing {
            bail!(
                "exterior must be a LinearRing",
            );
        }

        //ownership transferred to polygon
        exterior.owned = false;

        let nb_interiors = interiors.len();
        let ptr = unsafe {
            let mut geoms: Vec<*mut GEOSGeometry> =
                interiors.iter_mut().map(|g| {
                    //ownership transferred
                    g.owned = false;
                    g.c_handle
                }).collect();
            GEOSGeom_createPolygon_r(
                exterior.context_handle.c_handle,
                exterior.c_handle,
                geoms.as_mut_ptr() as *mut *mut GEOSGeometry,
                nb_interiors as _,
            )
        };

        //println!("Created polygon: {:?}", ptr);

        Ok(SimpleGeometry{
            c_handle: ptr,
            owned: true,
            context_handle: exterior.context_handle
        })
    }

    /// Returns a geometry collection of polygons
    pub fn voronoi(
        &self,
        envelope: Option<&SimpleGeometry<'c>>,
        tolerance: f64,
        only_edges: bool,
    ) -> Result<SimpleGeometry<'c>> {
        unsafe {
            let raw_voronoi = GEOSVoronoiDiagram_r(
                self.context_handle.c_handle,
                self.c_handle,
                envelope
                    .map(|e| e.c_handle)
                    .unwrap_or(std::ptr::null_mut()),
                tolerance,
                only_edges as _,
            );

            if raw_voronoi.is_null() {
                bail!("Voronoi failed");
            }

            println!("Created Voronoi {:?}", raw_voronoi);

            Ok(Self {
                c_handle: raw_voronoi,
                owned: true,
                context_handle: self.context_handle
            })
        }
    }

    pub fn envelope(&self) -> Result<SimpleGeometry<'c>> {
        unsafe {
            let ptr = GEOSEnvelope_r(self.context_handle.c_handle, self.c_handle);

            if ptr.is_null() {
                bail!("GEOSEnvelope_r");
            }

            Ok(SimpleGeometry {
                c_handle: ptr,
                owned: true,
                context_handle: self.context_handle
            })
        }
    }

    //Must call envelope first
    pub fn bbox(&self) -> Result< [f64;4] >
    {
        let exterior_ring = self.get_exterior_ring()?;
        let coord_seq = exterior_ring.get_coord_sequence()?;
        //the envelope has a specific order
        let x_min = coord_seq.get_x(0)?;
        let x_max = coord_seq.get_x(2)?;
        let y_min = coord_seq.get_y(0)?;
        let y_max = coord_seq.get_y(2)?;

        return Ok([x_min, y_min, x_max, y_max]);
    }

    pub fn center(&self) -> Result< (f64, f64) > {
        unsafe {
            let mut x_min: f64 = 0.0;
            let mut x_max: f64 = 0.0;
            let mut y_min: f64 = 0.0;
            let mut y_max: f64 = 0.0;

            // maybe use GEOSEnvelope ?
            let mut status = GEOSGeom_getXMin_r(self.context_handle.c_handle, self.c_handle, &mut x_min);
            if status < 0 {
                bail!("Error");
            }
            status = GEOSGeom_getXMax_r(self.context_handle.c_handle, self.c_handle, &mut x_max);
            if status < 0 {
                bail!("Error");
            }
            status = GEOSGeom_getYMin_r(self.context_handle.c_handle, self.c_handle, &mut y_min);
            if status < 0 {
                bail!("Error");
            }
            status = GEOSGeom_getYMax_r(self.context_handle.c_handle, self.c_handle, &mut y_max);
            if status < 0 {
                bail!("Error");
            }

            let x_center = (x_max + x_min ) / 2.0;
            let y_center  = (y_max + y_min ) / 2.0;

            return Ok((x_center, y_center));
        }
    }

    pub fn get_xy(&self) -> Result< (f64, f64) > {
        unsafe {
            let mut x: f64 = 0.0;
            let mut y: f64 = 0.0;
            let mut status = GEOSGeomGetX_r(self.context_handle.c_handle, self.c_handle, &mut x);
            if status < 0 {
                bail!("Error");
            }

            status = GEOSGeomGetY_r(self.context_handle.c_handle, self.c_handle, &mut y);
            if status < 0 {
                bail!("Error");
            }

            return Ok((x, y));
        }
    }

    pub fn centroid(&self) -> Result<Self> {
        unsafe {

            let centroid_ptr = GEOSGetCentroid_r(self.context_handle.c_handle, self.c_handle);

            Ok(SimpleGeometry {
                c_handle: centroid_ptr,
                owned: true,
                context_handle: self.context_handle
            })
        }
    }

    pub fn convex_hull(&self) -> Result<SimpleGeometry<'c>> {
        let c_geom = unsafe { GEOSConvexHull_r(
            self.context_handle.c_handle,
            self.c_handle) };
        if c_geom.is_null() {
            bail!("GEOSConvexHull_r");
        };

        Ok(SimpleGeometry {
                c_handle: c_geom,
                owned: true,
                context_handle: self.context_handle
            })
    }

    pub fn get_num_geometries(&self) -> Result<usize> {
        unsafe {
            let ret = GEOSGetNumGeometries_r(self.context_handle.c_handle, self.c_handle);
            if ret < 1 {
                bail!("GEOSGetNumGeometries_r failed");
            } else {
                Ok(ret as _)
            }
        }
    }

    pub fn get_geometry_n<'d>(&self, n: usize) -> Result<SimpleGeometry<'d>>
    // sub geometry lifetime is shorter than geometry
    where 'c: 'd
    {
        unsafe {
            let ptr = GEOSGetGeometryN_r(self.context_handle.c_handle, self.c_handle, n as _);

            if ptr.is_null() {
                bail!("GEOSGetGeometryN_r");
            }

            Ok(SimpleGeometry {
                c_handle: ptr as *mut GEOSGeometry,
                //internal pointer so don't destory it
                owned: false,
                context_handle: self.context_handle
            })
        }
    }

    pub fn get_type(&self) -> Result<String> {
        unsafe {
            let ptr = GEOSGeomType_r(self.context_handle.c_handle, self.c_handle);
            simple_managed_string(ptr, self.context_handle)
        }
    }

    pub fn geometry_type(&self) -> GeometryTypes {
        let type_geom = unsafe { GEOSGeomTypeId_r(self.context_handle.c_handle, self.c_handle) as i32 };

        GeometryTypes::from(type_geom)
    }

    pub fn get_num_interior_rings(&self) -> Result<usize> {
        unsafe {
            let ret = GEOSGetNumInteriorRings_r(self.context_handle.c_handle, self.c_handle);
            if ret == -1 {
                bail!("GEOSGetNumInteriorRings_r failed");
            } else {
                Ok(ret as _)
            }
        }
    }

    pub fn get_interior_ring_n(&self, n: u32) -> Result<SimpleGeometry<'c>> {
        unsafe {
            let ptr = GEOSGetInteriorRingN_r(self.context_handle.c_handle, self.c_handle, n as _);
            if ptr.is_null() {
                bail!("GEOSGetInteriorRingN_r");
            }
            Ok(SimpleGeometry {
                c_handle: ptr as *mut GEOSGeometry,
                //internal pointer so don't destroy it
                owned: false,
                context_handle: self.context_handle
            })
        }
    }

    pub fn get_exterior_ring(&self) -> Result<SimpleGeometry<'c>> {
        unsafe {
            let ptr = GEOSGetExteriorRing_r(self.context_handle.c_handle, self.c_handle);
            if ptr.is_null() {
                bail!("GEOSGetExteriorRing_r");
            }
            Ok(SimpleGeometry {
                c_handle: ptr as *mut GEOSGeometry,
                //internal pointer so don't destroy it
                owned: false,
                context_handle: self.context_handle
            })
        }
    }

    pub fn get_coord_sequence(&self) -> Result<SimpleCoordinateSequence> {
        unsafe {
            let ptr = GEOSGeom_getCoordSeq_r(self.context_handle.c_handle, self.c_handle);
            if ptr.is_null() {
                bail!("Must be a Linestring, LinearRing or Point but was {:?}.  Ptr={:?}",
                    self.get_type(),
                    self.c_handle
                );
            }
            Ok(SimpleCoordinateSequence {
                c_handle: ptr as *mut GEOSCoordSequence,
                //internal pointer so don't destroy it
                owned: false,
                context_handle: self.context_handle
            })
        }
    }

    pub fn has_holes(&self) -> bool {
        match self.geometry_type() {
            GeometryTypes::Polygon => {
                let n_interior = self.get_num_interior_rings().unwrap();
                n_interior > 0
            }
            GeometryTypes::MultiPolygon => {
                let mut r = false;
                let num_geom = self.get_num_geometries().unwrap();
                for p in 0..num_geom {
                    if self.get_geometry_n(p).unwrap().has_holes() {
                        r = true;
                        break;
                    }
                }
                r
            }
            _ => false
        }
    }

    pub fn remove_holes(&self, context: &'c SimpleContextHandle) -> Result<SimpleGeometry<'c>>
    {

        Ok(match self.geometry_type() {
            GeometryTypes::Polygon => {
                let exterior = self.get_exterior_ring()?.clone(context)?;
                SimpleGeometry::create_polygon(exterior, vec![])?
            }
            GeometryTypes::MultiPolygon => {
                let poly_count = self.get_num_geometries()?;
                let polygons = (0..poly_count)
                    .map(|n| self.get_geometry_n(n).unwrap().remove_holes(context).unwrap() )
                    .collect();
                SimpleGeometry::create_multi_geom(context, polygons, GeometryTypes::MultiPolygon)?
            }
            _ => bail!("Not a multipolygon nor polygon")
        })
    }

    pub fn union(&self, context: &'c SimpleContextHandle, rhs: &SimpleGeometry) -> Result<SimpleGeometry<'c>>
    {
        unsafe {
            let ptr = GEOSUnion_r(
                context.c_handle,
                self.c_handle,
                rhs.c_handle
            );

            if ptr.is_null() {
                bail!("GEOSUnion_r exception");
            }

            Ok(SimpleGeometry{
                c_handle: ptr,
                owned: true,
                context_handle: context
            })
        }
    }

    pub fn union_unary(&self, context: &'c SimpleContextHandle) -> Result<SimpleGeometry<'c>>
    {
        unsafe {
            let ptr = GEOSUnaryUnion_r(
                context.c_handle,
                self.c_handle,
            );

            if ptr.is_null() {
                bail!("GEOSUnaryUnion_r exception");
            }

            Ok(SimpleGeometry{
                c_handle: ptr,
                owned: true,
                context_handle: context
            })
        }
    }

    pub fn difference(&self, context: &'c SimpleContextHandle, rhs: &SimpleGeometry) -> Result<SimpleGeometry<'c>>
    {
        unsafe {
            let ptr = GEOSDifference_r(
                context.c_handle,
                self.c_handle,
                rhs.c_handle
            );

            if ptr.is_null() {
                bail!("GEOSDifference_r exception");
            }

            Ok(SimpleGeometry{
                c_handle: ptr,
                owned: true,
                context_handle: context
            })
        }
    }

    pub fn intersection(&self, context: &'c SimpleContextHandle, rhs: &SimpleGeometry) -> Result<SimpleGeometry<'c>>
    {
        unsafe {
            let ptr = GEOSIntersection_r(
                context.c_handle,
                self.c_handle,
                rhs.c_handle
            );

            if ptr.is_null() {
                bail!("GEOSIntersection_r exception");
            }

            Ok(SimpleGeometry{
                c_handle: ptr,
                owned: true,
                context_handle: context
            })
        }
    }

    pub fn intersects(&self, rhs: &SimpleGeometry) -> Result<bool>
    {
        unsafe {
            let r = GEOSIntersects_r(
                self.context_handle.c_handle,
                self.c_handle,
                rhs.c_handle
            );


            return match r {
                1 => Ok(true),
                0 => Ok(false),
                _ => bail!("intersects return invalid")
            }
        }

    }

    /// quadsegs is how many lines per quater circle -- 8 is a good start
    /// Use a new lifetime since the returned geometry depends on the passed in context lifetime
    pub fn buffer<'d>(&self, context: &'d SimpleContextHandle,
                  width: f64, quadsegs: i32) -> Result<SimpleGeometry<'d>> {
        assert!(quadsegs > 0);
        unsafe {
            let ptr = GEOSBuffer_r(
                context.c_handle,
                self.c_handle,
                width,
                quadsegs as _,
            );
            if ptr.is_null() {
                bail!("GEOSBuffer_r");
            }
            Ok(SimpleGeometry {
                c_handle: ptr,
                owned: true,
                context_handle: context
            })
        }
    }

    pub fn simplify<'d>(&self, context: &'d SimpleContextHandle,
                  tolerance: f64,
    preserve_topology: bool) -> Result<SimpleGeometry<'d>> {
        
        unsafe {
            let ptr = if preserve_topology {
              GEOSTopologyPreserveSimplify_r(
                    context.c_handle,
                    self.c_handle,
                    tolerance,
                )
            } else {
                GEOSSimplify_r(
                    context.c_handle,
                    self.c_handle,
                    tolerance,
                )
            };

            if ptr.is_null() {
                bail!("GEOSBuffer_r");
            }
            Ok(SimpleGeometry {
                c_handle: ptr,
                owned: true,
                context_handle: context
            })
        }
    }

    pub fn contains(&self, rhs: &SimpleGeometry) -> Result<bool>
    {
        unsafe {
            let c = GEOSContains_r(self.context_handle.c_handle,
                                   self.c_handle,
            rhs.c_handle
            );

            if c == 1 {
                return Ok(true);
            }
            if c == 0 {
                return Ok(false);
            }

            bail!("GEOSContains_r exception: {}", c);
        }
    }

    pub fn create_empty_collection(context: &'c SimpleContextHandle, geom_type: GeometryTypes) -> Result<Self> {
        match geom_type {
            GeometryTypes::GeometryCollection
            | GeometryTypes::MultiPoint
            | GeometryTypes::MultiLineString
            | GeometryTypes::MultiPolygon => {}
            _ => bail!("Invalid geometry type"),
        }
        unsafe {
            let ptr = GEOSGeom_createEmptyCollection_r(
                context.c_handle, geom_type.into());

            if ptr.is_null() {
                bail!("GEOSGeom_createEmptyCollection_r failed");
            }

            Ok(SimpleGeometry {
                c_handle: ptr,
                owned: true,
                context_handle: context
            })
        }
    }

    /// Convert to PostGIS EWKB format
    pub fn ewkb(&self) -> Result<CVec<u8>>
    {
        unsafe {
            //let ewkb = OGRGeometryToHexEWKB(self.c_geometry, srid, 3, 0);
            let writer_ptr = GEOSWKBWriter_create_r(self.context_handle.c_handle);


            GEOSWKBWriter_setIncludeSRID_r(self.context_handle.c_handle,
            writer_ptr, 1);

            //little endian
            GEOSWKBWriter_setByteOrder_r(self.context_handle.c_handle, writer_ptr, ByteOrder::LittleEndian.into());

            let mut size = 0;
            let w_ptr = GEOSWKBWriter_write_r(self.context_handle.c_handle,
            writer_ptr, self.c_handle, &mut size);

            //assume the CVec will be destoryed before the context handle...
            let c_context_handle = self.context_handle.c_handle;

            //Helps us avoid a copy
            let c_vec = CVec::new_with_dtor(w_ptr,
            size, move |c_vet_ptr| {
                   //println!("Running v cet destructor");
                   GEOSFree_r(c_context_handle, c_vet_ptr as _);
                });

            GEOSWKBWriter_destroy_r(self.context_handle.c_handle, writer_ptr);

            Ok(c_vec)
        }
    }

    pub fn to_wkt(&self) -> Result<String> {
        unsafe {
            let writer = GEOSWKTWriter_create_r(self.context_handle.c_handle);
            let c_result = GEOSWKTWriter_write_r(self.context_handle.c_handle, writer, self.c_handle);
            GEOSWKTWriter_destroy_r(self.context_handle.c_handle, writer);
            simple_managed_string(c_result, self.context_handle)
        }
    }

    pub fn to_wkt_precision(&self, precision: u32) -> Result<String> {
        unsafe {
            let writer = GEOSWKTWriter_create_r(self.context_handle.c_handle);
            GEOSWKTWriter_setRoundingPrecision_r(
                self.context_handle.c_handle,
                writer,
                precision as _,
            );
            let c_result = GEOSWKTWriter_write_r(self.context_handle.c_handle, writer, self.c_handle);
            GEOSWKTWriter_destroy_r(self.context_handle.c_handle, writer);
            simple_managed_string(c_result, self.context_handle)
        }
    }

    pub fn area(&self) -> Result<f64> {
        unsafe {
            let mut n = 0.;
            let ok = GEOSArea_r(self.context_handle.c_handle, self.c_handle,&mut n);
            if ok == 1 {
                return Ok(n);
            } else {
                bail!("Problem with GEOSArea_r");
            }
        }
    }

    pub fn is_valid(&self) -> bool {
        unsafe { GEOSisValid_r(self.context_handle.c_handle,
                               self.c_handle) == 1 }
    }

    pub fn make_valid(&self, context_handle: &'c SimpleContextHandle) -> Result<SimpleGeometry<'c>>
    {
        unsafe {

            let ptr = GEOSMakeValid_r(self.context_handle.c_handle,
            self.c_handle);

            if ptr.is_null() {
                bail!("GEOSMakeValid_r");
            }

            Ok(SimpleGeometry{
                c_handle: ptr,
                owned: true,
                context_handle
            })
        }
    }
    //

    pub fn set_srid(&self, srid: i32) {
        assert!(self.owned);

        unsafe {
            GEOSSetSRID_r(self.context_handle.c_handle, self.c_handle, srid)
        }
    }

    pub fn get_srid(&self) -> i32 {
        unsafe {
            GEOSGetSRID_r(self.context_handle.c_handle, self.c_handle)
        }
    }

    pub fn set_precision(&self, context_handle: &'c SimpleContextHandle,
                         grid_size: f64) -> Result<SimpleGeometry<'c>> {
        unsafe {
            let ptr = GEOSGeom_setPrecision_r(self.context_handle.c_handle,
            self.c_handle, grid_size, 0);

            if ptr.is_null() {
                bail!("GEOSGeom_setPrecision_r");
            }

            Ok(SimpleGeometry{
                c_handle: ptr,
                owned: true,
                context_handle
            })
        }
    }

    pub fn get_precision(&self) -> f64 {
        unsafe {
            GEOSGeom_getPrecision_r(self.context_handle.c_handle,
            self.c_handle)
        }
    }

    pub fn get_largest_polygon<'d>(&self) -> Result<SimpleGeometry<'d>>
    // sub geometry lifetime 'd is shorter than geometry
    //Note it is still possible to have the self geometry be dropped while we refer to the return value
    //Ideally for safety we would need a new wrapper that has a reference to the parent simple geometry
    where 'c: 'd
    {
        assert_eq!(self.geometry_type(), GeometryTypes::MultiPolygon);

        let num_geometries = self.get_num_geometries()?;

        let mut biggest_geom = self.get_geometry_n(0)?;
        let mut largest_area = biggest_geom.area()?;
        for g_idx in 1..num_geometries {
            let g = self.get_geometry_n(g_idx)?;

            let g_area = g.area()?;

            if g_area > largest_area {
                largest_area = g_area;
                biggest_geom = g;
            }
        }

        Ok(biggest_geom)

    }

    pub fn clone(&self, context_handle: &'c SimpleContextHandle) -> Result<SimpleGeometry<'c>>
    {
        unsafe {
            let c = GEOSGeom_clone_r(context_handle.c_handle, self.c_handle);
            if c.is_null() {
                bail!("clone");
            }

            Ok(SimpleGeometry{
                c_handle: c,
                owned: true,
                context_handle
            })
        }
    }

    pub fn polygon_to_multipolygon(&self, context_handle: &'c SimpleContextHandle) -> Result<SimpleGeometry<'c>> {

        unsafe {

            assert_eq!(self.geometry_type(), GeometryTypes::Polygon);

            let mut vec_polygons = Vec::new();


            //ownership will be transferred to the new multipolygon
            //this is also why we must clone, when the previous collection is destroyed
            let c = GEOSGeom_clone_r(context_handle.c_handle, self.c_handle);
            assert!(!c.is_null());
            vec_polygons.push(c );

            let mp = GEOSGeom_createCollection_r(
                context_handle.c_handle,
                GeometryTypes::MultiPolygon.into(),
                vec_polygons.as_mut_ptr() as *mut *mut GEOSGeometry,
                vec_polygons.len() as _
            );

            Ok(SimpleGeometry {
                c_handle: mp,
                owned: true,
                context_handle
            })
        }
    }

    pub fn geometry_collection_to_multipolygon(&self, context_handle: &'c SimpleContextHandle) -> Result<SimpleGeometry<'c>> {

        unsafe {

            assert_eq!(self.geometry_type(), GeometryTypes::GeometryCollection);

            let mut vec_polygons = Vec::new();
            let num_geometries = self.get_num_geometries()?;

            for i in 0..num_geometries {
                let g = self.get_geometry_n(i)?;

                match g.geometry_type() {
                    GeometryTypes::Polygon => {
                        //ownership will be transferred to the new multipolygon
                        //this is also why we must clone, when the previous collection is destroyed
                        let c = GEOSGeom_clone_r(context_handle.c_handle, g.c_handle);
                        assert!(!c.is_null());
                        vec_polygons.push(c );
                    },
                    GeometryTypes::LineString => {
                        //drop/ignore it
                    },
                    other => {
                        bail!("Unexpected geometry type: {:?}", other);
                    }
                }
            }

            assert!(!vec_polygons.is_empty());

            let mp = GEOSGeom_createCollection_r(
                context_handle.c_handle,
                GeometryTypes::MultiPolygon.into(),
                vec_polygons.as_mut_ptr() as *mut *mut GEOSGeometry,
                vec_polygons.len() as _
            );

            Ok(SimpleGeometry {
                c_handle: mp,
                owned: true,
                context_handle
            })
        }
    }
}

impl <'c> Drop for SimpleGeometry<'c> {
    fn drop(&mut self) {
        unsafe {
            if self.owned {
                //println!("GEOS: Dropping simple geometry: {:?} with context {:?}", self.c_handle, self.context_handle.c_handle);
                GEOSGeom_destroy_r(self.context_handle.c_handle, self.c_handle);
            }
        }
    }
}


//Conversions from
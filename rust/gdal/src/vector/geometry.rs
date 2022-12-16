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
use crate::spatial_ref::{CoordTransform, SpatialRef};
use crate::utils::{_last_null_pointer_err, _string};
use gdal_sys::{self, OGRErr, OGRGeometryH, OGRwkbGeometryType, OGR_G_WkbSize, OGR_G_ExportToWkb, OGRwkbByteOrder, OGREnvelope, OGR_G_ImportFromWkb};
use libc::{c_double, c_int, c_void};
use std::ffi::{CString};
use std::ptr::null_mut;

use crate::errors::*;
use anyhow::Result;
use crate::vector::{Feature};
use std::{ptr, slice};





/// OGR Geometry
pub struct Geometry {
    //move semantics so having a pointer is fine, cannot copy / clone
    //and its private
    pub(crate) c_geometry: OGRGeometryH,

    //when owned=false, this either means we surrendered ownership to GDAL; or this
    //is a temporary object from a FeatureGeometry.  That is done to not implement everything
    //twice...
    pub(crate) owned: bool,
}

impl Geometry {



    pub(crate) unsafe fn with_c_geometry(c_geom: OGRGeometryH, owned: bool) -> Geometry {
        Geometry {
            c_geometry: c_geom,
            owned,
        }
    }

    pub fn empty(wkb_type: OGRwkbGeometryType::Type) -> Result<Geometry> {
        let c_geom = unsafe { gdal_sys::OGR_G_CreateGeometry(wkb_type) };
        if c_geom.is_null() {
            Err(_last_null_pointer_err("OGR_G_CreateGeometry"))?;
        };
        Ok(unsafe { Geometry::with_c_geometry(c_geom, true) })
    }

    pub fn is_empty(&self) -> bool {
        unsafe { gdal_sys::OGR_G_IsEmpty(self.c_geometry) == 1 }
    }

    pub fn is_valid(&self) -> bool {
        unsafe { gdal_sys::OGR_G_IsValid(self.c_geometry) == 1 }
    }

    /// Create a geometry by parsing a
    /// [WKT](https://en.wikipedia.org/wiki/Well-known_text) string.
    pub fn from_wkt(wkt: &str) -> Result<Geometry> {
        let c_wkt = CString::new(wkt)?;
        let mut c_wkt_ptr = c_wkt.into_raw();
        let mut c_geom = null_mut();
        let rv = unsafe { gdal_sys::OGR_G_CreateFromWkt(&mut c_wkt_ptr, null_mut(), &mut c_geom) };
        if rv != OGRErr::OGRERR_NONE {
            Err(ErrorKind::OgrError {
                err: rv,
                method_name: "OGR_G_CreateFromWkt",
            })?;
        }
        //we need to free this
        Ok(unsafe { Geometry::with_c_geometry(c_geom, true) })
    }

    pub fn from_x_y(x: f64, y:f64) -> Result<Geometry> {
        //let mut c_geom = null_mut();

        let mut geom = Geometry::empty(OGRwkbGeometryType::wkbPoint)?;

        geom.set_point_2d(0, (x,y));

        //let mut point = unsafe { gdal_sys::OGRPoint::new1(x, y) };
        //let c_geom: *mut c_void = &mut point as *mut _ as *mut c_void;
        /*if rv != OGRErr::OGRERR_NONE {
            Err(ErrorKind::OgrError {
                err: rv,
                method_name: "OGRPoint_OGRPoint2",
            })?;
        }*/
        Ok(geom)
    }

    /// Create a rectangular geometry from West, South, East and North values.
    pub fn bbox(w: f64, s: f64, e: f64, n: f64) -> Result<Geometry> {
        Geometry::from_wkt(&format!(
            "POLYGON (({} {}, {} {}, {} {}, {} {}, {} {}))",
            w, n, e, n, e, s, w, s, w, n,
        ))
    }

    pub fn bbox_from_env(env: &OGREnvelope) -> Result<Geometry> {
        Geometry::bbox(env.MinX, env.MinY, env.MaxX, env.MaxY)
    }

    pub fn buffer(&self, buffer_size: f64, n_quads: i32 ) -> Result<Geometry> {

        let c_geom = unsafe { gdal_sys::OGR_G_Buffer(self.c_geometry, buffer_size, n_quads) };
        if c_geom.is_null() {
            Err(_last_null_pointer_err("OGR_G_Buffer"))?;
        };
        Ok(unsafe { Geometry::with_c_geometry(c_geom, true) })
    }

    /// Serialize the geometry as JSON.
    pub fn json(&self) -> Result<String> {
        let c_json = unsafe { gdal_sys::OGR_G_ExportToJson(self.c_geometry) };
        if c_json.is_null() {
            Err(_last_null_pointer_err("OGR_G_ExportToJson"))?;
        };
        let rv = _string(c_json);
        unsafe { gdal_sys::VSIFree(c_json as *mut c_void) };
        Ok(rv)
    }

    /// Serialize the geometry as WKT.
    pub fn wkt(&self) -> Result<String> {
        let mut c_wkt = null_mut();
        let rv = unsafe { gdal_sys::OGR_G_ExportToWkt(self.c_geometry, &mut c_wkt) };
        if rv != OGRErr::OGRERR_NONE {
            Err(ErrorKind::OgrError {
                err: rv,
                method_name: "OGR_G_ExportToWkt",
            })?;
        }
        let wkt = _string(c_wkt);
        unsafe { gdal_sys::OGRFree(c_wkt as *mut c_void) };
        Ok(wkt)
    }

    pub fn geometry_name(&self) -> String {
        let rv = unsafe { gdal_sys::OGR_G_GetGeometryName(self.c_geometry ) };
        _string(rv)
    }

    //pub unsafe fn c_geometry(&self) -> OGRGeometryH {
     //   self.c_geometry
   // }



    pub fn set_point_2d(&mut self, i: usize, p: (f64, f64)) {
        let (x, y) = p;
        unsafe {
            gdal_sys::OGR_G_SetPoint_2D(self.c_geometry, i as c_int, x as c_double, y as c_double)
        };
    }

    pub fn get_point_xyz(&self, i: i32) -> (f64, f64, f64) {
        let mut x: c_double = 0.;
        let mut y: c_double = 0.;
        let mut z: c_double = 0.;
        unsafe { gdal_sys::OGR_G_GetPoint(self.c_geometry, i, &mut x, &mut y, &mut z) };
        (x as f64, y as f64, z as f64)
    }

    pub fn get_point(&self, i: i32) -> [f64; 2] {
        let mut x: c_double = 0.;
        let mut y: c_double = 0.;
        let mut z: c_double = 0.;
        unsafe { gdal_sys::OGR_G_GetPoint(self.c_geometry, i, &mut x, &mut y, &mut z) };
        [x as f64, y as f64]
    }

    pub fn get_point_vec(&self) -> Vec<[f64;2]> {
        let length = unsafe { gdal_sys::OGR_G_GetPointCount(self.c_geometry) };
        (0..length).map(|i| self.get_point(i)).collect()
    }

    pub fn get_point_iterator(&self) -> PointIterator {
        PointIterator::new(self.c_geometry)
    }

    /// Compute the convex hull of this geometry.
    pub fn convex_hull(&self) -> Result<Geometry> {
        let c_geom = unsafe { gdal_sys::OGR_G_ConvexHull(self.c_geometry) };
        if c_geom.is_null() {
            Err(_last_null_pointer_err("OGR_G_ConvexHull"))?;
        };
        Ok(unsafe { Geometry::with_c_geometry(c_geom, true) })
    }

    pub fn centroid(&self) -> Result<Geometry> {
        let c_point = Geometry::from_x_y(3., 4.)?;
        let rv = unsafe { gdal_sys::OGR_G_Centroid(self.c_geometry, c_point.c_geometry) } as u32;
        if rv != OGRErr::OGRERR_NONE {
            Err(ErrorKind::OgrError {
                err: rv,
                method_name: "OGR_G_Centroid",
            })?;
        }
        Ok(c_point)
    }

    /// minx miny maxx maxy
    pub fn envelope(&self) -> OGREnvelope {

        let mut e = OGREnvelope{
            MinX: 0.0,
            MaxX: 0.0,
            MinY: 0.0,
            MaxY: 0.0
        } ;

        unsafe { gdal_sys::OGR_G_GetEnvelope(self.c_geometry, &mut e) };

        e
    }

    pub fn intersects(&self, other_geom: &Self) -> bool {
        unsafe {
            let r = gdal_sys::OGR_G_Intersects(self.c_geometry, other_geom.c_geometry);
            return r == 1
        }
    }

    pub fn geometry_type(&self) -> OGRwkbGeometryType::Type {
        unsafe { gdal_sys::OGR_G_GetGeometryType(self.c_geometry) }
    }

    pub fn geometry_count(&self) -> usize {
        let cnt = unsafe { gdal_sys::OGR_G_GetGeometryCount(self.c_geometry) };
        cnt as usize
    }

    pub fn point_count(&self) -> usize {
        let cnt = unsafe { gdal_sys::OGR_G_GetPointCount(self.c_geometry) };
        cnt as usize
    }

    pub fn get_geometry(&self, n: usize) -> Geometry {
        // get the n-th sub-geometry as a non-owned Geometry; don't keep this
        // object for longer than the owning geometry.

        //Not really safe, but for convenience.  To be safe, we'd need to return a struct
        //with a reference to self to ensure this geometry outlived the reference to the sub geometry
        let rv = unsafe {
            let c_geom = gdal_sys::OGR_G_GetGeometryRef(self.c_geometry, n as c_int);
            Geometry::with_c_geometry(c_geom, false)
        };

        rv
    }

    pub fn get_linear_geometry(&self) -> Geometry {
        let rv = unsafe {
            let c_geom = gdal_sys::OGR_G_GetLinearGeometry(self.c_geometry, 0.0, null_mut());
            Geometry::with_c_geometry(c_geom, true)
        };
        rv
    }

    pub fn make_valid(&self) -> Geometry {
        let rv = unsafe {
            let c_geom = gdal_sys::OGR_G_MakeValid(self.c_geometry);
            Geometry::with_c_geometry(c_geom, true)
        };
        rv
    }

    pub fn to_multi_polygon(&mut self) -> Geometry {
        //Ownership is surrendered then passed back, so we must be owned
        assert!(self.owned);

        //since it is surrendered, we don't want to destroy it ourselves
        self.owned = false;

        let rv = unsafe {
            //println!("to_multi_polygon before geometry: {:?}", self.c_geometry);
            let c_geom = gdal_sys::OGR_G_ForceToMultiPolygon(self.c_geometry);
            //println!("to_multi_polygon after geometry: {:?}", c_geom);

            //Ownership is passed back to caller
            Geometry {
                c_geometry: c_geom,
                owned: true
            }
        };
        rv
    }

    pub fn is_owned(&self) -> bool {
        self.owned
    }

    pub fn remove_lower_dim_sub_geoms(&self) -> Geometry {
        let rv = unsafe {
            //always copies
            let c_geom = gdal_sys::OGR_G_RemoveLowerDimensionSubGeoms(self.c_geometry);
            Geometry::with_c_geometry(c_geom, true)
        };
        rv
    }

    pub fn has_curve_geometry(&self, look_for_non_linear: bool) -> bool {
        let i_look_for_non_linear = look_for_non_linear.into();
        let rv = unsafe {
            gdal_sys::OGR_G_HasCurveGeometry(self.c_geometry, i_look_for_non_linear)
        };
        rv == 1
    }

    pub fn add_geometry(&mut self, mut sub: Geometry) -> Result<()> {
        assert!(sub.owned);
        sub.owned = false;
        let rv =
            unsafe { gdal_sys::OGR_G_AddGeometryDirectly(self.c_geometry, sub.c_geometry) };
        if rv != OGRErr::OGRERR_NONE {
            Err(ErrorKind::OgrError {
                err: rv,
                method_name: "OGR_G_AddGeometryDirectly",
            })?;
        }
        Ok(())
    }

    pub fn add_point(&mut self, x: f64, y: f64) {
        unsafe {
            gdal_sys::OGR_G_AddPoint_2D(self.c_geometry, x, y);
        }
    }

    // Transform the geometry inplace (when we own the Geometry)
    pub fn transform_inplace(&mut self, htransform: &CoordTransform) -> Result<()> {
        assert!(self.owned);

        let rv = unsafe { gdal_sys::OGR_G_Transform(self.c_geometry, htransform.to_c_hct()) };
        if rv != OGRErr::OGRERR_NONE {
            Err(ErrorKind::OgrError {
                err: rv,
                method_name: "OGR_G_Transform",
            })?;
        }
        Ok(())
    }

    // Return a new transformed geometry (when the Geometry is owned by a Feature)
    pub fn transform(&self, htransform: &CoordTransform) -> Result<Geometry> {
        let new_c_geom = unsafe { gdal_sys::OGR_G_Clone(self.c_geometry) };
        let rv = unsafe { gdal_sys::OGR_G_Transform(new_c_geom, htransform.to_c_hct()) };
        if rv != OGRErr::OGRERR_NONE {
            Err(ErrorKind::OgrError {
                err: rv,
                method_name: "OGR_G_Transform",
            })?;
        }
        Ok(unsafe { Geometry::with_c_geometry(new_c_geom, true) })
    }

    pub fn transform_to_inplace(&self, spatial_ref: &SpatialRef) -> Result<()> {
        let rv = unsafe { gdal_sys::OGR_G_TransformTo(self.c_geometry, spatial_ref.c_spatial_ref) };
        if rv != OGRErr::OGRERR_NONE {
            Err(ErrorKind::OgrError {
                err: rv,
                method_name: "OGR_G_TransformTo",
            })?;
        }
        Ok(())
    }

    pub fn transform_to(&self, spatial_ref: &SpatialRef) -> Result<Geometry> {
        let new_c_geom = unsafe { gdal_sys::OGR_G_Clone(self.c_geometry) };
        let rv = unsafe { gdal_sys::OGR_G_TransformTo(new_c_geom, spatial_ref.c_spatial_ref) };
        if rv != OGRErr::OGRERR_NONE {
            Err(ErrorKind::OgrError {
                err: rv,
                method_name: "OGR_G_TransformTo",
            })?;
        }
        Ok(unsafe { Geometry::with_c_geometry(new_c_geom, true) })
    }

    pub fn area(&self) -> f64 {
        unsafe { gdal_sys::OGR_G_Area(self.c_geometry) }
    }

    /// May or may not contain a reference to a SpatialRef: if not, it returns
    /// an `Ok(None)`; if it does, it tries to build a SpatialRef. If that
    /// succeeds, it returns an Ok(Some(SpatialRef)), otherwise, you get the
    /// Err.
    ///
    pub fn spatial_reference(&self) -> Option<SpatialRef> {
        let c_spatial_ref = unsafe { gdal_sys::OGR_G_GetSpatialReference(self.c_geometry) };

        if c_spatial_ref.is_null() {
            None
        } else {
            match SpatialRef::from_c_obj(c_spatial_ref) {
                Ok(sr) => Some(sr),
                Err(_) => None,
            }
        }
    }

    pub fn set_spatial_reference(&mut self, spatial_ref: &SpatialRef) {
        unsafe {
            gdal_sys::OGR_G_AssignSpatialReference(self.c_geometry, spatial_ref.c_spatial_ref)
        };
    }

    pub fn ewkb(&self) -> Result<String> {
        let srid = self.spatial_reference().map_or(4326, |sr| {
            let auth_code = sr.auth_code().unwrap();
            //let auth_name = sr.auth_name().unwrap();
            //println!("Auth: {} : {}", auth_name, auth_code);
            auth_code
        });

        unsafe {
            //let ewkb = OGRGeometryToHexEWKB(self.c_geometry, srid, 3, 0);

            //little endian
            let wkb_size = OGR_G_WkbSize(self.c_geometry);

            let mut v: Vec<u8> = Vec::with_capacity(4 + wkb_size as usize);
            let v_ptr: *mut u8 = v.as_mut_ptr();

            //The rest will start at byte 5 + 4 == 9
            let wkb_ret = OGR_G_ExportToWkb(self.c_geometry, OGRwkbByteOrder::wkbNDR, v_ptr.offset(4));

            // [1/0  geom type - srid - rest
            // [1/0  geom type - rest

            if wkb_ret != OGRErr::OGRERR_NONE {
                return Err(_last_null_pointer_err("OGR_G_ExportToWkb"))?;
            }

            *v_ptr = 1;

            let srid_flag: u32 = 0x20000000;

            let geom_type:u32 = self.geometry_type() | srid_flag;

            //println!("Srid flag is {} geom is {}  together {}", srid_flag, self.geometry_type(), geom_type);

            ptr::copy_nonoverlapping(geom_type.to_le_bytes().as_ptr(), v_ptr.offset(1), 4);

            ptr::copy_nonoverlapping(srid.to_le_bytes().as_ptr(), v_ptr.offset(5), 4);

            let s = hex::encode(slice::from_raw_parts(v_ptr, v.capacity()));

            Ok(s)
        }

    }

    pub fn ewkb_preamble(&self) -> Result<[u8; 9]>
    {
        let mut ret: [u8; 9] = [0; 9];
        let srid = self.spatial_reference().map_or(4326, |sr| {
            let auth_code = sr.auth_code().unwrap();
            //let auth_name = sr.auth_name().unwrap();
            //println!("Auth: {} : {}", auth_name, auth_code);
            auth_code
        });

        unsafe {
            //let ewkb = OGRGeometryToHexEWKB(self.c_geometry, srid, 3, 0);


            ret[0] = 1;

            let srid_flag: u32 = 0x20000000;

            let geom_type: u32 = self.geometry_type() | srid_flag;

            //println!("Srid flag is {} geom is {}  together {}", srid_flag, self.geometry_type(), geom_type);

            //bytes 1 to 5
            ptr::copy_nonoverlapping(geom_type.to_le_bytes().as_ptr(), ret.as_mut_ptr().offset(1), 4);

            //bytes 5 to 9
            ptr::copy_nonoverlapping(srid.to_le_bytes().as_ptr(), ret.as_mut_ptr().offset(5), 4);

            Ok(ret)
        }
    }

    pub fn ewkb_preamble_with(srid: i32, geometry_type: OGRwkbGeometryType::Type) -> [u8; 9]
    {
        let mut ret: [u8; 9] = [0; 9];


        unsafe {
            //let ewkb = OGRGeometryToHexEWKB(self.c_geometry, srid, 3, 0);


            ret[0] = 1;

            let srid_flag: u32 = 0x20000000;

            let geom_type: u32 = geometry_type | srid_flag;

            //println!("Srid flag is {} geom is {}  together {}", srid_flag, self.geometry_type(), geom_type);

            //bytes 1 to 5
            ptr::copy_nonoverlapping(geom_type.to_le_bytes().as_ptr(), ret.as_mut_ptr().offset(1), 4);

            //bytes 5 to 9
            ptr::copy_nonoverlapping(srid.to_le_bytes().as_ptr(), ret.as_mut_ptr().offset(5), 4);

            ret
        }
    }

    pub fn ewkb_bytes_with_preamble(&self, preamble: &[u8], buffer: &mut Vec<u8>) -> Result<()> {
        unsafe {
            //let ewkb = OGRGeometryToHexEWKB(self.c_geometry, srid, 3, 0);

            debug_assert_eq!(preamble.len(), 9);

            //little endian
            let wkb_size = OGR_G_WkbSize(self.c_geometry);

            //let mut v: Vec<u8> = Vec::with_capacity(4 + wkb_size as usize);

            if (buffer.capacity() as i32) < 4 + wkb_size {
                println!("Buffer capacity is {} but should be at least {}", buffer.capacity(), 4+wkb_size);
                return Err(ErrorKind::GenericError{})?;
                //assert!(4 + wkb_size <= buffer.capacity() as i32);
            }
            let v_ptr: *mut u8 = buffer.as_mut_ptr();

            //The rest will start at byte 5 + 4 == 9.  We have 5 bytes with the type + the endian marker
            //then the content
            let wkb_ret = OGR_G_ExportToWkb(self.c_geometry, OGRwkbByteOrder::wkbNDR, v_ptr.offset(4));

            if wkb_ret != OGRErr::OGRERR_NONE {
                return Err(_last_null_pointer_err("OGR_G_ExportToWkb"))?;
            }

            //this will overwire the first 5 bytes of wkb
            ptr::copy_nonoverlapping(preamble.as_ptr(), v_ptr, 9);

            buffer.set_len((4+wkb_size) as usize);

            Ok(())
        }
    }

    pub fn ewkb_bytes(&self) -> Result<Vec<u8>> {
        let srid = self.spatial_reference().map_or(4326, |sr| {
            let auth_code = sr.auth_code().unwrap();
            //let auth_name = sr.auth_name().unwrap();
            //println!("Auth: {} : {}", auth_name, auth_code);
            auth_code
        });

        unsafe {
            //let ewkb = OGRGeometryToHexEWKB(self.c_geometry, srid, 3, 0);

            //little endian
            let wkb_size = OGR_G_WkbSize(self.c_geometry);

            let mut v: Vec<u8> = Vec::with_capacity(4 + wkb_size as usize);
            let v_ptr: *mut u8 = v.as_mut_ptr();

            //The rest will start at byte 5 + 4 == 9.  We have 5 bytes with the type + the endian marker
            //then the content
            let wkb_ret = OGR_G_ExportToWkb(self.c_geometry, OGRwkbByteOrder::wkbNDR, v_ptr.offset(4));

            // [1/0  geom type - srid - rest
            // [1/0  geom type - rest

            if wkb_ret != OGRErr::OGRERR_NONE {
                return Err(_last_null_pointer_err("OGR_G_ExportToWkb"))?;
            }

            *v_ptr = 1;

            let srid_flag: u32 = 0x20000000;

            let geom_type: u32 = self.geometry_type() | srid_flag;

            //println!("Srid flag is {} geom is {}  together {}", srid_flag, self.geometry_type(), geom_type);

            //bytes 1 to 5
            ptr::copy_nonoverlapping(geom_type.to_le_bytes().as_ptr(), v_ptr.offset(1), 4);

            //bytes 5 to 9
            ptr::copy_nonoverlapping(srid.to_le_bytes().as_ptr(), v_ptr.offset(5), 4);

            v.set_len(v.capacity());

            Ok(v)
        }
    }

    pub fn ewkb_bytes_raw(&self) -> Result<Vec<u8>> {


        unsafe {
            //let ewkb = OGRGeometryToHexEWKB(self.c_geometry, srid, 3, 0);

            //little endian
            let wkb_size = OGR_G_WkbSize(self.c_geometry);

            let mut v: Vec<u8> = Vec::with_capacity( wkb_size as usize);
            let v_ptr: *mut u8 = v.as_mut_ptr();

            //The rest will start at byte 5 + 4 == 9.  We have 5 bytes with the type + the endian marker
            //then the content
            let wkb_ret = OGR_G_ExportToWkb(self.c_geometry, OGRwkbByteOrder::wkbNDR, v_ptr);


            if wkb_ret != OGRErr::OGRERR_NONE {
                return Err(_last_null_pointer_err("OGR_G_ExportToWkb"))?;
            }

            v.set_len(v.capacity());

            Ok(v)
        }
    }

    pub fn import_ewkb_bytes_raw(&mut self, data: &Vec<u8>) -> Result<()> {

        unsafe {

            //The rest will start at byte 5 + 4 == 9.  We have 5 bytes with the type + the endian marker
            //then the content
            let wkb_ret = OGR_G_ImportFromWkb(self.c_geometry, data.as_ptr()
                                              as * const libc::c_void, data.len() as _);

            if wkb_ret != OGRErr::OGRERR_NONE {
                return Err(_last_null_pointer_err("OGR_G_ImportFromWkb"))?;
            }

            Ok(())
        }
    }
}

impl Drop for Geometry {
    fn drop(&mut self) {
        if self.owned {
            //println!("Destroying geometry: {:?}", self.c_geometry);
            let c_geometry = self.c_geometry;
            unsafe { gdal_sys::OGR_G_DestroyGeometry(c_geometry) };
        }
    }
}

impl Clone for Geometry {
    fn clone(&self) -> Geometry {
        // assert!(self.has_gdal_ptr());
        let c_geometry = self.c_geometry;
        let new_c_geom = unsafe { gdal_sys::OGR_G_Clone(c_geometry) };
        unsafe { Geometry::with_c_geometry(new_c_geom, true) }
    }
}

/// Geometry that depends on an existing layer
/// Thus no Drop is needed and why we keep a reference to the feature
/// Layer 'l lifetime must at least be as long as the feature lifetime
/// And dataset lifetime must the longest
pub struct FeatureGeometry<'f, 'l: 'f, 'd: 'l> {
    pub(crate) c_geometry_ref: OGRGeometryH,
    pub(crate) _feature: &'f Feature<'l, 'd>
}

impl<'f, 'l, 'd> FeatureGeometry<'f, 'l, 'd> {

    /// Convenience, don't use for long.  Can do as_geom().clone() too
    pub fn as_geom(&self) -> Geometry {
        Geometry {
            c_geometry: self.c_geometry_ref,
            owned: false
        }
    }
}

pub struct PointIterator {
    point_count: i32,
    current_point: i32,
    c_geometry: OGRGeometryH,
}

impl PointIterator {
    fn new(c_geometry: OGRGeometryH) -> Self {
        let point_count = unsafe { gdal_sys::OGR_G_GetPointCount(c_geometry) };
        Self {
            point_count,
            current_point: 0,
            c_geometry
        }
    }
}

impl Iterator for PointIterator {
    type Item = [f64; 2];

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.current_point >= self.point_count {
            None
        } else {
            let mut x: c_double = 0.;
            let mut y: c_double = 0.;
            let mut _z: c_double = 0.;
            unsafe { gdal_sys::OGR_G_GetPoint(self.c_geometry, self.current_point, &mut x, &mut y, &mut _z) };
            let pt = [x, y];
            self.current_point += 1;
            Some(pt)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Geometry;
    use crate::spatial_ref::SpatialRef;

    #[test]
    pub fn test_area() {
        let geom = Geometry::empty(::gdal_sys::OGRwkbGeometryType::wkbMultiPolygon).unwrap();
        assert_eq!(geom.area(), 0.0);

        let geom = Geometry::from_wkt("POINT(0 0)").unwrap();
        assert_eq!(geom.area(), 0.0);

        let wkt = "POLYGON ((45.0 45.0, 45.0 50.0, 50.0 50.0, 50.0 45.0, 45.0 45.0))";
        let geom = Geometry::from_wkt(wkt).unwrap();
        assert_eq!(geom.area().floor(), 25.0);
    }

    #[test]
    pub fn test_is_empty() {
        let geom = Geometry::empty(::gdal_sys::OGRwkbGeometryType::wkbMultiPolygon).unwrap();
        assert!(geom.is_empty());

        let geom = Geometry::from_wkt("POINT(0 0)").unwrap();
        assert!(!geom.is_empty());

        let wkt = "POLYGON ((45.0 45.0, 45.0 50.0, 50.0 50.0, 50.0 45.0, 45.0 45.0))";
        let geom = Geometry::from_wkt(wkt).unwrap();
        assert!(!geom.is_empty());
    }

    #[test]
    pub fn test_spatial_reference() {
        let geom = Geometry::empty(::gdal_sys::OGRwkbGeometryType::wkbMultiPolygon).unwrap();
        assert!(geom.spatial_reference().is_none());

        let geom = Geometry::from_wkt("POINT(0 0)").unwrap();
        assert!(geom.spatial_reference().is_none());

        let wkt = "POLYGON ((45.0 45.0, 45.0 50.0, 50.0 50.0, 50.0 45.0, 45.0 45.0))";
        let mut geom = Geometry::from_wkt(wkt).unwrap();
        assert!(geom.spatial_reference().is_none());

        let srs = SpatialRef::from_epsg(4326).unwrap();
        geom.set_spatial_reference(&srs);
        assert!(geom.spatial_reference().is_some());
    }
}

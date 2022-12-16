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
use crate::utils::{_string, _last_null_pointer_err};
use crate::vector::{LayerDefinition, Layer};
use crate::gdal_major_object::MajorObject;
use std::ffi::{CString, CStr};
use gdal_sys::{
    OGRGeomFieldDefnH, OGRFieldType, OGRFieldSubType,
    OGRErr, OGRwkbGeometryType,
    OGRFeatureDefnH,
    OGRFieldDefnH, GDALMajorObjectH};
use crate::errors::*;
use anyhow::Result;
use std::os::raw::c_int;
use crate::spatial_ref::SpatialRef;


/// Owned, even after using add_to_layer
pub struct FieldDefinition {
    c_obj: OGRFieldDefnH,
}

impl Drop for FieldDefinition {
    fn drop(&mut self) {
        unsafe { gdal_sys::OGR_Fld_Destroy(self.c_obj) };
    }
}

impl MajorObject for FieldDefinition {
    unsafe fn gdal_object_ptr(&self) -> GDALMajorObjectH {
        self.c_obj
    }
}

impl FieldDefinition {
    pub fn new(name: &str, field_type: OGRFieldType::Type) -> Result<FieldDefinition> {
        let c_str = CString::new(name)?;
        let c_obj = unsafe { gdal_sys::OGR_Fld_Create(c_str.as_ptr(), field_type) };
        if c_obj.is_null() {
            Err(_last_null_pointer_err("OGR_Fld_Create"))?;
        };
        Ok(FieldDefinition { c_obj })
    }
    pub fn set_width(&self, width: i32) {
        unsafe { gdal_sys::OGR_Fld_SetWidth(self.c_obj, width as c_int) };
    }
    pub fn set_precision(&self, precision: i32) {
        unsafe { gdal_sys::OGR_Fld_SetPrecision(self.c_obj, precision as c_int) };
    }
    pub fn set_sub_type(&self, sub_type: OGRFieldSubType::Type) {
        //OGR_AreTypeSubTypeCompatible ?
        unsafe { gdal_sys::OGR_Fld_SetSubType(self.c_obj, sub_type ) };
    }
    pub fn add_to_layer(&self, layer: &mut Layer) -> Result<()> {
        //seems like you still need to destroy it afterwards, so self still runs drop afterwards
        let rv = unsafe { gdal_sys::OGR_L_CreateField(layer.c_layer(), self.c_obj, 1) };
        if rv != OGRErr::OGRERR_NONE {
            Err(ErrorKind::OgrError {
                err: rv,
                method_name: "OGR_L_CreateFeature",
            })?;
        }
        Ok(())
    }
}



/// This Field is owned by the layer definition.  So we keep a reference so rust
/// checks the lifetimes for us
/// Field lifetime at least as long as layer
/// layer at least as long as dataset
pub struct Field<'f: 'l, 'l: 'd, 'd> {
    pub(crate) _defn: &'f LayerDefinition<'l, 'd>,
    pub(crate) c_field_defn: OGRFieldDefnH,
}

impl<'f: 'l, 'l: 'd, 'd> Field<'f, 'l, 'd> {
    /// Get the name of this field.
    pub fn name(&'f self) -> String {
        let rv = unsafe { gdal_sys::OGR_Fld_GetNameRef(self.c_field_defn) };
        _string(rv)
    }

    pub fn field_type(&'f self) -> OGRFieldType::Type {
        unsafe { gdal_sys::OGR_Fld_GetType(self.c_field_defn) }
    }

    pub fn width(&'f self) -> i32 {
        unsafe { gdal_sys::OGR_Fld_GetWidth(self.c_field_defn) }
    }

    pub fn precision(&'f self) -> i32 {
        unsafe { gdal_sys::OGR_Fld_GetPrecision(self.c_field_defn) }
    }
}

//feature 'f less that layer 'l less than dataset 'd
pub struct FieldIterator<'f: 'l, 'l: 'd, 'd> {
    pub (crate) defn: &'f LayerDefinition<'l, 'd>,
    pub (crate) c_feature_defn: OGRFeatureDefnH,
    pub (crate) next_id: isize,
    pub (crate) total: isize,
}


impl<'f, 'l, 'd> Iterator for FieldIterator<'f, 'l, 'd> {
    type Item = Field<'f, 'l, 'd>;

    #[inline]
    fn next(&mut self) -> Option<Field<'f, 'l, 'd>> {
        if self.next_id == self.total {
            return None;
        }
        let field = Field {
            _defn: self.defn,
            c_field_defn: unsafe {
                gdal_sys::OGR_FD_GetFieldDefn(self.c_feature_defn, self.next_id as c_int)
            },
        };
        self.next_id += 1;
        Some(field)
    }
}




// http://gdal.org/classOGRGeomFieldDefn.html

pub struct GeomFieldIterator {
    //pub (crate) defn: &'f LayerDefinition<'l, 'd>,
    pub (crate) c_feature_defn: OGRFeatureDefnH,
    pub (crate) next_id: isize,
    pub (crate) total: isize,
}

impl Iterator for GeomFieldIterator {
    type Item = GeomField;

    #[inline]
    fn next(&mut self) ->  Option<GeomField> {
        if self.next_id == self.total {
            return None;
        }
        let field = GeomField {

            c_field_defn: unsafe {
                gdal_sys::OGR_FD_GetGeomFieldDefn(self.c_feature_defn, self.next_id as c_int)
            },
        };
        self.next_id += 1;
        Some(field)
    }
}

// http://gdal.org/classOGRGeomFieldDefn.html
//TODO maybe feature definition lifetime == layer lifetime?
//Owned vs non owned...Drop?
pub struct GeomField {
    //pub(crate) _defn: &'f LayerDefinition<'l, 'd>,
    pub(crate) c_field_defn: OGRGeomFieldDefnH,
}

impl GeomField {

    pub fn new(name: &str, geom_type: OGRwkbGeometryType::Type) -> Result<Self> {
        let c_str = CString::new(name)?;
        let c_obj = unsafe { gdal_sys::OGR_GFld_Create(c_str.as_ptr(), geom_type) };
        if c_obj.is_null() {
            Err(_last_null_pointer_err("OGR_Fld_Create"))?;
        };
        Ok(GeomField {  c_field_defn: c_obj })

    }

    /// Get the name of this field.
    pub fn name(&self) -> String {
        let rv = unsafe { gdal_sys::OGR_GFld_GetNameRef(self.c_field_defn) };
        _string(rv)
    }

    pub fn field_type(&self) -> OGRwkbGeometryType::Type {
        unsafe { gdal_sys::OGR_GFld_GetType(self.c_field_defn) }
    }

    pub fn spatial_ref(&self) -> Result<SpatialRef> {
        let c_obj = unsafe { gdal_sys::OGR_GFld_GetSpatialRef(self.c_field_defn) };
        if c_obj.is_null() {
            return Err(_last_null_pointer_err("OGR_GFld_GetSpatialRef"))?;
        }
        SpatialRef::from_c_obj(c_obj)
    }

    pub fn add_to_layer(&self, layer: &mut Layer) -> Result<()> {
        //seems like you still need to destroy it afterwards, so self still runs drop afterwards
        let rv = unsafe { gdal_sys::OGR_L_CreateGeomField(layer.c_layer(), self.c_field_defn, 1) };
        if rv != OGRErr::OGRERR_NONE {
            Err(ErrorKind::OgrError {
                err: rv,
                method_name: "OGR_L_CreateGeomField",
            })?;
        }
        Ok(())
    }
}

/*
struct OwnedGeometryField {

}

impl OwnedGeometryField {
    pub fn new(name: &str, field_type: OGRwkbGeometryType::Type) -> Result<GeomField> {
        let c_str = CString::new(name)?;
        let c_obj = unsafe { gdal_sys::OGR_GFld_Create(c_str.as_ptr(), field_type) };
        if c_obj.is_null() {
            Err(_last_null_pointer_err("OGR_GFld_Create"))?;
        };
        Ok(OwnedGeometryField { c_field_defn: c_obj })
    }
}*/


pub fn geometry_type_to_name(geom_type: OGRwkbGeometryType::Type) -> Result<&'static str> {
    unsafe {
        let name = gdal_sys::OGRGeometryTypeToName(geom_type);

        if name.is_null() {
            Err(_last_null_pointer_err("OGRGeometryTypeToName"))?;
        }

        let c_str = CStr::from_ptr(name);
        match c_str.to_str() {
            Ok(s) => Ok(s),
            Err(e) => Err(ErrorKind::StrUtf8Error(e))?
        }
    }
}


pub fn field_type_to_name(field_type: OGRFieldType::Type) -> Result<&'static str> {
    unsafe {
        let name = gdal_sys::OGR_GetFieldTypeName(field_type);

        if name.is_null() {
            Err(_last_null_pointer_err("OGRGeometryTypeToName"))?;
        }

        let c_str = CStr::from_ptr(name);
        match c_str.to_str() {
            Ok(s) => Ok(s),
            Err(e) => Err(ErrorKind::StrUtf8Error(e))?
        }
    }
}
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
use crate::vector::layer::Layer;
use gdal_sys::{self, OGRFeatureDefnH, OGRwkbGeometryType};

use crate::errors::*;
use std::convert::TryFrom;
use std::ffi::{CString};

use anyhow::{Result, bail};
use crate::vector::{FieldIterator, Field};
use crate::vector::field::{GeomFieldIterator, GeomField};

/// Layer definition
///
/// Defines the fields available for features in a layer.
/// This is not owned, so we use the _layer to make sure it doesn't outlive the layer
/// life time dataset must live at least as long as the life time of the layer
pub struct LayerDefinition<'l: 'd, 'd> {
    pub(crate) c_defn: OGRFeatureDefnH,

    pub(crate) _layer: &'l Layer<'d>
}

impl <'l, 'd> LayerDefinition<'l, 'd> {
    pub unsafe fn _with_c_defn(layer: &'l Layer, c_defn: OGRFeatureDefnH) -> LayerDefinition<'l, 'd> {
        //This comes from the layer
        LayerDefinition
        {
            _layer: layer,
            c_defn,
        }
    }

    pub unsafe fn c_defn(&self) -> OGRFeatureDefnH {
        self.c_defn
    }

    /// Iterate over the field schema of this layer.
    pub fn fields(&self) -> FieldIterator {
        let total = unsafe { gdal_sys::OGR_FD_GetFieldCount(self.c_defn) } as isize;
        FieldIterator {
            defn: self,
            c_feature_defn: self.c_defn,
            next_id: 0,
            total,
        }
    }

    /// Iterate over the geometry field schema of this layer.

    pub fn geom_fields(&self) -> GeomFieldIterator {
        let total = unsafe { gdal_sys::OGR_FD_GetGeomFieldCount(self.c_defn) } as isize;
        GeomFieldIterator {
            c_feature_defn: self.c_defn,
            next_id: 0,
            total,
        }
    }

    pub fn geom_field_count(&self) -> i32 {
        let total = unsafe { gdal_sys::OGR_FD_GetGeomFieldCount(self.c_defn) } ;
        i32::try_from(total).expect("total is not convertable")
    }

    pub fn field_count(&self) -> i32 {
        let total = unsafe { gdal_sys::OGR_FD_GetFieldCount(self.c_defn) } ;
        i32::try_from(total).expect("total is not convertable")
    }

    pub fn get_field_index(&self, field_name: &str) -> Result<i32> {

        let c_str_field_name = CString::new(field_name)?;
        let idx =
            unsafe { gdal_sys::OGR_FD_GetFieldIndex(self.c_defn, c_str_field_name.as_ptr()) };

        if idx == -1 {

            //Build a better error message
            let fc = self.field_count();
            let mut field_names = Vec::with_capacity(fc as _);
            for f_idx in 0..fc {
                let f = self.get_field(f_idx);
                field_names.push(f.name());
            }
            let joined = field_names.join(", ");

            bail!("Invalid field name {} found names {}", field_name, joined);
        }

        Ok(idx)
    }

    pub fn get_field(&self, field_index: i32) -> Field {
        Field {
            _defn: self,
            c_field_defn: unsafe {
                //This object should not be modified or freed by the application.
                gdal_sys::OGR_FD_GetFieldDefn(self.c_defn, field_index)
            },
        }

    }

    pub fn get_geom_field(&self, field_index: i32) -> GeomField {
        GeomField {
            //_defn: self,
            c_field_defn: unsafe {
                //This object should not be modified or freed by the application.
                gdal_sys::OGR_FD_GetGeomFieldDefn(self.c_defn, field_index)
            },
        }
    }

    pub fn get_geom_field_index(&self, field_name: &str) -> Result<i32> {

        let c_str_field_name = CString::new(field_name)?;
        let idx =
            unsafe { gdal_sys::OGR_FD_GetGeomFieldIndex(self.c_defn, c_str_field_name.as_ptr()) };

        if idx == -1 {
            Err(ErrorKind::InvalidFieldName {
                field_name: field_name.to_string(),
                method_name: "OGR_FD_GetGeomFieldIndex",
            })?;
        }

        Ok(idx)
    }

    pub fn get_geometry_type(&self) -> OGRwkbGeometryType::Type
    {
        let geom_type =
            unsafe { gdal_sys::OGR_FD_GetGeomType(self.c_defn) };
        geom_type
    }
}



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
use crate::gdal_major_object::MajorObject;
use crate::metadata::Metadata;
use crate::spatial_ref::SpatialRef;
use crate::utils::{_last_null_pointer_err, _string};
use crate::vector::layer_definition::LayerDefinition;
use crate::vector::{Feature, Geometry, Field, Dataset, FieldValue};
use gdal_sys::{
    self, GDALMajorObjectH, OGREnvelope, OGRErr, OGRFieldType, OGRLayerH,
};
use std::ffi::CString;
use std::ptr::null_mut;

use crate::errors::*;
use anyhow::Result;
use crate::vector::field::{FieldDefinition, GeomField};

/// Layer in a vector dataset.  This is an existing layer that will live shorter than the dataset
///

pub struct Layer<'d> {
    pub (crate) c_layer: OGRLayerH,
    pub (crate) _dataset: &'d Dataset,
    pub (crate) owned: bool
}

impl <'a> MajorObject for Layer<'a > {
    unsafe fn gdal_object_ptr(&self) -> GDALMajorObjectH {
        self.c_layer
    }
}

impl <'a> Metadata for Layer<'a> {}

impl <'d> Drop for Layer<'d> {
    fn drop(&mut self) {
        if self.owned {
            println!("Dropping layer created by execute sql");
            unsafe {
                gdal_sys::OGR_DS_ReleaseResultSet(self._dataset.c_dataset, self.c_layer)
            }
        }
    }
}
//
// mod capability_constants {
//     const OLC_CREATE_FIELD: &str = "CreateField";
//     const OLC_CREATE_GEOM_FIELD: &str = "CreateGeomField";
//
// }

impl <'a> Layer<'a> {


    pub unsafe fn c_layer(&self) -> OGRLayerH {
        self.c_layer
    }

    pub fn layer_definition(&self) -> LayerDefinition {
        unsafe {
            //This does not require freeing, so we also pass a reference so rust
            //checks lifetimes for us, it can't out live layer
            let c_defn = gdal_sys::OGR_L_GetLayerDefn(self.c_layer);

            LayerDefinition {
                c_defn,
                _layer: self
            }
        }
    }

    pub fn test_capability(&self, capability: &str) -> Result<bool> {
        let c_capability = CString::new(capability)?;

        let r_int = unsafe {
            gdal_sys::OGR_L_TestCapability(self.c_layer, c_capability.as_ptr() )
        } ;

        Ok(r_int == 1)
    }

    /// Iterate over all features in this layer.
    pub fn features(&self) -> FeatureIterator {
        FeatureIterator::_with_layer(self)
    }

    pub fn get_feature_by_id<'l, 'd>(&'d self, fid: i64) -> Result<Feature<'l, 'd>>
    {
        unsafe {
            let feature_h = gdal_sys::OGR_L_GetFeature(self.c_layer, fid);
            if feature_h.is_null() {
                Err(_last_null_pointer_err("OGR_L_GetFeature"))?;
            }

            Ok(Feature::_with_c_feature(
                self,
                feature_h
            ))
        }
    }

    pub fn set_spatial_filter(&self, geometry: &Geometry) {
        unsafe { gdal_sys::OGR_L_SetSpatialFilter(self.c_layer, geometry.c_geometry) };
    }

    pub fn set_spatial_filter_rect(&self, min_x: f64, min_y: f64, max_x: f64, max_y: f64)
    {
        unsafe {
            gdal_sys::OGR_L_SetSpatialFilterRect(self.c_layer, min_x, min_y, max_x, max_y);

            //as we just set the filter, well want to reset this
            gdal_sys::OGR_L_ResetReading(self.c_layer);
        }
    }

    pub fn set_feature(&self, feature: &Feature) -> Result<()> {
        unsafe {
            let rv = gdal_sys::OGR_L_SetFeature(self.c_layer, feature.c_feature);
            if rv != OGRErr::OGRERR_NONE {
                Err(ErrorKind::OgrError {
                    err: rv,
                    method_name: "OGR_L_SetFeature",
                })?;
            }

            Ok(())
        }
    }

    pub fn set_attribute_filter(&self, filter: &str) {
        let c_filter = CString::new(filter).unwrap();
        unsafe { gdal_sys::OGR_L_SetAttributeFilter(self.c_layer, c_filter.as_ptr()) };
    }

    pub fn clear_spatial_filter(&self) {
        unsafe { gdal_sys::OGR_L_SetSpatialFilter(self.c_layer, null_mut()) };
    }

    pub fn count(&self, force:bool) -> i64 {
        let fc = unsafe { gdal_sys::OGR_L_GetFeatureCount(self.c_layer, if force {1} else {0}) };
        fc
    }

    /// Get the name of this layer.
    pub fn name(&self) -> String {
        let rv = unsafe { gdal_sys::OGR_L_GetName(self.c_layer) };
        _string(rv)
    }


    pub fn create_defn_fields(&mut self, fields_def: &[(&str, OGRFieldType::Type)]) -> Result<()> {
        for fd in fields_def {
            let fdefn = FieldDefinition::new(fd.0, fd.1)?;
            fdefn.add_to_layer(self)?;
        }
        Ok(())
    }

    pub fn create_geom_field(&mut self, geom_field: &GeomField, approx_ok: bool) -> Result<()> {
        let b_approx_ok: libc::c_int = if approx_ok {1} else {0};
        let rv = unsafe { gdal_sys::OGR_L_CreateGeomField(self.c_layer, geom_field.c_field_defn, b_approx_ok) };
        if rv != OGRErr::OGRERR_NONE {
            Err(ErrorKind::OgrError {
                err: rv,
                method_name: "OGR_L_CreateGeomField",
            })?;
        }
        Ok(())
    }

    pub fn create_field(&mut self, field: &Field, approx_ok: bool) -> Result<()> {

        let b_approx_ok: libc::c_int = if approx_ok {1} else {0};

        //Note to add to a field definition it is OGR_FD_AddFieldDefn
        let rv = unsafe { gdal_sys::OGR_L_CreateField(self.c_layer, field.c_field_defn, b_approx_ok) };
        if rv != OGRErr::OGRERR_NONE {
            Err(ErrorKind::OgrError {
                err: rv,
                method_name: "OGR_L_CreateField",
            })?;
        }
        Ok(())
    }

    /*
    pub fn create_feature(&mut self, geometry: Geometry) -> Result<()> {
        let c_feature = unsafe { gdal_sys::OGR_F_Create(self.defn.c_defn()) };
        let c_geometry = unsafe { geometry.into_c_geometry() };
        let rv = unsafe { gdal_sys::OGR_F_SetGeometryDirectly(c_feature, c_geometry) };
        if rv != OGRErr::OGRERR_NONE {
            Err(ErrorKind::OgrError {
                err: rv,
                method_name: "OGR_F_SetGeometryDirectly",
            })?;
        }
        let rv = unsafe { gdal_sys::OGR_L_CreateFeature(self.c_layer, c_feature) };
        if rv != OGRErr::OGRERR_NONE {
            Err(ErrorKind::OgrError {
                err: rv,
                method_name: "OGR_L_CreateFeature",
            })?;
        }
        Ok(())
    }*/


    pub fn create_feature_fields(
        &mut self,
        geometry: Geometry,
        field_names: &[&str],
        values: &[FieldValue],
    ) -> Result<()> {
        let layer_def = self.layer_definition();
        let mut ft = Feature::new(&layer_def)?;
        ft.set_geometry(geometry)?;
        for (fd, val) in field_names.iter().zip(values.iter()) {
            ft.set_field(fd, val)?;
        }
        ft.create(self)?;
        Ok(())
    }

    pub fn get_extent(&self, force: bool) -> Result<gdal_sys::OGREnvelope> {
        let mut envelope = OGREnvelope {
            MinX: 0.0,
            MaxX: 0.0,
            MinY: 0.0,
            MaxY: 0.0,
        };
        let force = if force { 1 } else { 0 };
        let rv = unsafe { gdal_sys::OGR_L_GetExtent(self.c_layer, &mut envelope, force) };
        if rv != OGRErr::OGRERR_NONE {
            Err(ErrorKind::OgrError {
                err: rv,
                method_name: "OGR_L_GetExtent",
            })?;
        }
        Ok(envelope)
    }

    pub fn spatial_reference(&self) -> Result<SpatialRef> {
        let c_obj = unsafe { gdal_sys::OGR_L_GetSpatialRef(self.c_layer) };
        if c_obj.is_null() {
            Err(_last_null_pointer_err("OGR_L_GetSpatialRef"))?;
        }
        SpatialRef::from_c_obj(c_obj)
    }
}

/// Lifetime of dataset must at least be as long of the layer
pub struct FeatureIterator<'l, 'd: 'l> {
    layer: &'l Layer<'d>,
}

impl<'l, 'd> Iterator for FeatureIterator<'l, 'd> {
    type Item = Feature<'l, 'd>;

    #[inline]
    fn next(&mut self) -> Option<Feature<'l, 'd>> {
        let c_feature = unsafe { gdal_sys::OGR_L_GetNextFeature(self.layer.c_layer) };
        if c_feature.is_null() {
            None
        } else {
            Some(unsafe { Feature::_with_c_feature(&self.layer, c_feature) })
        }
    }
}

impl<'l, 'd: 'l> FeatureIterator<'l, 'd> {
    pub fn _with_layer(layer: &'l Layer<'d>) -> FeatureIterator<'l, 'd> {
        FeatureIterator { layer }
    }
}

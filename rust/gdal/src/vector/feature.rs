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
use crate::utils::{_last_null_pointer_err, _string};
use crate::vector::geometry::{Geometry, FeatureGeometry};
use crate::vector::layer::Layer;
use crate::vector::LayerDefinition;
use gdal_sys::{self, OGRErr, OGRFeatureH, OGRFieldType};
use libc::{c_double, c_int};
use std::ffi::CString;

use chrono::{DateTime, Datelike, FixedOffset, TimeZone, Timelike, NaiveDate, NaiveTime, NaiveDateTime};
use serde::{Deserialize, Serialize};

use crate::errors::*;
use anyhow::Result;
use std::convert::TryFrom;

/// OGR Feature
/// This corresponds to an item returned by OGR_L_GetNextFeature
/// We must drop it when done
/// but the drops must be done before the layer is dropped, so we need the layer reference
/// to have Rust ensure that.  dataset lifetime must be at least as long as the layer lifetime
pub struct Feature<'l, 'd: 'l> {
    pub(crate) c_feature: OGRFeatureH,
    _layer: &'l Layer<'d>
}

fn field_from_idx(feature: OGRFeatureH, field_id: i32) -> Result<FieldValue> {

    let field_defn = unsafe { gdal_sys::OGR_F_GetFieldDefnRef(feature, field_id) };
    let field_type = unsafe { gdal_sys::OGR_Fld_GetType(field_defn) };
    match field_type {
        OGRFieldType::OFTString => {
            let rv = unsafe { gdal_sys::OGR_F_GetFieldAsString(feature, field_id) };
            Ok(FieldValue::StringValue(_string(rv)))
        }
        OGRFieldType::OFTReal => {
            let rv = unsafe { gdal_sys::OGR_F_GetFieldAsDouble(feature, field_id) };
            Ok(FieldValue::RealValue(rv as f64))
        }
        OGRFieldType::OFTInteger => {
            let rv = unsafe { gdal_sys::OGR_F_GetFieldAsInteger(feature, field_id) };
            Ok(FieldValue::IntegerValue(rv as i32))
        }
        OGRFieldType::OFTInteger64 => {
            let rv = unsafe { gdal_sys::OGR_F_GetFieldAsInteger64(feature, field_id) };
            Ok(FieldValue::Integer64Value(rv as i64))
        }
        OGRFieldType::OFTDateTime => {
            let rv = get_field_datetime(feature, field_id);
            if let Ok(rv) = rv {
                Ok(FieldValue::DateTimeValue(rv))
            } else {
                Ok(FieldValue::Null)
            }
        },
        OGRFieldType::OFTDate => {
            let rv = get_field_datetime(feature, field_id);
            if let Ok(rv) = rv {
                Ok(FieldValue::DateTimeValue(rv))
            } else {
                Ok(FieldValue::Null)
            }
        },
        OGRFieldType::OFTRealList => {
            let mut double_count: c_int = 0;
            let rv = unsafe { gdal_sys::OGR_F_GetFieldAsDoubleList(feature, field_id, &mut double_count) };
            let slice = unsafe { std::slice::from_raw_parts(rv, double_count as usize) };
            Ok(FieldValue::RealListValue(slice.to_vec()))

        }
        _ => Err(ErrorKind::UnhandledFieldType {
            field_type,
            method_name: "OGR_Fld_GetType",
        })?,
    }
}

fn get_field_datetime(feature: OGRFeatureH, field_id: c_int) -> Result<DateTime<FixedOffset>> {
    let mut year: c_int = 0;
    let mut month: c_int = 0;
    let mut day: c_int = 0;
    let mut hour: c_int = 0;
    let mut minute: c_int = 0;
    let mut second: c_int = 0;
    let mut tzflag: c_int = 0;

    let success = unsafe {
        gdal_sys::OGR_F_GetFieldAsDateTime(
            feature,
            field_id,
            &mut year,
            &mut month,
            &mut day,
            &mut hour,
            &mut minute,
            &mut second,
            &mut tzflag,
        )
    };
    if success == 0 {
        Err(ErrorKind::OgrError {
            err: OGRErr::OGRERR_FAILURE,
            method_name: "OGR_F_GetFieldAsDateTime",
        })?;
    }

    // from https://github.com/OSGeo/gdal/blob/33a8a0edc764253b582e194d330eec3b83072863/gdal/ogr/ogrutils.cpp#L1309
    // GByte   TZFlag; /* 0=unknown, 1=localtime(ambiguous),
    //                            100=GMT, 104=GMT+1, 80=GMT-5, etc */
    let tzoffset_secs = if tzflag == 0 || tzflag == 100 {
        0
    } else {
        (tzflag as i32 - 100) * 15 * 60
    };
    let rv = FixedOffset::east(tzoffset_secs)
        .ymd(year as i32, month as u32, day as u32)
        .and_hms(hour as u32, minute as u32, second as u32);
    Ok(rv)
}

pub fn set_field_datetime(feature: OGRFeatureH, field_idx: i32, value: DateTime<FixedOffset>) -> Result<()> {


    let year = value.year() as c_int;
    let month = value.month() as c_int;
    let day = value.day() as c_int;
    let hour = value.hour() as c_int;
    let minute = value.minute() as c_int;
    let second = value.second() as c_int;
    let tzflag: c_int = if value.offset().local_minus_utc() == 0 {
        0
    } else {
        100 + (value.offset().local_minus_utc() / (15 * 60))
    };

    unsafe {
        gdal_sys::OGR_F_SetFieldDateTime(
            feature,
            field_idx,
            year,
            month,
            day,
            hour,
            minute,
            second,
            tzflag,
        )
    };
    Ok(())
}


impl<'l, 'd> Feature<'l, 'd> {

    pub fn new(defn: &'l LayerDefinition<'l,'d>) -> Result<Feature<'l, 'd>> {
        // Note that the OGRFeature will increment the reference count of its defining OGRFeatureDefn.
        // Destruction of the OGRFeatureDefn before destruction of all OGRFeatures that depend on it is likely to result in a crash.
        let c_feature = unsafe { gdal_sys::OGR_F_Create(defn.c_defn()) };
        if c_feature.is_null() {
            Err(_last_null_pointer_err("OGR_F_Create"))?;
        };
        Ok(Feature {
            _layer: defn._layer,
            c_feature,
        })
    }

    pub unsafe fn _with_c_feature(layer: &'l Layer<'d>, c_feature: OGRFeatureH) -> Feature<'l, 'd> {
        Feature {
            _layer: layer,
            c_feature,
            //geometry: Feature::_lazy_feature_geometries(defn),
        }
    }

    /*
    pub fn _lazy_feature_geometries(defn: &'a FeatureDefinition) -> Vec<Geometry> {
        let geom_field_count =
            unsafe { gdal_sys::OGR_FD_GetGeomFieldCount(defn.c_defn()) } as isize;
        (0..geom_field_count)
            .map(|_| unsafe { Geometry::lazy_feature_geometry() })
            .collect()
    }*/

    pub fn field_count(&self) -> i32 {
        let field_count = unsafe { gdal_sys::OGR_F_GetFieldCount(self.c_feature) };

        return i32::try_from(field_count).expect("Not an i32");
    }

    /// Get the value of a named field. If the field exists, it returns a
    /// `FieldValue` wrapper, that you need to unpack to a base type
    /// (string, float, etc). If the field is missing, returns `None`.
    pub fn field(&self, name: &str) -> Result<FieldValue> {
        let c_name = CString::new(name)?;
        let field_id = unsafe { gdal_sys::OGR_F_GetFieldIndex(self.c_feature, c_name.as_ptr()) };
        if field_id == -1 {
            Err(ErrorKind::InvalidFieldName {
                field_name: name.to_string(),
                method_name: "OGR_F_GetFieldIndex",
            })?;
        }
        self.field_from_idx(field_id)
    }

    pub fn field_from_idx(&self, field_id: i32) -> Result<FieldValue> {
        field_from_idx(self.c_feature, field_id)
    }

    pub fn is_field_set_and_not_null(&self, field_id: i32) -> bool {
        let rv = unsafe { gdal_sys::OGR_F_IsFieldSetAndNotNull(self.c_feature, field_id)};

        return rv != 0;
    }
    pub fn get_field_as_string(&self, field_id: i32) -> String {
        let rv = unsafe { gdal_sys::OGR_F_GetFieldAsString(self.c_feature, field_id) };
        _string(rv)
    }
    pub fn get_field_as_int(&self, field_id: i32) -> i32 {
        let rv = unsafe { gdal_sys::OGR_F_GetFieldAsInteger(self.c_feature, field_id) };
        rv
    }
    pub fn get_field_as_int64(&self, field_id: i32) -> i64 {
        let rv = unsafe { gdal_sys::OGR_F_GetFieldAsInteger64(self.c_feature, field_id) };
        rv
    }
    pub fn get_field_as_real(&self, field_id: i32) -> f64 {
        let rv = unsafe { gdal_sys::OGR_F_GetFieldAsDouble(self.c_feature, field_id) };
        rv
    }

    pub fn fid(&self) -> i64 {
        let fid = unsafe { gdal_sys::OGR_F_GetFID(self.c_feature) };
        fid
    }



    /// Get the field's geometry.
    pub fn geometry<'f>(&'f self) -> FeatureGeometry<'f, 'l, 'd>
    {

        //No memory cleanup is needed
        let c_geom = unsafe { gdal_sys::OGR_F_GetGeometryRef(self.c_feature) };

        //The geometry cannot outlive the feature, so we pass a reference to it
        //we want the geometry to disappear, then the feature, then the layer
        FeatureGeometry {
            c_geometry_ref: c_geom,
            _feature: &self
        }
    }

    pub fn geometry_by_name<'f>(&'f self, field_name: &str) ->  Result<FeatureGeometry<'f, 'l, 'd>> {
        let c_str_field_name = CString::new(field_name)?;
        let idx =
            unsafe { gdal_sys::OGR_F_GetGeomFieldIndex(self.c_feature, c_str_field_name.as_ptr()) };
        if idx == -1 {
            Err(ErrorKind::InvalidFieldName {
                field_name: field_name.to_string(),
                method_name: "geometry_by_name",
            })?
        } else {
            self.geometry_by_index(idx )
        }
    }

    pub fn geometry_by_index<'f>(&'f self, idx: i32) -> Result<FeatureGeometry<'f, 'l, 'd>> {
        let c_geom = unsafe { gdal_sys::OGR_F_GetGeomFieldRef(self.c_feature, idx ) };
        if c_geom.is_null() {
            Err(_last_null_pointer_err("OGR_F_GetGeomFieldRef"))?;
        }
        //The geometry cannot outlive the feature, so we pass a reference to it
        //we want the geometry to disappear, then the feature, then the layer
        Ok(FeatureGeometry {
            c_geometry_ref: c_geom,
            _feature: &self
        })
    }


    pub fn set_field_string(&self, field_name: &str, value: &str) -> Result<()> {
        let c_str_field_name = CString::new(field_name)?;
        let c_str_value = CString::new(value)?;
        let idx =
            unsafe { gdal_sys::OGR_F_GetFieldIndex(self.c_feature, c_str_field_name.as_ptr()) };
        if idx == -1 {
            Err(ErrorKind::InvalidFieldName {
                field_name: field_name.to_string(),
                method_name: "OGR_F_GetFieldIndex",
            })?;
        }
        unsafe { gdal_sys::OGR_F_SetFieldString(self.c_feature, idx, c_str_value.as_ptr()) };
        Ok(())
    }

    pub fn set_field_listf64(&self, field_name: &str, value: &Vec<f64>) -> Result<()> {
        let c_str_field_name = CString::new(field_name)?;
        let idx =
            unsafe { gdal_sys::OGR_F_GetFieldIndex(self.c_feature, c_str_field_name.as_ptr()) };
        if idx == -1 {
            Err(ErrorKind::InvalidFieldName {
                field_name: field_name.to_string(),
                method_name: "OGR_F_GetFieldIndex",
            })?;
        }
        self.set_field_listf64_by_index(idx, value)
    }

    pub fn set_field_double(&self, field_name: &str, value: f64) -> Result<()> {
        let c_str_field_name = CString::new(field_name)?;
        let idx =
            unsafe { gdal_sys::OGR_F_GetFieldIndex(self.c_feature, c_str_field_name.as_ptr()) };
        if idx == -1 {
            Err(ErrorKind::InvalidFieldName {
                field_name: field_name.to_string(),
                method_name: "OGR_F_GetFieldIndex",
            })?;
        }
        unsafe { gdal_sys::OGR_F_SetFieldDouble(self.c_feature, idx, value as c_double) };
        Ok(())
    }

    pub fn set_field_double_by_index(&self, field_idx: i32, value: f64) -> Result<()> {
        unsafe { gdal_sys::OGR_F_SetFieldDouble(self.c_feature, field_idx, value ) };
        Ok(())
    }

    pub fn set_field_integer(&self, field_name: &str, value: i32) -> Result<()> {
        let c_str_field_name = CString::new(field_name)?;
        let idx =
            unsafe { gdal_sys::OGR_F_GetFieldIndex(self.c_feature, c_str_field_name.as_ptr()) };
        if idx == -1 {
            Err(ErrorKind::InvalidFieldName {
                field_name: field_name.to_string(),
                method_name: "OGR_F_GetFieldIndex",
            })?;
        }
        unsafe { gdal_sys::OGR_F_SetFieldInteger(self.c_feature, idx, value ) };
        Ok(())
    }

    pub fn set_field_integer64(&self, field_name: &str, value: i64) -> Result<()> {
        let c_str_field_name = CString::new(field_name)?;
        let idx =
            unsafe { gdal_sys::OGR_F_GetFieldIndex(self.c_feature, c_str_field_name.as_ptr()) };
        if idx == -1 {
            Err(ErrorKind::InvalidFieldName {
                field_name: field_name.to_string(),
                method_name: "OGR_F_GetFieldIndex",
            })?;
        }
        unsafe { gdal_sys::OGR_F_SetFieldInteger64(self.c_feature, idx, value ) };
        Ok(())
    }

    /*
    Use feature definition
     */
    /*
    pub fn get_field_index(&self, field_name: &str) -> Result<i32> {
        let c_str_field_name = CString::new(field_name)?;
        let idx =
            unsafe { gdal_sys::OGR_F_GetFieldIndex(self.c_feature, c_str_field_name.as_ptr()) };

        if idx == -1 {
            Err(ErrorKind::InvalidFieldName {
                field_name: field_name.to_string(),
                method_name: "OGR_F_GetFieldIndex",
            })?;
        }

        Ok(idx)
    }*/

    pub fn set_field_integer_by_index(&self, field_idx: i32, value: i32) -> Result<()> {

        unsafe { gdal_sys::OGR_F_SetFieldInteger(self.c_feature, field_idx, value as c_int) };
        Ok(())
    }

    pub fn set_field_datetime(&self, field_name: &str, value: DateTime<FixedOffset>) -> Result<()> {
        let c_str_field_name = CString::new(field_name)?;
        let idx =
            unsafe { gdal_sys::OGR_F_GetFieldIndex(self.c_feature, c_str_field_name.as_ptr()) };
        if idx == -1 {
            Err(ErrorKind::InvalidFieldName {
                field_name: field_name.to_string(),
                method_name: "OGR_F_GetFieldIndex",
            })?;
        }

        set_field_datetime(self.c_feature, idx, value)
    }


    pub fn set_field_string_by_index(&self, field_idx: i32, value: &str) -> Result<()> {

        let c_str_value = CString::new(value)?;
        unsafe { gdal_sys::OGR_F_SetFieldString(self.c_feature, field_idx, c_str_value.as_ptr()) };
        Ok(())
    }

    pub fn set_field_integer64_by_index(&self, field_idx: i32, value: i64) -> Result<()> {

        unsafe { gdal_sys::OGR_F_SetFieldInteger64(self.c_feature, field_idx, value ) };
        Ok(())
    }

    pub fn set_field_listf64_by_index(&self, field_idx: i32, value: &Vec<f64>) -> Result<()> {

        unsafe { gdal_sys::OGR_F_SetFieldDoubleList(self.c_feature, field_idx, value.len() as c_int,
        value.as_ptr()
        ) };
        Ok(())
    }


    pub fn set_field_datetime_by_index(&self, field_idx: i32, value: DateTime<FixedOffset>) -> Result<()> {
        set_field_datetime(self.c_feature, field_idx, value)
    }

    pub fn set_geometry_directly(&mut self, mut geom: Geometry) -> Result<()> {
        assert!(geom.owned);
        geom.owned = false;
        //sets in memory, transfers ownership even if it fails
        let rv = unsafe { gdal_sys::OGR_F_SetGeometryDirectly(self.c_feature, geom.c_geometry) };
        if rv != OGRErr::OGRERR_NONE {
            Err(ErrorKind::OgrError {
                err: rv,
                method_name: "OGR_G_SetGeometry",
            })?;
        }
        Ok(())
    }

    pub fn set_geometry(&mut self, geom: Geometry) -> Result<()> {
        //sets in memory, makes a copy of geom
        let rv = unsafe { gdal_sys::OGR_F_SetGeometry(self.c_feature, geom.c_geometry) };
        if rv != OGRErr::OGRERR_NONE {
            Err(ErrorKind::OgrError {
                err: rv,
                method_name: "OGR_G_SetGeometry",
            })?;
        }
        Ok(())
    }

     pub fn set_geometry_directly_with_index(&mut self, mut geom: Geometry, index: i32) -> Result<()> {
        assert!(geom.owned);
        geom.owned = false;
        //sets in memory, transfers ownership even if it fails
        let rv = unsafe { gdal_sys::OGR_F_SetGeomFieldDirectly(self.c_feature, index,geom.c_geometry) };
        if rv != OGRErr::OGRERR_NONE {
            Err(ErrorKind::OgrError {
                err: rv,
                method_name: "OGR_F_SetGeomFieldDirectly",
            })?;
        }
        Ok(())
    }

    pub fn set_geometry_with_index(&mut self, geom: Geometry, index: i32) -> Result<()> {
        //sets in memory, makes a copy of geom
        let rv = unsafe { gdal_sys::OGR_F_SetGeomField(self.c_feature, index, geom.c_geometry) };
        if rv != OGRErr::OGRERR_NONE {
            Err(ErrorKind::OgrError {
                err: rv,
                method_name: "OGR_F_SetGeomField",
            })?;
        }
        Ok(())
    }

    pub fn set_field(&self, field_name: &str, value: &FieldValue) -> Result<()> {
        match *value {
            FieldValue::RealValue(value) => self.set_field_double(field_name, value),
            FieldValue::StringValue(ref value) => self.set_field_string(field_name, value.as_str()),
            FieldValue::RealListValue(ref value) => self.set_field_listf64(field_name, value),
            FieldValue::IntegerValue(value) => self.set_field_integer(field_name, value),
            FieldValue::Integer64Value(value) => self.set_field_integer64(field_name, value),

            //#[cfg(feature = "datetime")]
            FieldValue::DateTimeValue(value) => self.set_field_datetime(field_name, value),

            //#[cfg(feature = "datetime")]
            FieldValue::DateValue(value) => {
                let tz_offset = FixedOffset::east(0);
                // The known time
                let time = NaiveTime::from_hms(0, 0, 0);
                let datetime = NaiveDateTime::new(value, time);
                let dt_with_tz: DateTime<FixedOffset> = tz_offset.from_local_datetime(&datetime).unwrap();

                self.set_field_datetime(field_name, dt_with_tz)
            },
            FieldValue::Null => {
                //do nothing
                Ok(())
            }
        }
    }
    
    pub fn set_field_by_index(&self, field_index: i32, value: &FieldValue) -> Result<()> {
        match *value {
            FieldValue::RealValue(value) => self.set_field_double_by_index(field_index, value),
            FieldValue::StringValue(ref value) => self.set_field_string_by_index(field_index, value.as_str()),
            FieldValue::IntegerValue(value) => self.set_field_integer_by_index(field_index, value),
            FieldValue::Integer64Value(value) => self.set_field_integer64_by_index(field_index, value),

            //#[cfg(feature = "datetime")]
            FieldValue::DateTimeValue(value) => self.set_field_datetime_by_index(field_index, value),

            //#[cfg(feature = "datetime")]
            FieldValue::DateValue(value) => {
                let tz_offset = FixedOffset::east(0);
                // The known time
                let time = NaiveTime::from_hms(0, 0, 0);
                let datetime = NaiveDateTime::new(value, time);
                let dt_with_tz: DateTime<FixedOffset> = tz_offset.from_local_datetime(&datetime).unwrap();

                self.set_field_datetime_by_index(field_index, dt_with_tz)
            },

            FieldValue::Null => {
                //do nothing
                Ok(())
            }

            FieldValue::RealListValue(ref value) => {
                self.set_field_listf64_by_index(field_index, value)
            }
        }
    }


    pub fn set_fid(&mut self, fid: i64) -> Result<()> {
        let rv = unsafe { gdal_sys::OGR_F_SetFID(self.c_feature, fid)};
        if rv != OGRErr::OGRERR_NONE {
            Err(ErrorKind::OgrError {
                err: rv,
                method_name: "OGR_F_SetFID",
            })?;
        }
        Ok(())
    }

    pub fn create(&self, lyr: &Layer) -> Result<()> {
        let rv = unsafe { gdal_sys::OGR_L_CreateFeature(lyr.c_layer(), self.c_feature) };
        if rv != OGRErr::OGRERR_NONE {
            Err(ErrorKind::OgrError {
                err: rv,
                method_name: "OGR_L_CreateFeature",
            })?;
        }
        Ok(())
    }
}

impl<'l, 'd> Drop for Feature<'l, 'd> {
    fn drop(&mut self) {
        unsafe {
            gdal_sys::OGR_F_Destroy(self.c_feature);
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub enum FieldValue {
    IntegerValue(i32),
    Integer64Value(i64),
    StringValue(String),
    RealValue(f64),
    RealListValue(Vec<f64>),
    //In order to be serializable/deserializable
    DateValue(NaiveDate),
    DateTimeValue(DateTime<FixedOffset>),
    Null
}

impl FieldValue {
    /// Interpret the value as `String`. Panics if the value is something else.
    pub fn into_string(self) -> Option<String> {
        match self {
            FieldValue::StringValue(rv) => Some(rv),
            _ => None,
        }
    }

    /// Interpret the value as `f64`. Panics if the value is something else.
    pub fn into_real(self) -> Option<f64> {
        match self {
            FieldValue::RealValue(rv) => Some(rv),
            _ => None,
        }
    }

    /// Interpret the value as `i32`. Panics if the value is something else.
    pub fn into_int(self) -> Option<i32> {
        match self {
            FieldValue::IntegerValue(rv) => Some(rv),
            _ => None,
        }
    }

    /// Interpret the value as `Date`.
    #[cfg(feature = "datetime")]
    pub fn into_date(self) -> Option<Date<FixedOffset>> {
        match self {
            FieldValue::DateValue(rv) => Some(rv),
            FieldValue::DateTimeValue(rv) => Some(rv.date()),
            _ => None,
        }
    }

    /// Interpret the value as `DateTime`.
    #[cfg(feature = "datetime")]
    pub fn into_datetime(self) -> Option<DateTime<FixedOffset>> {
        match self {
            FieldValue::DateTimeValue(rv) => Some(rv),
            _ => None,
        }
    }
}

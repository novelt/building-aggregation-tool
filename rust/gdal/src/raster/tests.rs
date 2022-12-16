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
use crate::metadata::Metadata;
use crate::raster::{Dataset, Driver};
use gdal_sys::GDALDataType;
use std::path::Path;

#[cfg(feature = "ndarray")]
use ndarray::arr2;
use crate::raster::types::GdalType;
use crate::raster::driver::GTIFF_DRIVER;

macro_rules! fixture {
    ($name:expr) => {
        /*Path::new(file!())
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("fixtures")
            .as_path()*/
            Path::new("/rust/gdal/fixtures/")
            .join($name)
            .as_path()
    };
}

#[test]
fn test_open() {
    let dataset = Dataset::open(fixture!("tinymarble.png"), true);
    assert!(dataset.is_ok());

    let missing_dataset = Dataset::open(fixture!("no_such_file.png"), true);
    assert!(missing_dataset.is_err());
}

#[test]
fn test_get_raster_size() {
    let dataset = Dataset::open(fixture!("tinymarble.png"),true).unwrap();
    let (size_x, size_y) = dataset.size::<i32>();
    assert_eq!(size_x, 100);
    assert_eq!(size_y, 50);
}

#[test]
fn test_get_raster_block_size() {
    let band_index = 1;
    let dataset = Dataset::open(fixture!("tinymarble.png"),true).unwrap();
    let rasterband = dataset.rasterband(band_index).unwrap();
    let (size_x, size_y) = rasterband.block_size();
    assert_eq!(size_x, 100);
    assert_eq!(size_y, 1);
}

#[test]
fn test_get_raster_count() {
    let dataset = Dataset::open(fixture!("tinymarble.png"), true).unwrap();
    let count = dataset.count();
    assert_eq!(count, 3);
}

#[test]
fn test_get_projection() {
    let dataset = Dataset::open(fixture!("tinymarble.png"), true).unwrap();
    //dataset.set_projection("WGS84");
    let projection = dataset.projection();
    assert_eq!(
        projection.chars().take(16).collect::<String>(),
        "GEOGCS[\"WGS 84\","
    );
}


#[test]
fn test_get_dataset_driver() {
    let dataset = Dataset::open(fixture!("tinymarble.png"), true).unwrap();
    let driver = dataset.driver();
    assert_eq!(driver.short_name(), "PNG");
    assert_eq!(driver.long_name(), "Portable Network Graphics");
}

#[test]
fn test_get_description() {
    let driver = Driver::get("mem").unwrap();
    assert_eq!(driver.description().unwrap(), "MEM".to_string());
}

#[test]
fn test_get_metadata_item() {
    let dataset = Dataset::open(fixture!("tinymarble.png"), true).unwrap();
    let key = "None";
    let domain = "None";
    let meta = dataset.metadata_item(key, domain);
    assert_eq!(meta, None);

    let key = "INTERLEAVE";
    let domain = "IMAGE_STRUCTURE";
    let meta = dataset.metadata_item(key, domain);
    assert_eq!(meta, Some(String::from("PIXEL")));
}

#[test]
fn test_set_metadata_item() {
    let driver = Driver::get("MEM").unwrap();
    let mut dataset = driver.create("", 1, 1, 1).unwrap();

    let key = "Test_Key";
    let domain = "Test_Domain";
    let value = "Test_Value";
    let result = dataset.set_metadata_item(key, value, domain);
    assert!(result.is_ok());

    let result = dataset.metadata_item(key, domain);
    assert_eq!(Some(value.to_owned()), result);
}

#[test]
fn test_create() {
    let driver = Driver::get("MEM").unwrap();
    let dataset = driver.create("", 10, 20, 3).unwrap();
    assert_eq!(dataset.size(), (10, 20));
    assert_eq!(dataset.count(), 3);
    assert_eq!(dataset.driver().short_name(), "MEM");
}

#[test]
fn test_create_with_band_type() {
    let driver = Driver::get("MEM").unwrap();
    let dataset = driver.create_with_band_type::<&str, _>("", 10, 20, 3, f32::gdal_type(), &[]).unwrap();
    assert_eq!(dataset.size(), (10, 20));
    assert_eq!(dataset.count(), 3);
    assert_eq!(dataset.driver().short_name(), "MEM");
    assert_eq!(dataset.band_type(1).unwrap(), GDALDataType::GDT_Float32)
}

#[test]
fn test_create_copy() {
    let driver = Driver::get("MEM").unwrap();
    let dataset = Dataset::open(fixture!("tinymarble.png"), true).unwrap();
    let copy = dataset.create_copy(&driver, "").unwrap();
    assert_eq!(copy.size(), (100, 50));
    assert_eq!(copy.count(), 3);
}

#[test]
fn test_geo_transform() {
    let driver = Driver::get("MEM").unwrap();
    let dataset = driver.create("", 20, 10, 1).unwrap();
    let transform = [0., 1., 0., 0., 0., 1.];
    assert!(dataset.set_geo_transform(&transform).is_ok());
    assert_eq!(dataset.geo_transform().unwrap(), transform);
}

#[test]
fn test_get_driver_by_name() {
    let missing_driver = Driver::get("wtf");
    assert!(missing_driver.is_err());

    let ok_driver = Driver::get(GTIFF_DRIVER);
    assert!(ok_driver.is_ok());
    let driver = ok_driver.unwrap();
    assert_eq!(driver.short_name(), GTIFF_DRIVER);
    assert_eq!(driver.long_name(), "GeoTIFF");
}


#[test]
#[cfg(feature = "ndarray")]
fn test_read_raster_as_array() {
    let band_index = 1;
    let (left, top) = (19, 5);
    let (window_size_x, window_size_y) = (3, 4);

    let dataset = Dataset::open(fixture!("tinymarble.png"), true).unwrap();

    let band = dataset.rasterband(1).unwrap();

    let values = band
        .read_as_array::<u8>(
            (left, top),
            (window_size_x, window_size_y),
        )
        .unwrap();

    let data = arr2(&[
        [226, 225, 157],
        [215, 222, 225],
        [213, 231, 229],
        [171, 189, 192],
    ]);

    assert_eq!(values, data);
    assert_eq!(
        dataset.band_type(band_index).unwrap(),
        GDALDataType::GDT_Byte
    );
}


#[test]
#[cfg(feature = "ndarray")]
fn test_read_block_as_array() {
    let band_index = 1;
    let block_index = (0, 0);
    let dataset = Dataset::open(fixture!("tinymarble.png"), true).unwrap();
    let rasterband = dataset.rasterband(band_index).unwrap();
    let result = rasterband.read_block::<u8>(block_index);
    assert!(result.is_ok());
}

#[test]
#[cfg(feature = "ndarray")]
fn test_read_block_dimension() {
    let band_index = 1;
    let block = (0, 0);
    let dataset = Dataset::open(fixture!("tinymarble.png"), true).unwrap();
    let rasterband = dataset.rasterband(band_index).unwrap();
    let array = rasterband.read_block::<u8>(block).unwrap();
    let dimension = (1, 100);
    assert_eq!(array.dim(), dimension);
}

#[test]
#[cfg(feature = "ndarray")]
fn test_read_block_last_dimension() {
    let band_index = 1;
    let block = (0, 49);
    let dataset = Dataset::open(fixture!("tinymarble.png"), true).unwrap();
    let rasterband = dataset.rasterband(band_index).unwrap();
    let array = rasterband.read_block::<u8>(block).unwrap();
    let dimension = (1, 100);
    assert_eq!(array.dim(), dimension);
}

#[test]
#[cfg(feature = "ndarray")]
fn test_read_block_data() {
    let band_index = 1;
    let block = (0, 0);
    let dataset = Dataset::open(fixture!("tinymarble.png"), true).unwrap();
    let rasterband = dataset.rasterband(band_index).unwrap();
    let array = rasterband.read_block::<u8>(block).unwrap();
    assert_eq!(array[[0, 0]], 0);
    assert_eq!(array[[0, 1]], 9);
    assert_eq!(array[[0, 98]], 24);
    assert_eq!(array[[0, 99]], 51);
}

#[test]
fn test_get_band_type() {
    let driver = Driver::get("MEM").unwrap();
    let dataset = driver.create("", 20, 10, 1).unwrap();
    assert_eq!(dataset.band_type(1).unwrap(), GDALDataType::GDT_Byte);
    assert!(dataset.band_type(2).is_err());
}

#[test]
fn test_get_rasterband() {
    let driver = Driver::get("MEM").unwrap();
    let dataset = driver.create("", 20, 10, 1).unwrap();
    let rasterband = dataset.rasterband(1);
    assert!(rasterband.is_ok())
}

#[test]
fn test_get_no_data_value() {
    let dataset = Dataset::open(fixture!("tinymarble.png"), true).unwrap();
    let rasterband = dataset.rasterband(1).unwrap();
    let no_data_value = rasterband.no_data_value();
    assert!(no_data_value.is_none());

    // let dataset = Dataset::open(fixture!("bluemarble.tif")).unwrap();
    // let rasterband = dataset.get_rasterband(1).unwrap();
    // let no_data_value = rasterband.get_no_data_value();
    // assert_eq!(no_data_value, Some(0.0));
}

#[test]
fn test_set_no_data_value() {
    let driver = Driver::get("MEM").unwrap();
    let dataset = driver.create("", 20, 10, 1).unwrap();
    let rasterband = dataset.rasterband(1).unwrap();
    assert_eq!(rasterband.no_data_value(), None);
    assert!(rasterband.set_no_data_value(3.14).is_ok());
    assert_eq!(rasterband.no_data_value(), Some(3.14));
}

#[test]
fn test_get_rasterband_size() {
    let dataset = Dataset::open(fixture!("tinymarble.png"), true).unwrap();
    let rasterband = dataset.rasterband(1).unwrap();
    let size = rasterband.size();
    assert_eq!(size, (100, 50));
}

#[test]
fn test_get_rasterband_block_size() {
    let dataset = Dataset::open(fixture!("tinymarble.png"), true).unwrap();
    let rasterband = dataset.rasterband(1).unwrap();
    let size = rasterband.block_size();
    assert_eq!(size, (100, 1));
}

#[test]
#[cfg(feature = "gdal_2_2")]
fn test_get_rasterband_actual_block_size() {
    let dataset = Dataset::open(fixture!("tinymarble.png"), true).unwrap();
    let rasterband = dataset.rasterband(1).unwrap();
    let size = rasterband.actual_block_size((0, 40));
    assert_eq!(size.unwrap(), (100, 1));
}

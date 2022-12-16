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
use crate::raster::{Raster, create_empty_raster, RasterStats};
use gdal::raster::types::GdalType;
use std::path::{PathBuf, Path};
use anyhow::Result;
use uuid::Uuid;

pub fn get_temp_filename(file_name: &str) -> PathBuf {
    ["/modules/temp", &Uuid::new_v4().to_string(), file_name].iter().collect()
}


pub fn create_test_raster<T:Copy + GdalType>(in_file_name: &str, input_raster_stats: &RasterStats, input_raster_data: &Vec<T>) -> Result<PathBuf> {
    create_test_raster_with_path(
        &get_temp_filename(in_file_name),
            input_raster_stats, input_raster_data)
}

pub fn create_test_raster_with_path<T:Copy + GdalType>(input_path: &Path, input_raster_stats: &RasterStats, input_raster_data: &Vec<T>) -> Result<PathBuf> {

    assert!(!input_path.exists());

    create_empty_raster(&input_path, input_raster_stats, false).unwrap();

    assert!(input_path.exists());

    {
        let input_raster = Raster::read(&input_path, false);

        let input_raster_band = input_raster.dataset.rasterband(1)?;

        let num_rows = input_raster_stats.num_rows;
        let num_cols = input_raster_stats.num_cols;

        input_raster_band.write((0, 0), (num_cols as i32, num_rows as i32),
                                &input_raster_data).unwrap();
    }

    Ok(input_path.to_path_buf())
}
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
use std::path::{PathBuf, Path};
use gdal::raster::{Dataset, RasterBand};

mod raster_stats;
mod raster_resample;
mod burn_polygon;
mod algo;
pub mod combine_rasters;

//#[cfg(test)]
mod test_util;

pub use raster_stats::*;
pub use burn_polygon::*;
pub use algo::*;
pub use raster_resample::*;
pub use combine_rasters::*;
//#[cfg(test)]
pub use test_util::*;


pub struct Raster
{
    pub path: PathBuf,
    pub stats: RasterStats,
    pub dataset: Dataset,
}

impl Raster {
    pub fn read(path: &Path, readonly: bool) -> Raster {
        //println!("Reading raster at path {:?}", path);

        let dataset = Dataset::open(path, readonly).unwrap();

        let band: RasterBand = dataset.rasterband(1).unwrap();

        let stats = RasterStats::new(&dataset, &band);

        Raster {
            path: path.to_path_buf(),
            stats,
            dataset,
        }
    }

    pub fn band(&self) -> RasterBand {
        self.dataset.rasterband(1).unwrap()
    }
}

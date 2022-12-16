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
use crate::raster::Raster;
use gdal::raster::{Driver, GDALDataType};
//use gdal::spatial_ref::SpatialRef;
use anyhow::{Result};
use crate::io::InputOgrLayer;
use gdal::vector::Dataset;
use gdal::raster::global_func::rasterize;
use std::fs::create_dir_all;
use gdal::raster::driver::{DEFAULT_RASTER_OPTIONS, GTIFF_DRIVER};
use std::path::Path;
use log::{debug,};

pub fn rasterize_polygon(
    snap_raster: &Path, input_layer: &InputOgrLayer, output_path: &Path,
    all_touched: bool,
    gdal_type: GDALDataType::Type,
    fid_field: Option<&str>,
    no_data_value: f64
) -> Result<()>
{
    let snap_raster = Raster::read(snap_raster, true);
    debug!("Use snap raster: {:?} with stats {}", snap_raster.path, snap_raster.stats);

    if !output_path.parent().unwrap().is_dir() {
        create_dir_all(output_path.parent().unwrap())?;
    }

    let drv = Driver::get(GTIFF_DRIVER)?;

    //Create the raster with appropriate projection, no data value, datatype, etc.
    {
        debug!("Creating output tif {:?}", output_path);

        //just want to create it and close it
        let ds = drv.create_with_band_type(
            output_path.to_str().unwrap(),
            snap_raster.stats.num_cols as isize,
            snap_raster.stats.num_rows as isize, 1, gdal_type,
            &DEFAULT_RASTER_OPTIONS
            )?;

        debug!("Created output tif {:?}", output_path);

        let output_raster_band = ds.rasterband(1)?;

        debug!("setting no data to 0");
        output_raster_band.set_no_data_value(no_data_value)?;
        output_raster_band.fill(no_data_value)?;

        let left = snap_raster.stats.origin_x;
        let top = snap_raster.stats.origin_y;
        let raster_tile_size_x = snap_raster.stats.pixel_width;
        let raster_tile_size_y = snap_raster.stats.pixel_height;

        //because y is the top not the bottom
        assert!(raster_tile_size_y < 0.0);
        debug!("setting geo transform & projection");
        ds.set_geo_transform(&[left, raster_tile_size_x, 0.0, top, 0.0, raster_tile_size_y])?;
        ds.set_projection(&snap_raster.stats.projection)?;

        debug!("Set projection to {}", &ds.projection());

    }

    //let output_raster = Raster::read(output_path.to_path_buf(), false);

    let dataset = Dataset::open(&input_layer.ogr_conn_str).unwrap();

    let layer_name = if input_layer.layer_name.is_empty() {
        dataset.layer(0)?.name()
    } else {
        input_layer.layer_name.clone()
    };

    /*
    cmd_line_components = [
        os.path.join(cfg.OSGEO_BIN_DIR, "gdal_rasterize"),
        "-a id",
        f"-l \"{schema_name}.{table_name}\"",
        'PG:"%s"' % (geo_db_utils.get_gdal_connection_string(cfg, cfg.LOCAL_PREFIX),),
        "\"{}\"".format(feature_tif_path)
    ]
     */

    let the_fid_field = fid_field.unwrap_or("FID");
    let mut sql = format!("SELECT {} as the_fid FROM \"{}\"",
                          &the_fid_field,
                          &layer_name);
    if let Some(af) = input_layer.attribute_filter.as_ref() {
        sql.push_str(" WHERE ");
        sql.push_str(af);
    }

    let mut options :Vec<&str> = vec![
        "-a", "the_fid",
        "-sql",
        &sql,
        //Use this dialect so we can use FID even with SQL sources
        "-dialect",
        "OGRSQL"
    ];

    if all_touched {
        options.push("-at");
    }

    rasterize(
        &dataset,&output_path, &options
    )?;

    debug!("Done rasterize");

    Ok(())
}
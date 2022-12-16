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
use anyhow::Result;
use std::time::Instant;
use gdal::version_info;
use std::path::PathBuf;
use geo_util::io::InputOgrLayer;
use geo_util::raster::rasterize_polygon;
use geo_util::util::format_duration;
use structopt::StructOpt;
use std::fs::remove_file;
use gdal::raster::types::convert_string_to_gdal_type;
use log::{debug,};

#[derive(StructOpt)]
pub struct PolygonArgs {

    #[structopt(long)]
    layer_name: Option<String>,

    #[structopt(long)]
    ogr_conn_str: String,

    #[structopt(parse(from_os_str), long)]
    output_raster: PathBuf,

    #[structopt(parse(from_os_str), long)]
    snap_raster: PathBuf,

    #[structopt(long)]
    all_touched: bool,

    #[structopt(long)]
    clean: bool,

    #[structopt(long, help="data type, can be one of Byte/Int16/UInt16/UInt32/Int32/Float32/Float64/
            CInt16/CInt32/CFloat32/CFloat64")]
    data_type: Option<String>,

    #[structopt(long, help="Field to burn, default is the fid")]
    burn_field: Option<String>,

    #[structopt(long, help="Optional SQL filter to filter the input")]
    attribute_filter: Option<String>,

    #[structopt(long, default_value="0.0")]
    no_data_value: f64
}

pub fn burn_polygon_to_raster(args: &PolygonArgs) -> Result<()> {
    let now = Instant::now();

    let version_text = version_info("--version");

    debug!("GDAL version: {}", version_text);

    if args.clean && args.output_raster.exists() {
        debug!("Removing/cleaning Output {:?}", &args.output_raster);
        remove_file(&args.output_raster)?;
    }

    if args.output_raster.exists() {
        debug!("Output {:?} already exists", &args.output_raster);
        return Ok(());
    }

    let input_ogr_layer = InputOgrLayer {
        name: "".to_string(),
        layer_name: args.layer_name.clone().unwrap_or("".to_string()),
        ogr_conn_str: args.ogr_conn_str.to_string(),
        attribute_filter: args.attribute_filter.clone(),
        ..Default::default()
    };

    let gdal_type = convert_string_to_gdal_type(args.data_type.as_deref().unwrap_or( "UInt32")).unwrap();

    rasterize_polygon(
        &args.snap_raster,
        &input_ogr_layer,
        &args.output_raster,
        args.all_touched,
        gdal_type,
        args.burn_field.as_deref(),
        args.no_data_value
    )?;

    debug!("Finished in {}", format_duration(now.elapsed()));

    Ok(())
}
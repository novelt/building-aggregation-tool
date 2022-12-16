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
use std::path::PathBuf;
use std::fs::{remove_file};
use structopt::StructOpt;
use crate::lhs_rhs_args::LhsRhsArgs;
use geo_util::raster::combine_rasters::combine_rasters;
use geo_util::raster::Raster;

#[derive(StructOpt)]
pub struct SetNoDataIfRhsArgs {

    #[structopt(flatten)]
    lhs_rhs: LhsRhsArgs,

    #[structopt(long, parse(from_os_str))]
    output: PathBuf,

    #[structopt(long)]
    clean: bool,

}


pub fn set_nodata_if_rhs(args: &SetNoDataIfRhsArgs) -> Result<()> {

    if args.clean && args.output.exists() {
        remove_file(&args.output)?;
    }

    if args.output.exists() {
        println!("{:?} already exists and --clean not passed, doing nothing", &args.output);
        return Ok(());
    }

    let nodata_value = {
        let input_stats = Raster::read(&args.lhs_rhs.raster_lhs, true);
        input_stats.stats.no_data_value
    };

    let nd_typed = nodata_value as i32;

    combine_rasters(
        &args.lhs_rhs.raster_lhs,
        &args.lhs_rhs.raster_rhs,
        &args.output,
        nodata_value,
        |v1: i32, _is_left_nodata, _v2, is_right_nodata| {

            Ok( if is_right_nodata { v1 } else { nd_typed } )
        }
    )?;


    Ok(())
}


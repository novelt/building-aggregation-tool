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
//#![allow(warnings)]

mod cmd_floodfill;
mod cmd_filter_using_raster;
mod cmd_fix_reproject_split;
mod cmd_fill_center_building_groups;
mod cmd_count_area_and_total;

mod cmd_check_raster_center;
mod cmd_create_centroids;
mod cmd_create_corners;
mod raster_center_dto;

use crate::cmd_floodfill::{flood_fill, FloodFillArgs};

use anyhow::Result;
use log::LevelFilter;
use simple_logger::SimpleLogger;
use structopt::StructOpt;
use crate::cmd_check_raster_center::{check_raster_center, CheckRasterCenterArgs};
use crate::cmd_count_area_and_total::{count_area_total, CountAreaTotalArgs};
use crate::cmd_create_centroids::{create_centroid, CreateCentroidArgs};
use crate::cmd_create_corners::{create_corners, CreateCornersArgs};
use crate::cmd_fill_center_building_groups::{fill_center_building_groups, FillCenterBuildingGroupsArgs};
use crate::cmd_filter_using_raster::{filter_using_raster, FilterUsingRasterArgs};
use crate::cmd_fix_reproject_split::{fix_reproject_split, FixReprojectSplitArgs};

#[derive(StructOpt)]
struct Cli {

    #[structopt(long, default_value = "Warn")]
    log_level: LevelFilter,

    #[structopt(subcommand)]  // Note that we mark a field as a subcommand
    cmd: Command
}

#[derive(StructOpt)]
enum Command {


    #[structopt(help="Groups near polygons to multipolygons")]
    FloodFill(FloodFillArgs),

    #[structopt(help="Faster intersects test using an already rasterized vector")]
    FilterUsingRaster(FilterUsingRasterArgs),

    FixReprojectSplit(FixReprojectSplitArgs),

    FillCenterBuildingGroups(FillCenterBuildingGroupsArgs),

    CountAreaAndTotal(CountAreaTotalArgs),

    CheckRasterCenter(CheckRasterCenterArgs),

    CreateCentroids(CreateCentroidArgs),

    CreateCorners(CreateCornersArgs),
}

fn run() -> Result<()> {

    let args = Cli::from_args();

    SimpleLogger::new().with_level(args.log_level).init()?;

    match &args.cmd {

        Command::FilterUsingRaster(r) => {
            filter_using_raster(r)?;
        }
        Command::FloodFill(r) => {
            flood_fill(r)?;
        }
        Command::FixReprojectSplit(r) => {
            fix_reproject_split(r)?;
        }
        Command::FillCenterBuildingGroups(r) => {
            fill_center_building_groups(r)?;
        }
        Command::CountAreaAndTotal(r) => {
            count_area_total(r)?;
        }
        Command::CheckRasterCenter(r) => {
            check_raster_center(r)?;
        }
        Command::CreateCentroids(r) => {
            create_centroid(r)?;
        }
        Command::CreateCorners(r) => {
            create_corners(r)?;
        }
    }

    Ok(())
}






fn main() {
    run().unwrap();
}
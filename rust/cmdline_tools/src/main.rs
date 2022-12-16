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
//#![allow(warnings, unused)]
use anyhow::Result;
use log::LevelFilter;
use simple_logger::SimpleLogger;
use structopt::StructOpt;


use crate::cmd_ratio_raster::{RatioArgs, create_ratio_raster};
use crate::cmd_diff_raster::{DiffArgs, create_diff_raster};
use crate::cmd_burn_polygon_to_raster::{PolygonArgs, burn_polygon_to_raster};
use crate::cmd_fix_geom::{FixGeomArgs, run_fix_geom};
use crate::lhs_rhs_args::LhsRhsArgs;
use crate::cmd_set_no_data::{NoDataArgs, set_no_data};
use crate::cmd_raster_stats::print_stats;
use crate::cmd_count_to_raster::{CountToRasterCli, burn_count_to_raster};
use crate::cmd_contour::ContourArgs;
use crate::cmd_contour::main_contour_lines;
use crate::cmd_get_projected_extent::{get_projected_extent, GetProjectedExtentArgs};

use crate::cmd_group_to_multi::{group_to_multi, GroupToMultiArgs, };
use crate::cmd_id_set_cmp::{id_set_cmp, IdSetCmpArgs};
use crate::cmd_ra_hf_csv::{ra_hf_csv, RaHfCsvArgs};
use crate::cmd_merge_vector_layers_fast::{MergeVectorFastArgs, merge_vectors_fast};
use crate::cmd_set_nodata_if_rhs::{set_nodata_if_rhs, SetNoDataIfRhsArgs};
use crate::cmd_test_projection::{test_projection, TestProjectionArgs};
use crate::cmd_to_single::{to_single, ToSingleArgs};

mod cmd_burn_polygon_to_raster;
mod cmd_count_to_raster;
mod cmd_diff_raster;
mod cmd_fix_geom;
mod cmd_raster_stats;
mod cmd_ratio_raster;
mod cmd_set_no_data;
mod lhs_rhs_args;
mod cmd_contour;
mod cmd_merge_vector_layers_fast;

mod cmd_to_single;
mod cmd_set_nodata_if_rhs;
mod cmd_group_to_multi;
mod cmd_get_projected_extent;
mod contour;
mod cmd_ra_hf_csv;
mod cmd_test_projection;
mod cmd_id_set_cmp;

#[derive(StructOpt)]
struct Cli {

    #[structopt(long, default_value = "Warn")]
    log_level: LevelFilter,

    #[structopt(subcommand)]  // Note that we mark a field as a subcommand
    cmd: Command
}

#[derive(StructOpt)]
enum Command {
    #[structopt(help="Prints statistics on 2 rasters.  Total, pairwise diff, abs pairwise diff")]
    Stats(LhsRhsArgs),
    #[structopt(help="Divides one raster by another, outputs ratio raster")]
    Ratio(RatioArgs),
    #[structopt(help="Subtracts one raster by another, outputs diff raster and QGIS color map")]
    Diff(DiffArgs),
    #[structopt(help="Sets Nodata in 1 raster according to if NoData is in another raster.  Used to clip pop rasters to country")]
    SetNoData(NoDataArgs),

    SetNoDataIfRhs(SetNoDataIfRhsArgs),

    #[structopt(help="Burns geometry")]
    BurnPolygonToRaster(PolygonArgs),
    #[structopt(help="Creates an FGB (FlatGeoBuff) with geometry corrections")]
    FixGeom(FixGeomArgs),
    #[structopt(help="Burns point count to a raster")]
    BurnCountToRaster(CountToRasterCli),
   

    #[structopt(help="Merge polygon vector layers together, outputs fixed (valid) non curved multi polygons.  Assumes inputs do not overlap")]
    MergeVectorFast(MergeVectorFastArgs),

    #[structopt(help="Create contour lines")]
    Contours(ContourArgs),


    #[structopt(help="Multi to Single Polygons")]
    ToSingle(ToSingleArgs),

    #[structopt(help="Groups near polygons to multipolygons")]
    GroupToMulti(GroupToMultiArgs),

    GetProjectedExtent(GetProjectedExtentArgs),

    RaHfCsv(RaHfCsvArgs),

    TestProjection(TestProjectionArgs),

    IdSetCmp(IdSetCmpArgs),
}



fn run() -> Result<()> {
    let args = Cli::from_args();

    SimpleLogger::new().with_level(args.log_level).init()?;

    match &args.cmd {
        Command::Stats(r) => {
            print_stats(r)?;
        },
        Command::Ratio(r) => {
            create_ratio_raster(r)?;
        },
        Command::Diff(r) => {
            create_diff_raster( r)?;
        },
        Command::SetNoData(r) => {
            set_no_data(r)?;
        },
        Command::SetNoDataIfRhs(r) => {
            set_nodata_if_rhs(r)?;
        },
        Command::BurnPolygonToRaster(r) => {
            burn_polygon_to_raster(r)?;
        },
        Command::FixGeom(r) => {
            run_fix_geom(r)?;
        },
        Command::BurnCountToRaster(r) => {
            burn_count_to_raster(r)?;
        },        
        Command::MergeVectorFast(r) => {
            merge_vectors_fast(r)?;
        }
        Command::Contours(r) => {
            main_contour_lines(r)?;
        }
        Command::ToSingle(r) => {
            to_single(r)?;
        }
        Command::GroupToMulti(r) => {
            group_to_multi(r)?;
        }
        Command::GetProjectedExtent(r) => {
            get_projected_extent(r)?;
        }
        Command::RaHfCsv(r) => {
            ra_hf_csv(r)?;
        }
        Command::TestProjection(r) => {
            test_projection(r)?;
        }
        Command::IdSetCmp(r) => {
            id_set_cmp(r)?;
        }
    }

    Ok(())
}






fn main() {
    run().unwrap();
}

#[cfg(test)]
mod cmdline_tools_tests {




}


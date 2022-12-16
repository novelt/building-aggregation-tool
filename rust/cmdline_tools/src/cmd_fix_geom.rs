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
use geo_util::vector::run_fix_geometry;
use structopt::StructOpt;

#[derive(StructOpt)]
pub struct FixGeomArgs {

    #[structopt(long)]
    input_dataset: String,

    #[structopt(long)]
    input_layer: String,

    #[structopt(parse(from_os_str), long)]
    output_dataset: PathBuf,

    #[structopt(long, default_value = "FlatGeobuf")]
    output_format: String,

}

pub fn run_fix_geom(args: &FixGeomArgs) -> Result<()> {
    run_fix_geometry(
        &args.input_dataset,
        &args.input_layer,
        args.output_dataset.to_str().unwrap(),
        args.output_dataset.file_stem().unwrap().to_str().unwrap(),
        &args.output_format,
    )
}
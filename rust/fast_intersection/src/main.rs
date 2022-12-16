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
//extern crate geo_types;
//extern crate geo_booleanop;
use simple_logger::SimpleLogger;
use anyhow::Result;
use log::{LevelFilter};
use structopt::StructOpt;
use crate::cmd_intersect::{intersect, IntersectArgs};
use crate::cmd_prepare::{prepare, PrepareArgs};

mod cmd_prepare;
mod cmd_intersect;


#[derive(StructOpt)]
struct Cli {

    #[structopt(long, default_value = "Warn")]
    log_level: LevelFilter,

    #[structopt(subcommand)]  // Note that we mark a field as a subcommand
    cmd: Command
}

#[derive(StructOpt)]
enum Command {
    #[structopt(help="")]
    Prepare(PrepareArgs),

    Intersect(IntersectArgs),
}



fn run() -> Result<()> {

    let args = Cli::from_args();

    SimpleLogger::new().with_level(args.log_level).init()?;

    match &args.cmd {
        Command::Prepare(r) => {
            prepare(r)?;
        },
        Command::Intersect(r) => {
            intersect(r)?;
        }

    }

    Ok(())
}






fn main() {
    run().unwrap();
}

#[cfg(test)]
mod fast_intersection_tests {




}


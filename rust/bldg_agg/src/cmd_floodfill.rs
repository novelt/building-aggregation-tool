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
use std::collections::VecDeque;
use std::path::PathBuf;
use structopt::StructOpt;
use anyhow::Result;
use geo_util::raster::{create_empty_raster, Raster};
use bitvec::prelude::*;
use gdal::raster::types::GdalType;
use geo_util::util::RasterChunkIterator;

#[derive(StructOpt)]
pub struct FloodFillArgs {
    #[structopt(parse(from_os_str), long)]
    input_raster: PathBuf,

    #[structopt(parse(from_os_str), long)]
    output_raster: PathBuf,

    // #[structopt(parse(from_os_str), long)]
    // output_bitvec: PathBuf,

}

pub fn read_raster(args: &FloodFillArgs) -> Result<BitVec>
{
    let input_raster = Raster::read(&args.input_raster, true);

    let stats = &input_raster.stats;

    let mut bv = BitVec::new();

    bv.resize( (stats.num_rows * stats.num_cols) as usize, false);

    for raster_window in RasterChunkIterator::new(
        stats.num_rows as i32, stats.num_cols as i32, 10)
    {
        let data = input_raster.band().read_as::<u32, i32>(raster_window.window_offset,
                                                raster_window.window_size)?;

        for (idx, value) in data.iter().enumerate() {

            if *value == 0 {
                continue;
            }
            let win_x = idx % (raster_window.window_size.0 as usize);
            let win_y = idx / (raster_window.window_size.0 as usize);

            let x = win_x + raster_window.window_offset.0 as usize;
            let y = win_y + raster_window.window_offset.1 as usize;

            let bv_index = x + y * stats.num_cols as usize;

            bv.set(bv_index, true);

        }

    }
    Ok(bv)
}

pub fn flood_fill(args: &FloodFillArgs) -> Result<()>
{

    let mut stats = {
        let input_raster = Raster::read(&args.input_raster, true);
        input_raster.stats.clone()
    };

    let bv = read_raster(args)?;

    let mut bv_output = BitVec::<Msb0, u8>::new();
    bv_output.resize( (stats.num_rows * stats.num_cols) as usize, true);

    let mut bv_seen = BitVec::<Msb0, u8>::new();
    bv_seen.resize( (stats.num_rows * stats.num_cols) as usize, false);
    //Flood fill this guy, we allow the coordinates to be just outside too, anything we can reach on the outside
    //is false

    let mut deq = VecDeque::new();

    let num_cols = stats.num_cols as isize;
    let num_rows = stats.num_rows as isize;

    //seed with top row
    for x in 0..num_cols {
        deq.push_back(x);
    }

    while !deq.is_empty() {

        let current_idx = deq.pop_front().unwrap();
        if bv_seen[current_idx as usize] {
            continue;
        }

        bv_seen.set(current_idx as usize, true);

        //If we have buildings, we can't move
        if bv[current_idx as usize] {
            continue;
        }

        bv_output.set(current_idx as usize, false);

        let y = current_idx / num_cols;
        let x = current_idx % num_cols;

        for dx in -1..=1 {
            let try_x = x + dx;
            if try_x < 0 || try_x >= num_cols {
                continue;
            }
            for dy in -1..=1 {
                let try_y = y + dy;
                if try_y < 0 || try_y >= num_rows {
                    continue;
                }

                let try_index = try_x + try_y * num_cols;
                if bv_seen[try_index as usize] {
                    continue;
                }

                deq.push_back(try_index);

            }
        }
    }



    stats.no_data_value = 0.;  //0 is not a settlement more or less
    stats.gdal_type = u8::gdal_type();

    create_empty_raster(&args.output_raster, &stats, false)?;

    let output_raster = Raster::read(&args.output_raster, false);
    let output_band = output_raster.band();

    for raster_window in RasterChunkIterator::new(
        stats.num_rows as i32, stats.num_cols as i32, 10)
    {
        let cap = (raster_window.window_size.0 * raster_window.window_size.1) as usize;
        let mut data = Vec::with_capacity(cap);

        for idx in 0..cap {
            let window_x = idx % raster_window.window_size.0 as usize;
            let window_y = idx / raster_window.window_size.0 as usize;
            let x = raster_window.window_offset.0 as usize + window_x;
            let y = raster_window.window_offset.1 as usize + window_y;
            let bv_index = y * num_cols as usize + x;

            if bv_output[bv_index] {
                data.push(1u8);
            } else {
                data.push(0u8);
            }
        }

        output_band.write(
            raster_window.window_offset ,
            raster_window.window_size, &data)?;
    }
    Ok(())
}
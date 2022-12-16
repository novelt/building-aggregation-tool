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
use std::path::PathBuf;
use std::time::Instant;
use anyhow::{Result};
use gdal::vector::{Driver, Feature, OGRwkbGeometryType};
use structopt::StructOpt;
use geo_util::util::{print_remaining_time};

#[derive(StructOpt)]
pub struct CreateCentroidArgs {
    #[structopt(long, parse(from_os_str), help="Building")]
    pub(crate) in_fgb: PathBuf,

    #[structopt(long, parse(from_os_str), help="Centroid")]
    pub(crate) out_fgb: PathBuf,

}

pub fn create_centroid(args: &CreateCentroidArgs) -> Result<()>
{
    let fgb_driver = Driver::get(Driver::DRIVER_NAME_FLATGEOBUF)?;
    let ds_slice = fgb_driver.open(args.in_fgb.to_str().unwrap(), true ).unwrap();
    let lyr = ds_slice.layer(0)?;

    let now  = Instant::now();
    let total_to_process = lyr.count(false);
    let mut last_output = Instant::now();

    let in_proj = lyr.spatial_reference()?;

    let out_ds = fgb_driver.create(&args.out_fgb.to_str().unwrap())?;
    let out_lyr = out_ds.create_layer_ext::<String>(
        &args.out_fgb.file_stem().unwrap().to_str().unwrap(),
        &in_proj,
        OGRwkbGeometryType::wkbPoint,
        &[]
        // &vec!["GEOMETRY_NAME=shape".to_string(),
        // "FID=id".to_string()
        // ],
    )?;


    let out_def = out_lyr.layer_definition();

    for (f_idx,f) in lyr.features().enumerate() {
        if last_output.elapsed().as_secs() >= 3 {
            last_output = Instant::now();
            print_remaining_time(&now, f_idx as _, total_to_process as _);
        }

        let g = f.geometry().as_geom();
        let centroid = g.centroid()?;

        let mut out_ft = Feature::new(&out_def).unwrap();

        out_ft.set_geometry_directly(centroid).unwrap();

        // Add the feature to the layer:
        out_ft.create(&out_lyr).unwrap();
    }

    Ok(())
}
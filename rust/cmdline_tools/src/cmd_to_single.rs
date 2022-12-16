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
use anyhow::{Result};
use gdal::spatial_ref::{SpatialRef, OSRAxisMappingStrategy};
use gdal::vector::{Driver, Feature, OGRwkbGeometryType,
                   Dataset,
};
use structopt::StructOpt;

use std::time::Instant;
use geo_util::util::print_remaining_time;

/*
A faster geospatial filter

Any polygon whose extremes are inside the raster are filtered out,
everything else the shape is written to the output
 */

#[derive(StructOpt)]
pub struct ToSingleArgs {
    //Note this isn't generic with respect to the fields, so for now keeping osm & dg
    #[structopt(long, help = "OGR Connection string for inputs")]
    pub(crate) in_ogr_conn: String,

    #[structopt(long, help = "Layer names for input, use - to use default, and all for everything")]
    pub(crate) in_ogr_layer: String,

    #[structopt(long)]
    pub(crate) out_ogr_conn: String,

    #[structopt(long)]
    pub(crate) out_ogr_layer: String,

}


/// Combines building layers, in order of priority, not including ones that intersect an already existing building
pub fn to_single(args: &ToSingleArgs) -> Result<()> {
    println!("Starting merging vector layers");

    let now = Instant::now();
    let mut last_output = Instant::now();

    let mut target_sr = SpatialRef::from_epsg(4326).unwrap();
    target_sr.set_axis_mapping_strategy(OSRAxisMappingStrategy::OAMS_TRADITIONAL_GIS_ORDER);

    let drv = Driver::get(Driver::DRIVER_NAME_FLATGEOBUF)?;

    let mut n_processed = 0;

    let ds = drv.create(&args.out_ogr_conn)?;

    let output_lyr = ds.create_layer_ext::<String>(
        &args.out_ogr_layer,
        &target_sr,
        OGRwkbGeometryType::wkbPolygon,
        &vec![],
    )?;

    let output_layer_def = output_lyr.layer_definition();

    let ds = Dataset::open(&args.in_ogr_conn)?;
    let lyr = ds.layer_by_name(&args.in_ogr_layer)?;

    let total_to_process = lyr.count(false);


    for input_feature in lyr.features()
    {
        if last_output.elapsed().as_secs() >= 3 {
            last_output = Instant::now();
            print_remaining_time(&now, n_processed, total_to_process as u32);
        }

        n_processed += 1;

        let geom = input_feature.geometry().as_geom();

        assert!(geom.is_valid());

        let geometry_type = geom.geometry_type();

        match geometry_type {
            OGRwkbGeometryType::wkbPolygon => {
                let mut ft = Feature::new(&output_layer_def)?;
                ft.set_geometry(geom)?;
                ft.create(&output_lyr)?;
            }
            OGRwkbGeometryType::wkbMultiPolygon => {
                let poly_count = geom.geometry_count();
                for p in 0..poly_count {
                    let poly = geom.get_geometry(p);
                    let mut ft = Feature::new(&output_layer_def)?;
                    ft.set_geometry(poly)?;
                    ft.create(&output_lyr)?;
                }
            }
            _ => {
                continue;
            }
        }
    }


    println!("Wrote {}", n_processed);
    Ok(())
}


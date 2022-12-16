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
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use std::time::Instant;
use anyhow::{Result};
use itertools::Itertools;

use gdal::vector::{Driver, Feature, Geometry, GeometryIntersection, OGRwkbGeometryType};
use structopt::StructOpt;
use log::{debug, info};
use gdal::spatial_ref::{OSRAxisMappingStrategy, SpatialRef};

use geo_util::raster::{Raster, RasterStats};
use geo_util::util::print_remaining_time_msg;
use crate::raster_center_dto::{Corners, RasterCenterDto};

#[derive(StructOpt)]
pub struct CreateCornersArgs {

    #[structopt(long, parse(from_os_str), help="Building Centroid FGB Path")]
    pub(crate) out_fgb: PathBuf,

    #[structopt(long, parse(from_os_str), )]
    pub(crate) in_bin: PathBuf,

    #[structopt(long, parse(from_os_str))]
    pub(crate) snap_raster: PathBuf,

}

fn create_square(x_offset: f64, y_offset: f64) -> Result<Geometry> {
    let mut gdal_geom = Geometry::empty(OGRwkbGeometryType::wkbPolygon)?;
    let mut exterior_ring = Geometry::empty(OGRwkbGeometryType::wkbLinearRing)?;

    exterior_ring.set_point_2d(0, (0.0+x_offset, 0.0+y_offset));
    exterior_ring.set_point_2d(1, (1.0+x_offset, 0.0+y_offset));
    exterior_ring.set_point_2d(2, (1.0+x_offset, 1.0+y_offset));
    exterior_ring.set_point_2d(3, (0.0+x_offset, 1.0+y_offset));
    exterior_ring.set_point_2d(4, (0.0+x_offset, 0.0+y_offset));

    gdal_geom.add_geometry(exterior_ring)?;

    Ok(gdal_geom)
}

fn create_template_shapes() -> Result<Vec<Geometry>> {
    let geom = Geometry::from_x_y(0.,0.)?;

    let buffered = geom.buffer(0.8, 4)?;

    //now intersect each cuadrant, order in Corners
    let mut squares = Vec::with_capacity(4);

    squares.push(create_square(0.0, 0.0)?);
    squares.push( create_square(0.0, -1.0)?);
    squares.push( create_square(-1.0, -1.0)?);
    squares.push( create_square(-1.0, 0.0)?);

    let buffers = squares.iter().map(|s| {
        debug!("Intersecting {} and {}", s.wkt().unwrap(), buffered.wkt().unwrap());
        buffered.intersection(&s).unwrap()
    }).collect_vec();

    Ok(buffers)
}

fn create_shape(
    r: &RasterCenterDto,
    stats: &RasterStats,
    templates: &Vec<Geometry>
) -> Result<Geometry> {

    let multiplier = stats.pixel_width;

    //center is at 0,0 ; we need to calculate the translation x y values

    let starting_template = match r.corner {
        Corners::NorthEast => {
            //we need opposite one
            &templates[Corners::SouthWest as usize]
        }
        Corners::SouthEast => {
            &templates[Corners::NorthWest as usize]
        }
        Corners::SouthWest => {
            &templates[Corners::NorthEast as usize]
        }
        Corners::NorthWest => {
            &templates[Corners::SouthEast as usize]
        }
    };

    let translation_xy = match r.corner {
     Corners::NorthEast => {
         (stats.calc_x_coord(r.raster_x+1),
            stats.calc_y_coord(r.raster_y) )
        }
        Corners::SouthEast => {
            (stats.calc_x_coord(r.raster_x+1),
            stats.calc_y_coord(r.raster_y+1) )
        }
        Corners::SouthWest => {
            (stats.calc_x_coord(r.raster_x),
            stats.calc_y_coord(r.raster_y+1) )
        }
        Corners::NorthWest => {
            (stats.calc_x_coord(r.raster_x),
            stats.calc_y_coord(r.raster_y) )
        }
    };

    assert_eq!(starting_template.geometry_type(), OGRwkbGeometryType::wkbPolygon);
    let template_ring = starting_template.get_geometry(0);
    //assert_eq!(template_ring.geometry_type(), OGRwkbGeometryType::wkbLinearRing);

    let mut gdal_geom = Geometry::empty(OGRwkbGeometryType::wkbPolygon)?;
    let mut exterior_ring = Geometry::empty(OGRwkbGeometryType::wkbLinearRing)?;

    for point_num in 0..template_ring.point_count() {
        let [x, y] = template_ring.get_point(point_num as _);
        let x_new = x * multiplier + translation_xy.0;
        let y_new = y * multiplier + translation_xy.1;

        exterior_ring.set_point_2d(point_num, (x_new, y_new));
    }

    gdal_geom.add_geometry(exterior_ring)?;


    Ok(gdal_geom)

}

pub fn create_corners(args: &CreateCornersArgs) -> Result<()>
{
    info!("Starting check_raster_center...");

    // let mut rdr = csv::ReaderBuilder::new()
    //     .has_headers(true)
    //     .from_path(&args.in_csv).unwrap();
    // let records: Vec<RasterCenterDto> = rdr.deserialize().filter_map(|f| f.ok()).collect_vec();

    let mut reader = BufReader::new(File::open(&args.in_bin).unwrap());
    let records: Vec<RasterCenterDto> = bincode::deserialize_from(&mut reader).unwrap();

    let fgb_driver = Driver::get(Driver::DRIVER_NAME_FLATGEOBUF)?;

    let mut target_sr = SpatialRef::from_epsg(4326).unwrap();
    target_sr.set_axis_mapping_strategy(OSRAxisMappingStrategy::OAMS_TRADITIONAL_GIS_ORDER);

    let ds_out = fgb_driver.create(args.out_fgb.to_str().unwrap() ).unwrap();
    let lyr_out = ds_out.create_layer_ext::<String>(
            &args.out_fgb.file_stem().unwrap().to_str().unwrap(),
            &target_sr,
            OGRwkbGeometryType::wkbPolygon,
            &[
                //"SPATIAL_INDEX=NO".to_string()
            ]
        ).unwrap();
    let lyr_out_def = lyr_out.layer_definition();

    let snap_raster = Raster::read(&args.snap_raster, true);
    let stats = &snap_raster.stats;

    let now  = Instant::now();
    let total_to_process = records.len();
    let mut last_output = Instant::now();

    let template_shapes = create_template_shapes()?;

    debug!("Looping through records");
    for (r_idx, r) in records.into_iter().enumerate() {

        if last_output.elapsed().as_secs() >= 3 {
            last_output = Instant::now();
            print_remaining_time_msg(&now, r_idx as _, total_to_process as u32,
                ""
            );
        }

        // let raster_x = r.raster_x;
        // let raster_y = r.raster_y;

        // let square_x_min = stats.calc_x_coord(raster_x);
        // let square_x_max = stats.calc_x_coord(raster_x+1);
        // let square_y_min = stats.calc_y_coord(raster_y+1);
        // let square_y_max = stats.calc_y_coord(raster_y);
        //
        // assert!(square_y_max > square_y_min);

        let gdal_geom = create_shape(&r, &stats, &template_shapes)?;

        let mut ft = Feature::new(&lyr_out_def)?;
        ft.set_geometry_directly(gdal_geom)?;

        ft.create(&lyr_out)?;
    }

    Ok(())
}

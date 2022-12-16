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
use gdal::vector::{OGREnvelope};
use gdal::spatial_ref::{CoordTransform, SpatialRef, OSRAxisMappingStrategy};
use itertools::Itertools;
use geo::{Point as GeoPoint};
use geo::algorithm::geodesic_distance::GeodesicDistance;
use geo::algorithm::euclidean_distance::EuclideanDistance;
use std::collections::HashSet;
use std::cmp::{min, max};
use std::path::PathBuf;
use geo_util::raster::Raster;
use structopt::StructOpt;

#[derive(StructOpt)]
pub struct TestProjectionArgs {

    #[structopt(long, parse(from_os_str))]
    pub (crate) ref_raster: PathBuf,



}

fn get_extent(args: &TestProjectionArgs) -> OGREnvelope {
    let  r = Raster::read(&args.ref_raster, true);
    let stats = &r.stats;

    OGREnvelope{
        MinX: stats.origin_x,
        MaxX: stats.right_x_coord(),
        MinY: stats.bottom_y_coord(),
        MaxY: stats.origin_y
    }
}


fn get_grid_points(env: &OGREnvelope) -> Vec<GeoPoint<f64>>
{
    //do slighly smaller to make sure we don't have too many slices
    let chunks = 10.0 - 1e-6;
    let mut y = env.MinY;
    let y_step = (env.MaxY - env.MinY) / chunks ;
    let x_step = (env.MaxX - env.MinX) / chunks ;

    assert!(y_step > 0.);
    assert!(x_step > 0.);

    let mut v = Vec::new();
    while y < env.MaxY {
        let mut x = env.MinX;

        while x < env.MaxX {

            v.push( GeoPoint::new(x, y) );

            if v.len() > (1.1 * chunks * chunks) as usize {
                panic!("Problem!! {:?}", v);
            }

            x += x_step;
        }

        y += y_step;
    }

    v
}

fn project_points(points: &Vec<GeoPoint<f64>>, xform: &CoordTransform) -> Vec<GeoPoint<f64>> {
    let mut xs = points.iter().map( |p| p.x()).collect_vec();
    let mut ys = points.iter().map( |p| p.y()).collect_vec();
    let mut zs = vec![0.0; xs.len()];

    xform.transform_coords(&mut xs, &mut ys, &mut zs).unwrap();

    (0..zs.len()).map( |i| GeoPoint::new(xs[i], ys[i])).collect_vec()
}

/// Run an accuracy test on a projection
pub fn test_projection(args: &TestProjectionArgs) -> Result<()> {
    let extent = get_extent(args);


    println!("Extent is {:?}", extent);

    let grid_points = get_grid_points(&extent);

    //println!("Grid points {:?}", grid_points);

    println!("{}", "*".repeat(80));
    println!("Large distance test");
    try_points(&grid_points, &extent)?;

    //Then try the 4 extreme corners, and the center
    let small_width = 0.001;
    let small_height = 0.001;

    let [center_x, center_y] = extent.center();

    let envelopes_to_try = vec![
    ("Top Left", OGREnvelope {
        MinX: extent.MinX,
        MaxX: extent.MinX + small_width,
        MinY: extent.MaxY - small_height,
        MaxY: extent.MaxY
    }),
        ("Bottom Left", OGREnvelope {
        MinX: extent.MinX,
        MaxX: extent.MinX + small_width,
        MinY: extent.MinY ,
        MaxY: extent.MinY + small_height
    }),
        ("Bottom Right", OGREnvelope {
        MinX: extent.MaxX - small_width,
        MaxX: extent.MaxX ,
        MinY: extent.MinY ,
        MaxY: extent.MinY + small_height
    }),
        ("Top Right", OGREnvelope {
        MinX: extent.MaxX - small_width,
        MaxX: extent.MaxX ,
        MinY: extent.MaxY - small_height,
        MaxY: extent.MaxY
    }),
        ("Center", OGREnvelope {
        MinX: center_x - small_width / 2.0,
        MaxX: center_x + small_width / 2.0,
        MinY: center_y - small_height / 2.0,
        MaxY: center_y + small_height / 2.0
    }),

    ];

    for (env_name, env) in envelopes_to_try {
        println!("{}\n{}", "*".repeat(80), env_name);

        let grid_points = get_grid_points(&env);
        try_points(&grid_points, &extent)?;
    }



    Ok(())
}

fn find_utm(pt: &GeoPoint<f64>) -> (u8, bool) {
    let utm_zone = ((pt.x() + 180.0) / 6.0).ceil() as i32;

    assert!(utm_zone > 0);

    (utm_zone as u8, pt.y() > 0.)

}

fn get_srs_to_try(extent: &OGREnvelope) -> Result< Vec<(String, SpatialRef)> > {

    let [center_x, center_y] = extent.center();

    //calculate UTM zones, test all corners, take only unique zones
    let bottom_left_utm = find_utm(&GeoPoint::new(extent.MinX, extent.MinY));
    let top_right_utm = find_utm(&GeoPoint::new(extent.MaxX, extent.MaxY));

    let min_utm = min(bottom_left_utm.0, top_right_utm.0);
    let max_utm = max(bottom_left_utm.0, top_right_utm.0);

    let north_south: HashSet<bool> = [bottom_left_utm.1, top_right_utm.1].iter().cloned().collect();

    let mut utms: Vec<(u8, bool)> = Vec::new();
    for utm in min_utm..=max_utm {
        for ns in north_south.iter() {
            utms.push( (utm, *ns));
        }
    }

    let mut srs = utms.iter().map( | (utm_zone, is_north) | {
        let mut utm = SpatialRef::from_proj4(
            &format!("+proj=utm +zone={} +{} +ellps=WGS84 +datum=WGS84 +units=m +no_defs",
            utm_zone, if *is_north {"north"} else {"south"}
            )).unwrap();
        utm.auto_identify_epsg().unwrap();
        let code = utm.auth_code().unwrap();

        (format!("UTM {} {} - EPSG {}", utm_zone, if *is_north {"North"} else {"South"}, code), utm)

    }).collect_vec();

    let mercat = SpatialRef::from_epsg(3857)?;

    let transverse_mercador =
    SpatialRef::from_proj4(&format!("+proj=tmerc +lat_0={} +lon_0={} +k=1 +x_0=0 +y_0=0 +ellps=WGS84 +towgs84=0,0,0,0,0,0,0 +units=m +no_defs",
        center_y, center_x
    ))?;

    let laea = SpatialRef::from_proj4(&format!(
        "+proj=laea +lat_0={} +lon_0={} +ellps=GRS80 +towgs84=0,0,0,0,0,0,0 +units=m +no_defs",
        center_y, center_x
    ))?;

    let mut universal = vec![

        ("Transverse Mercator".to_string(), transverse_mercador),
        ("LAEA Lambert Azimuthal Equal Area".to_string(), laea),
        ("3857 Web Mercator".to_string(), mercat),


    ];

    srs.append(&mut universal);

    Ok(srs)
}

fn try_points(grid_points: &Vec<GeoPoint<f64>>, extent: &OGREnvelope) -> Result<()>
{
    let srs_to_try  = get_srs_to_try(extent)?;

    let mut lat_lon = SpatialRef::from_epsg(4326)?;
    lat_lon.set_axis_mapping_strategy(OSRAxisMappingStrategy::OAMS_TRADITIONAL_GIS_ORDER);


    let xforms = srs_to_try.iter().map( |(_, sr) | {
        CoordTransform::new(&lat_lon, sr).unwrap()
    }).collect_vec();

    let projected_points = xforms.iter().map( |x| {
        project_points(&grid_points, x)
    }).collect_vec();

    let print = false;

    let mut num_points = 0;
    let mut total_actual_distance = 0.0;
    let mut total_distance_diff = vec![0.0; srs_to_try.len()];

    //First test large distances
    for i in 0..grid_points.len() {
        for j in i+1..grid_points.len() {
            let p1 = grid_points[i];
            let p2 = grid_points[j];

            //This is extremely accurate
            let distance = p1.geodesic_distance(&p2);

            num_points += 1;
            total_actual_distance += distance;

            for (idx, (name, _)) in srs_to_try.iter().enumerate() {
                let proj_p1 = projected_points[idx][i];
                let proj_p2 = projected_points[idx][j];

                let proj_distance = proj_p1.euclidean_distance(&proj_p2);

                total_distance_diff[idx] += (proj_distance - distance).abs();

                if print {
                    println!("\nProjection: {} For Lon {:.p$} Lat {:.p$} and Lon {:.p$} Lat {:.p$}\nActual distance: {:.2} Projected Distance: {:.2}\nDifference: {:.2}",
                             name,
                             p1.x(), p1.y(),
                             p2.x(), p2.y(),
                             distance,
                             proj_distance,
                             (distance-proj_distance).abs(),
                             p=5);
                }
            }
        }
    }

    println!("Number of points: {} Average Distance: {:.2}",
        num_points,
        total_actual_distance / num_points as f64
    );
    for (idx, (name, _)) in srs_to_try.iter().enumerate() {
        println!("For {}, Average distance difference is {:.2}",
            name, total_distance_diff[idx] / num_points as f64
        );
    }



    Ok(())
}

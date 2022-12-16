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
use structopt::StructOpt;
use crate::util::{print_remaining_time, RasterChunkIterator};
use crate::raster::{Raster, RasterStats, create_empty_raster, };
use std::fs::remove_file;
use anyhow::{bail, Result};
use gdal::spatial_ref::{CoordTransform, SpatialRef};
use std::time::Instant;
use geo::{Polygon, polygon};
use std::fmt;

#[derive(StructOpt, Default)]
pub struct RasterResampleCli {

    /// The path to the file to read
    #[structopt(parse(from_os_str), long="input")]
    pub input_tif: std::path::PathBuf,

    /// The path to the shapefile to write
    #[structopt(parse(from_os_str), long="output")]
    pub output_tif: std::path::PathBuf,

    #[structopt(long="clean")]
    pub clean_output: bool,

    #[structopt(parse(from_os_str), long="snap")]
    pub snap_raster: Option<std::path::PathBuf>,

    //to override any of the snap raster values
    #[structopt(help = "New Y origin, blank to keep the same.  In output_tif coordinates", short="y", long)]
    pub origin_y: Option<f64>,

    #[structopt(help = "New X origin, blank to keep the same", short="x", long)]
    pub origin_x: Option<f64>,

    #[structopt(help = "Num Cols", short="c", long)]
    pub num_cols: Option<u32>,

    #[structopt(help = "Num Rows", short="r", long)]
    pub num_rows: Option<u32>,

    #[structopt(help = "Pixel height", short="h", long)]
    pub pixel_height: Option<f64>,

    #[structopt(help = "Pixel width", short="w", long)]
    pub pixel_width: Option<f64>,

    #[structopt(help = "Projection EPSG", short="p", long)]
    pub projection: Option<u32>,

    #[structopt(help = "Projection Proj", long)]
    pub projection_proj4: Option<String>,

    #[structopt(help = "NoData value", short="n", long)]
    pub no_data_value: Option<f64>,
}

fn get_output_stats(args: &RasterResampleCli) -> Result<RasterStats> {
    //Either use the provided snap raster or default to the input tif values
    let snap_raster = Raster::read(&args.snap_raster.as_ref().unwrap_or(&args.input_tif), true);

    let mut stats = snap_raster.stats.clone();

    if let Some(o_x) = args.origin_x {
        stats.origin_x = o_x;
    }
    if let Some(o_y) = args.origin_y {
        stats.origin_y = o_y;
    }
    if let Some(nc) = args.num_cols {
        stats.num_cols = nc;
    }
    if let Some(nr) = args.num_rows {
        stats.num_rows = nr;
    }
    if let Some(ph) = args.pixel_height {
        stats.pixel_height = ph;
    }
    if let Some(pw) = args.pixel_width {
        stats.pixel_width = pw;
    }
    if let Some(proj) = args.projection {
        let srs = SpatialRef::from_epsg(proj)?;
        stats.projection = srs.to_wkt()?;
    }
    if let Some(proj) = args.projection_proj4.as_ref() {
        let srs = SpatialRef::from_proj4(&proj)?;
        stats.projection = srs.to_wkt()?;
    }
    if let Some(nd) = args.no_data_value {
        stats.no_data_value = nd;
    }

    Ok(stats)
}

struct Extent<T>
{
    min_x: T,
    max_x: T,
    min_y: T,
    max_y: T
}

impl <T> Extent<T>
where T : PartialOrd + fmt::Display
{
    fn check(&self) -> Result<()> {
        if self.min_x > self.max_x {
            bail!("min x {} > max x {}", self.min_x, self.max_x);
        }
        if self.min_y > self.max_y {
            bail!("min y {} > max y {}", self.min_y, self.max_y);
        }
        Ok(())
    }

}

impl Extent<i32> {
    fn window_size(&self) -> (i32, i32) {
        (1+self.max_x - self.min_x, 1+self.max_y-self.min_y)
    }
    fn window_offset(&self) -> (i32, i32) {
        (self.min_x, self.min_y)
    }
}

impl <T> fmt::Display for Extent<T>
where T: fmt::Display
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "x: ({}, {}) y: ({}, {})", self.min_x, self.max_x, self.min_y, self.max_y)
    }
}

pub fn run_raster_resample(args: &RasterResampleCli) -> Result<()> {



    println!("Starting");

    let now = Instant::now();
    let mut last_output = Instant::now();

    let input_raster = Raster::read(&args.input_tif, true);

    println!("Using input raster: {:?} with stats {}", input_raster.path, input_raster.stats);

    if args.output_tif.is_file() {
        if args.clean_output {
            remove_file(&args.output_tif)?;
        } else {
            panic!("{:?} exists already and --clean is not specified", args.output_tif);
        }
    }

    assert!(!args.output_tif.is_file());

    let output_stats = get_output_stats(&args)?;

    println!("Input stats: {}", &input_raster.stats);

    println!("Going to resize/resample to {}",
        &output_stats
    );

    assert!(output_stats.pixel_height < 0.0);

    create_empty_raster(&args.output_tif, &output_stats, false)?;

    assert!(args.output_tif.is_file());

    let output_raster = Raster::read(&args.output_tif, false);

    let input_raster_band = input_raster.band();
    let output_raster_band = output_raster.band();

    //Need to project from output_to_input
    let input_sr = input_raster.dataset.spatial_reference()?;
    let output_sr = output_raster.dataset.spatial_reference()?;

    let xform_out_in = CoordTransform::new(&output_sr, &input_sr)?;

    let input_square_area = (input_raster.stats.pixel_height * input_raster.stats.pixel_width).abs();

    let input_raster_proj_extent: Extent<f64> =
        Extent {
            min_x: input_raster.stats.origin_x,
            max_x: input_raster.stats.origin_x + input_raster.stats.num_cols as f64 * input_raster.stats.pixel_width,
            max_y: input_raster.stats.origin_y,
            min_y: input_raster.stats.origin_y + input_raster.stats.num_rows as f64 * input_raster.stats.pixel_height
        };
    input_raster_proj_extent.check()?;

    for raster_window in RasterChunkIterator::<i32>::new(
        output_raster.stats.num_rows as _,
        output_raster.stats.num_cols as _, 10)
    {
        let output_window_extent: Extent<i32> = Extent {
            min_x: raster_window.x_range_inclusive.0,
            max_x: raster_window.x_range_inclusive.1,
            min_y: raster_window.y_range_inclusive.0,
            max_y: raster_window.y_range_inclusive.1
        };
        output_window_extent.check()?;

        //First fetch all the input data we might need, add 1 just to be sure
        let input_window_extent = {

            let (input_proj_extent, _input_proj_poly) = get_projected_coordinates(
                &output_window_extent,
                &output_stats,
                Some(&xform_out_in)
            )?;

            //println!("Input projected extent of output pixels: {} of {}", input_proj_extent, output_window_extent);

            let mut input_proj_extent= input_proj_extent;

            //coords_to_raster does bounds checks
            //To workaround the the output window, in input coordinates, may be curved, we need to add some margin
            input_proj_extent.min_y -= input_raster.stats.pixel_height.abs();
            input_proj_extent.max_y += input_raster.stats.pixel_height.abs();
            input_proj_extent.min_x -= input_raster.stats.pixel_width.abs();
            input_proj_extent.max_x += input_raster.stats.pixel_width.abs();

            //range check with overall input raster coords with block coords
            if !extent_ranges_overlap(&input_proj_extent, &input_raster_proj_extent) {
                continue;
            }

            //println!("Output coords in input projected coords x {}", input_proj_extent);


            coords_to_raster(
                &input_raster.stats,
                &input_proj_extent
            )?
        };

        /*
        println!("Computing output raster squares x {} ", output_pixel_extent);
        println!("for input squares {} ", input_window_extent);

         */

        let input_window_data =
                    input_raster_band.read_as_array::<f64>(
                        input_window_extent.window_offset(),
                        input_window_extent.window_size())?;

        let (output_width, output_height) = output_window_extent.window_size();

        assert!(output_width > 0);
        assert!(output_height > 0);

        let mut output_data_vec = vec![output_stats.no_data_value; (output_width * output_height) as usize];

        for output_pixel_x in output_window_extent.min_x..=output_window_extent.max_x {
            for output_pixel_y in output_window_extent.min_y..=output_window_extent.max_y {

                let ope = Extent {
                    min_x: output_pixel_x,
                    max_x: output_pixel_x,
                    min_y: output_pixel_y,
                    max_y: output_pixel_y
                };

                //These are the coordinates of the output pixel in the input raster projection
                let (input_proj_extent_of_output_pixel, _input_poly) = get_projected_coordinates(
                    &ope,
                    &output_stats,
                    Some(&xform_out_in)
                )?;

                if !extent_ranges_overlap(&input_proj_extent_of_output_pixel, &input_raster_proj_extent) {
                    continue;
                }

                let input_pixel_extent = coords_to_raster(
                    &input_raster.stats,
                    &input_proj_extent_of_output_pixel
                )?;


                if input_pixel_extent.max_y > input_window_extent.max_y {
                    println!("For output pixel x {} y {}, input proj. coords {} input pixel {} input window extent {}",
                         output_pixel_x, output_pixel_y,
                         input_proj_extent_of_output_pixel,
                             input_pixel_extent,
                             input_window_extent,
                    );
                }

                let mut at_least_one = false;
                //loop through each input square that intersects with the output square
                //get the coordinates of the square
                //compare to coordinates of the output square
                let mut calc_output_value = 0.0;

                input_pixel_extent.check()?;

                assert!(input_pixel_extent.min_y >= input_window_extent.min_y);
                assert!(input_pixel_extent.min_y <= input_window_extent.max_y);
                assert!(input_pixel_extent.max_y >= input_window_extent.min_y);
                assert!(input_pixel_extent.max_y <= input_window_extent.max_y);

                for i_x in input_pixel_extent.min_x..=input_pixel_extent.max_x {
                    for i_y in input_pixel_extent.min_y..=input_pixel_extent.max_y {

                        //get the input value
                        let input_raster_value = input_window_data[ ((i_y - input_window_extent.min_y) as usize,
                                                                     (i_x - input_window_extent.min_x) as usize )
                        ];

                        //Ignore No Data
                        if input_raster.stats.is_nodata(input_raster_value) {
                            continue;
                        }

                        let ipe = Extent {
                            min_x: i_x,
                            max_x: i_x,
                            min_y: i_y,
                            max_y: i_y
                        };

                        //Get the input square coordinates in the input raster projection
                        let (input_extent, _) = get_projected_coordinates(
                            &ipe,
                            &input_raster.stats,
                            None
                        )?;

                        //Compute %, note more exact would be to intersect the actual polygons, which may not be rectangles because of reprojection
                        let x_overlap = 0f64.max(input_extent.max_x.min( input_proj_extent_of_output_pixel.max_x) -
                            input_extent.min_x.max(input_proj_extent_of_output_pixel.min_x));

                        //Since y row is higher, the highest coord is 0
                        let y_overlap = 0f64.max(input_extent.max_y.min( input_proj_extent_of_output_pixel.max_y) -
                            input_extent.min_y.max(input_proj_extent_of_output_pixel.min_y));

                        let overlap_area = x_overlap * y_overlap;
                        let ratio = overlap_area / input_square_area;

                        //We don't want rounding errors at the edges to include too much
                        if ratio < 1e-6 {
                            continue;
                        }

                        assert!(ratio <= 1.0 + 1e-10);

                        calc_output_value += ratio * input_raster_value;
                        at_least_one = true;
                    }
                }

                if at_least_one {
                    let output_window_row = output_pixel_y - output_window_extent.min_y;
                    let output_window_col = output_pixel_x - output_window_extent.min_x;

                    let output_window_idx = output_window_row * output_width + output_window_col;

                    output_data_vec[output_window_idx as usize] = calc_output_value;
                }

                if last_output.elapsed().as_secs() >= 3 {
                    last_output = Instant::now();

                    print_remaining_time(&now, raster_window.current_step as _, raster_window.num_steps as _);
                }
            }
        }

        //done with a block
        output_raster_band.write(
            output_window_extent.window_offset(),
            output_window_extent.window_size(),
             &output_data_vec)?;


    }

    Ok(())

}

/// Gets coordinates of a raster squares and transform them
fn get_projected_coordinates(
    //inclusive range of the raster x's & y's
    raster_point_extent: &Extent<i32>,

    //stats associated with the coordinates above
    stats: &RasterStats,
    xform: Option<&CoordTransform>,
) -> Result< (Extent<f64>, Polygon<f64>) > {
        //Get the 4 rectangular projected coordinates in the output raster projection
    let output_x_coords = [
        stats.calc_x_coord(raster_point_extent.min_x),
        stats.calc_x_coord(1 + raster_point_extent.max_x)
    ];
    let output_y_coords = [
        //smallest y will be the greatest raster y
        stats.calc_y_coord(1 + raster_point_extent.max_y),
        stats.calc_y_coord(raster_point_extent.min_y),
    ];

    //println!("for {} x={:?} y={:?}", raster_point_extent, output_x_coords, output_y_coords);

    assert!(output_x_coords[0] < output_x_coords[1]);
    assert!(output_y_coords[0] < output_y_coords[1]);

    let mut x_coords = [
        output_x_coords[0],
        output_x_coords[0],
        output_x_coords[1],
        output_x_coords[1],
    ];
    let mut y_coords = [
        output_y_coords[0],
        output_y_coords[1],
        output_y_coords[1],
        output_y_coords[0],
    ];
    let mut z_coords = [0.,0.,0.,0.,];

    if let Some(xform) = xform {
        xform.transform_coords(&mut x_coords, &mut y_coords, &mut z_coords)?;
        //println!("After xform y {:?}", y_coords);
    }

    let min_x = x_coords[0].min(x_coords[1]);
    let max_x = x_coords[2].max( x_coords[3] );
    let min_y = y_coords[0].min(y_coords[3]);
    let max_y = y_coords[1].max( y_coords[2] );

    let poly = polygon![
        (x: min_x, y: min_y),
        (x: min_x, y: max_y),
        (x: max_x, y: max_y),
        (x: max_x, y: min_y),
    ];

    let e = Extent { min_x, max_x, min_y, max_y };
    e.check()?;

    Ok( (e, poly) )
}

fn coords_to_raster(
    stats: &RasterStats,
    coords: &Extent<f64>
) -> Result<Extent<i32>>
{
    coords.check()?;

    //Now we need the input squares this covers, with bounds check.

    let pixel = Extent {
        min_x: stats.bounds_x(stats.calc_x(coords.min_x)),
        max_x: stats.bounds_x(stats.calc_x(coords.max_x)),
        min_y: stats.bounds_y(stats.calc_y(coords.max_y)),
        max_y: stats.bounds_y(stats.calc_y(coords.min_y)),
    };

    pixel.check()?;

    Ok( pixel )
}

fn ranges_overlap(x1: f64, x2: f64, y1: f64, y2: f64) -> bool {
    assert!(x1 <= x2);
    assert!(y1 <= y2);
    // https://stackoverflow.com/questions/3269434/whats-the-most-efficient-way-to-test-two-integer-ranges-for-overlap
    x1 <= y2 && y1 <= x2
}

fn extent_ranges_overlap( e1: &Extent<f64>, e2: &Extent<f64>) -> bool {
    return ranges_overlap(
        e1.min_x, e1.max_x, e2.min_x, e2.max_x
    ) && ranges_overlap(
        e1.min_y, e1.max_y, e2.min_y, e2.max_y
    )
}

/// cd /rust
/// cargo test --bin raster_resample
#[cfg(test)]
mod raster_sample_test {
    //use geo_util::raster::{RasterStats, create_empty_raster};
    use gdal::spatial_ref::SpatialRef;
    use gdal::raster::types::GdalType;
    use crate::raster::{assert_float_within_eps, MEDIUM_EPSILON};
    use std::path::{PathBuf, Path};
    use geos::{SimpleGeometry, SimpleContextHandle, SimpleCoordinateSequence, };
    use super::*;
    use float_cmp::{F64Margin, ApproxEq, F32Margin};
    use itertools::Itertools;
    

    fn get_temp_filename(file_name: &str) -> PathBuf {
        [ "/country_specific/temp", file_name ].iter().collect()
    }

    ///
    /// Checks the output raster has the same values by area of the input_raster, taking into account
    /// no data
    fn check_rasters(input_raster: &Path, output_raster: &Path) -> Result<(RasterStats,RasterStats)>
    {
        let in_raster = Raster::read(input_raster, true);
        let out_raster = Raster::read(output_raster, true);

        let input_sr = in_raster.dataset.spatial_reference()?;
        let ch = SimpleContextHandle::new();

        let in_polys = vectorize_raster(&in_raster, &input_sr, &ch)?;
        let out_polys = vectorize_raster(&out_raster, &input_sr, &ch)?;

        let in_raster_sq_area = (in_raster.stats.pixel_width * in_raster.stats.pixel_height).abs();

        for (out_index, (out_polygon, out_value)) in out_polys.iter().enumerate() {

            let mut at_least_one_intersection = false;
            let mut calc_out_value = 0.0;

            for (in_polygon, in_value) in in_polys.iter() {

                if in_raster.stats.is_nodata(*in_value) {
                    continue;
                }

                let intersection = in_polygon.intersection(&ch, &out_polygon)?;

                let area = intersection.area()?;

                if area == 0. {
                    continue;
                }

                assert!(area > 0.0);

                calc_out_value += area / in_raster_sq_area * in_value;
                at_least_one_intersection = true;
                //println!("Intersection is {} with area {}", intersection.to_wkt_precision(5)?, area);
            }

            if !at_least_one_intersection {
                calc_out_value = out_raster.stats.no_data_value;
            }

            if !(calc_out_value.approx_eq(*out_value, F64Margin{ ulps: 5, epsilon: 1e-6})) {

                //2nd check with casting to f32
                if out_raster.stats.gdal_type != f32::gdal_type() ||
                    !( (calc_out_value as f32).approx_eq(*out_value as f32, F32Margin{ ulps: 5, epsilon: 1e-6})) {
                    println!("For output polygon {}, row {}, col {}.  Value was\n{}\nbut should be\n{}\n",
                             out_index,
                             out_index / out_raster.stats.num_cols as usize,
                             out_index % out_raster.stats.num_cols as usize,
                             out_value,
                             calc_out_value
                    );
                    assert!(false);
                }
            }
        }

        Ok( (in_raster.stats, out_raster.stats) )
    }

    fn check_output_stats(in_stats: &RasterStats, out_stats:  &RasterStats, args: &RasterResampleCli)
    {
        assert_float_within_eps( args.origin_x.unwrap_or(in_stats.origin_x), out_stats.origin_x, MEDIUM_EPSILON, "origin_x");
        assert_float_within_eps( args.origin_y.unwrap_or(in_stats.origin_y), out_stats.origin_y, MEDIUM_EPSILON, "origin_y");

        assert_float_within_eps( args.pixel_height.unwrap_or(in_stats.pixel_height), out_stats.pixel_height, MEDIUM_EPSILON, "pixel_height");
        assert_float_within_eps( args.pixel_width.unwrap_or(in_stats.pixel_width), out_stats.pixel_width, MEDIUM_EPSILON, "pixel_width");

        assert_float_within_eps( args.no_data_value.unwrap_or(in_stats.no_data_value), out_stats.no_data_value, MEDIUM_EPSILON, "no_data_value");

        assert_eq!( args.num_rows.unwrap_or(in_stats.num_rows), out_stats.num_rows);
        assert_eq!( args.num_cols.unwrap_or(in_stats.num_cols), out_stats.num_cols);

    }

    /// Returns a list of rectangles with coordinates of the given raster squares
    fn vectorize_raster<'a>(raster: &Raster, target_srs: &'a SpatialRef, ch: &'a SimpleContextHandle) -> Result<Vec< (SimpleGeometry<'a>, f64) >> {
        let mut polygons_and_values = Vec::new();

        let xform = CoordTransform::new(&raster.dataset.spatial_reference()?, target_srs)?;

        let input_band = raster.dataset.rasterband(1)?;

        let data = input_band.read_as::<f64, i32>( (0,0), (raster.stats.num_cols as i32, raster.stats.num_rows as i32))?;

        let stats = &raster.stats;

        let mut idx = 0;
        for pixel_y in 0..raster.stats.num_rows as i32 {
            for pixel_x in 0..raster.stats.num_cols as i32 {

                let pe = Extent {
                            min_x: pixel_x,
                            max_x: pixel_x,
                            min_y: pixel_y,
                            max_y: pixel_y
                        };

                //Get the 4 rectangular projected coordinates
                //in the input projection
                let (trans_extent, _) =
                    get_projected_coordinates( &pe, stats, Some(&xform))?;

                let (trans_x_coords, trans_y_coords) = (
                    [trans_extent.min_x, trans_extent.max_x],
                    [trans_extent.min_y, trans_extent.max_y]
                    );


                let mut coord_seq = SimpleCoordinateSequence::new(5, &ch)?;
                coord_seq.set_x(0, trans_x_coords[0])?;
                coord_seq.set_y(0, trans_y_coords[0])?;

                coord_seq.set_x(1, trans_x_coords[1])?;
                coord_seq.set_y(1, trans_y_coords[0])?;

                coord_seq.set_x(2, trans_x_coords[1])?;
                coord_seq.set_y(2, trans_y_coords[1])?;

                coord_seq.set_x(3, trans_x_coords[0])?;
                coord_seq.set_y(3, trans_y_coords[1])?;

                coord_seq.set_x(4, trans_x_coords[0])?;
                coord_seq.set_y(4, trans_y_coords[0])?;

                let outer_ring = SimpleGeometry::create_linear_ring(coord_seq)?;
                let polygon = SimpleGeometry::create_polygon(outer_ring, Vec::new())?;

                polygons_and_values.push((polygon, data[idx] ) );
                idx += 1;
            }
        }

        Ok(polygons_and_values)
    }

    fn run_test<T>(
        input_raster_stats: &RasterStats, cli_args_list: &[RasterResampleCli], input_raster_data: Vec<T>,
        in_file_name: &str
    ) -> Result<()>
    where T: GdalType + Copy
    {

        let input_path = get_temp_filename(in_file_name);

        if input_path.exists() {
            remove_file(&input_path)?;
        }

        assert!(!input_path.exists());

        create_empty_raster(&input_path, input_raster_stats, false)?;

        assert!(input_path.exists());

        {
            let input_raster = Raster::read(&input_path, false);

            let input_raster_band = input_raster.dataset.rasterband(1)?;

            let num_rows = input_raster_stats.num_rows;
            let num_cols = input_raster_stats.num_cols;

            input_raster_band.write((0, 0), (num_cols as i32, num_rows as i32),
                                    &input_raster_data)?;
        }

        for cli_args in cli_args_list.iter() {

            if cli_args.output_tif.exists() {
                remove_file(&cli_args.output_tif)?;
            }

            assert!(!cli_args.output_tif.exists());
            assert_eq!(cli_args.input_tif, input_path);

            run_raster_resample(&cli_args)?;

            assert!(cli_args.output_tif.exists());

            let (in_stats, out_stats) = check_rasters(&input_path, &cli_args.output_tif)?;
            check_output_stats(&in_stats, &out_stats, &cli_args);
        }
        Ok(())
    }

    #[test]
    fn test_shifting() -> Result<()> {
        let srs = SpatialRef::from_epsg(4326)?;

        let origin_y = 46.242485;
        let origin_x = 6.021557;

        //Create a 3 x 3 raster
        let stats = RasterStats {
            origin_y,
            origin_x,
            pixel_height: -0.005,
            pixel_width: 0.004,
            num_rows: 3,
            num_cols: 3,
            no_data_value: -1000.5,
            gdal_type: f32::gdal_type(),
            projection: srs.to_wkt()?
        };

        let input_path = get_temp_filename("3x3.tif");

        run_test(
            &stats,
            &[
                RasterResampleCli {
            input_tif: input_path.clone(),
            output_tif: get_temp_filename("3x3_shifted_up.tif"),
            origin_y: Some(origin_y + 0.0020),
            ..Default::default()
        },
                RasterResampleCli {
            input_tif: input_path.clone(),
            output_tif: get_temp_filename("3x3_shifted_down.tif"),
            origin_y: Some(origin_y - 0.0020),
            ..Default::default()
        },
                RasterResampleCli {
            input_tif: input_path.clone(),
            output_tif: get_temp_filename("3x3_shifted_left.tif"),
            clean_output: true,
            origin_x: Some(origin_x - 0.0030),
            ..Default::default()
        },
                RasterResampleCli {
            input_tif: input_path.clone(),
            output_tif: get_temp_filename("3x3_shifted_right.tif"),
            clean_output: true,
            origin_x: Some(origin_x + 0.0030),
            ..Default::default()
        }
            ],
            vec![
                1f32, 2f32, 3f32, 4f32, 5f32, 6f32, 7f32, 8f32, 9f32
            ],
            input_path.file_name().unwrap().to_str().unwrap()
        )?;


        Ok(())
    }

    #[test]
    fn test_small_to_big() -> Result<()> {
        let srs = SpatialRef::from_epsg(4326)?;

        let origin_y = 46.242485;
        let origin_x = 6.021557;
        let no_data = -1e10;

        let stats = RasterStats {
            origin_y,
            origin_x,
            pixel_height: -0.005,
            pixel_width: 0.004,
            num_rows: 30,
            num_cols: 35,
            no_data_value: no_data,
            gdal_type: f64::gdal_type(),
            projection: srs.to_wkt()?
        };

        //Add some no data to check that
        let input_raster_data = (0..stats.num_rows *stats.num_cols).map( |v|
            if v % 8 == 0 {
                no_data
            } else {
                (1 + v) as f64
            }).collect_vec();



        let input_path = get_temp_filename("30x35.tif");

        run_test(
            &stats,
            &[
                RasterResampleCli {
            input_tif: input_path.clone(),
            output_tif: get_temp_filename("7x5.tif"),
            num_cols: Some(num::Integer::div_ceil(&stats.num_cols, &5)),
            num_rows: Some(num::Integer::div_ceil(&stats.num_rows, &7)),
            pixel_width: Some(stats.pixel_width * 5.),
            pixel_height: Some(stats.pixel_height * 7.),
            ..Default::default()
        },
                 RasterResampleCli {
            input_tif: input_path.clone(),
            output_tif: get_temp_filename("7x5_shifted_small.tif"),
            num_cols: Some(num::Integer::div_ceil(&stats.num_cols, &5)),
            num_rows: Some(num::Integer::div_ceil(&stats.num_rows, &7)),
            pixel_width: Some(stats.pixel_width * 5.),
            pixel_height: Some(stats.pixel_height * 7.),
            origin_x: Some(stats.origin_x + 0.0001),
            origin_y: Some(stats.origin_y + 0.0001),
            ..Default::default()
        },
                RasterResampleCli {
            input_tif: input_path.clone(),
            output_tif: get_temp_filename("7x5_shifted_big.tif"),
            num_cols: Some(num::Integer::div_ceil(&stats.num_cols, &5)),
            num_rows: Some(num::Integer::div_ceil(&stats.num_rows, &7)),
            pixel_width: Some(stats.pixel_width * 5.),
            pixel_height: Some(stats.pixel_height * 7.),
            origin_x: Some(stats.origin_x - 0.11),
            origin_y: Some(stats.origin_y + 0.12),
            ..Default::default()
        },
                RasterResampleCli {
            input_tif: input_path.clone(),
            output_tif: get_temp_filename("7x5_shifted_3857_small.tif"),
            num_cols: Some(num::Integer::div_ceil(&stats.num_cols, &5)),
            num_rows: Some(num::Integer::div_ceil(&stats.num_rows, &7)),
            origin_x: Some(670700.0 ),
            origin_y: Some( 5818726.0),
            projection: Some(3857),
            pixel_width: Some(315.),
            pixel_height: Some(-545.),
            ..Default::default()
        },
                RasterResampleCli {
            input_tif: input_path.clone(),
            output_tif: get_temp_filename("7x5_shifted_3857.tif"),
            num_cols: Some(num::Integer::div_ceil(&stats.num_cols, &5)),
            num_rows: Some(num::Integer::div_ceil(&stats.num_rows, &7)),
            origin_x: Some(670700.0 ),
            origin_y: Some( 5818726.0),
            projection: Some(3857),
            pixel_width: Some(315. * 11.),
            pixel_height: Some(-545. * 10.),
            ..Default::default()
        },
RasterResampleCli {
            input_tif: input_path.clone(),
            output_tif: get_temp_filename("zoom_out_many_rows.tif"),
            num_cols: Some(num::Integer::div_ceil(&stats.num_cols, &5)),
            num_rows: Some(stats.num_rows * 2),
            pixel_width: Some(stats.pixel_width * 5.),
            pixel_height: Some(stats.pixel_height / 1.9),
            ..Default::default()
        },
            ],
            input_raster_data,
            input_path.file_name().unwrap().to_str().unwrap()
        )?;


        Ok(())
    }

    #[test]
    fn test_big_to_small() -> Result<()> {
        let srs = SpatialRef::from_epsg(4326)?;

        let origin_y = 46.242485;
        let origin_x = 6.021557;
        let no_data = 1e10;

        let stats = RasterStats {
            origin_y,
            origin_x,
            pixel_height: -0.005,
            pixel_width: 0.004,
            num_rows: 5,
            num_cols: 4,
            no_data_value: no_data,
            gdal_type: f64::gdal_type(),
            projection: srs.to_wkt()?
        };

        //Add some no data to check that
        let input_raster_data = (0..stats.num_rows *stats.num_cols).map( |v|
            if v % 8 == 0 {
                no_data
            } else {
                (1 + v) as f64
            }).collect_vec();


        let input_path = get_temp_filename("4x5.tif");

        run_test(
            &stats,
            &[
                RasterResampleCli {
            input_tif: input_path.clone(),
            output_tif: get_temp_filename("28x25.tif"),
            num_cols: Some(stats.num_cols * 5),
            num_rows: Some(stats.num_rows * 7),
            pixel_width: Some(stats.pixel_width / 5.),
            pixel_height: Some(stats.pixel_height / 7.),
            ..Default::default()
        },
                 RasterResampleCli {
            input_tif: input_path.clone(),
            output_tif: get_temp_filename("28x25_shifted_small.tif"),
            num_cols: Some(stats.num_cols * 5),
            num_rows: Some(stats.num_rows * 7),
            pixel_width: Some(stats.pixel_width / 5.),
            pixel_height: Some(stats.pixel_height / 7.),
            origin_x: Some(stats.origin_x + 0.001),
            origin_y: Some(stats.origin_y + 0.001),
            ..Default::default()
        },
                RasterResampleCli {
            input_tif: input_path.clone(),
            output_tif: get_temp_filename("28x25_shifted_big.tif"),
            num_cols: Some(stats.num_cols * 5),
            num_rows: Some(stats.num_rows * 7),
            pixel_width: Some(stats.pixel_width / 5.),
            pixel_height: Some(stats.pixel_height / 7.),
            origin_x: Some(stats.origin_x - 0.11),
            origin_y: Some(stats.origin_y + 0.12),
            ..Default::default()
        },
                RasterResampleCli {
            input_tif: input_path.clone(),
            output_tif: get_temp_filename("28x25_shifted_3857_small.tif"),
            num_cols: Some(stats.num_cols * 5),
            num_rows: Some(stats.num_rows * 7),
            origin_x: Some(670700.0 ),
            origin_y: Some( 5818726.0),
            projection: Some(3857),
            pixel_width: Some(315. / 5.),
            pixel_height: Some(-545. / 7.),
            ..Default::default()
        },

            ],
            input_raster_data,
            input_path.file_name().unwrap().to_str().unwrap()
        )?;


        Ok(())
    }

    #[test]
    fn test_nodata() -> Result<()> {
        let srs = SpatialRef::from_epsg(4326)?;

        let origin_y = 12.4179;
        let origin_x = 0.774583;
        let no_data = -3.4e38;

        let stats = RasterStats {
            origin_y,
            origin_x,
            pixel_height: -0.0008333333300539083697,
            pixel_width: 0.000833333330083942613,
            num_rows: 5,
            num_cols: 3,
            //on purpose use a real no data value that is slightly off
            no_data_value: -3.39999999999999996e+38,
            gdal_type: f32::gdal_type(),
            projection: srs.to_wkt()?
        };

        //Add some no data to check that
        let mut input_raster_data = (0..stats.num_rows *stats.num_cols).map( |v|
            if v % 8 == 0 {
                no_data
            } else {
                (1 + v) as f64
            }).collect_vec();

        //make the 3rd column all no data
        for r in 0..stats.num_rows
        {
            input_raster_data[ (r * stats.num_cols + 2) as usize ] = no_data;
        }

        let input_path = get_temp_filename("nodata.tif");

        run_test(
            &stats,
            &[
                RasterResampleCli {
            input_tif: input_path.clone(),
            output_tif: get_temp_filename("nodata_shifted.tif"),
            origin_x: Some(stats.origin_x + stats.pixel_width),
            origin_y: Some(stats.origin_y + stats.pixel_height),
            no_data_value: Some(-99999.0),
            ..Default::default()
        },


            ],
            input_raster_data,
            input_path.file_name().unwrap().to_str().unwrap()
        )?;


        Ok(())
    }

    #[test]
    fn test_window_projections() -> Result<()> {
        let input_srs = SpatialRef::from_epsg(32632)?;
        // let input_stats = RasterStats {
        //     origin_y: 1554651.200170952361077,
        //     origin_x: -223293.464999244431965,
        //     pixel_height: -92.783445792139503,
        //     pixel_width: 92.783445792139517,
        //     num_rows: 11701,
        //     num_cols:  14787,
        //     //on purpose use a real no data value that is slightly off
        //     no_data_value: -3.39999999999999996e+38,
        //     gdal_type: f32::gdal_type(),
        //     projection: input_srs.to_wkt()?
        // };

        let output_srs = SpatialRef::from_epsg(3857)?;
        let output_stats = RasterStats {
            origin_y: 1561769.205499100033194,
            origin_x: 299728.272533965995535,
            pixel_height: -100.,
            pixel_width: 100.,
            num_rows: 10860,
            num_cols:  13343,
            //on purpose use a real no data value that is slightly off
            no_data_value: -3.39999999999999996e+38,
            gdal_type: f32::gdal_type(),
            projection: output_srs.to_wkt()?
        };

        let output_pixel_extent: Extent<i32> = Extent {
            min_x: 6675,
            max_x: 8009,
            min_y: 0,
            max_y: 1085
        };

        let xform_out_in = CoordTransform::new(
            &output_srs, &input_srs)?;


        //To illustrate the problem, where we have basically a curve in the projection

        let y_coord = 1453169.2054991;
        for x in 0..10 {
            let mut x_coords = [
                967228.272533966 + 10000. * x as f64
            ];
            let mut y_coords = [y_coord];
            let mut z_coords = [0.];
            xform_out_in.transform_coords(&mut x_coords, &mut y_coords, &mut z_coords)?;

            println!("Y coord: {}", y_coords[0]);
        }

        let (input_proj_extent, _input_proj_poly) = get_projected_coordinates(
                &output_pixel_extent,
                &output_stats,
                Some(&xform_out_in)
            )?;

        assert_float_within_eps(input_proj_extent.min_y,
        1430802.806174992, MEDIUM_EPSILON, "");
        assert_float_within_eps(input_proj_extent.max_y,
        1535903.0464974316, MEDIUM_EPSILON, "");

        //Now one pixel that is within the window

        for x in [6675, 6680, 6685, 6888, 8009] {
            let ope = Extent {
                min_x: x,
                max_x: x,
                min_y: 1085,
                max_y: 1085
            };

            let (input_proj_extent_of_output_pixel, _input_poly) = get_projected_coordinates(
                &ope,
                &output_stats,
                Some(&xform_out_in)
            )?;

            println!("Extent {} of {}.  Input window {}", input_proj_extent_of_output_pixel, ope, input_proj_extent);

            //This is the bug, because the code assumes that the projected squares are not curved
            //assert!(input_proj_extent_of_output_pixel.min_y >= input_proj_extent.min_y);
        }
        Ok(())
    }

}
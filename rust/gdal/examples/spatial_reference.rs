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
use gdal::spatial_ref::{CoordTransform, SpatialRef};
use gdal::vector::Geometry;
use anyhow::Result;

fn run() -> Result<()> {
    let spatial_ref1 = SpatialRef::from_proj4(
        "+proj=laea +lat_0=52 +lon_0=10 +x_0=4321000 +y_0=3210000 +ellps=GRS80 +units=m +no_defs",
    )?;
    println!(
        "Spatial ref from proj4 to wkt:\n{:?}\n",
        spatial_ref1.to_wkt()?
    );
    let spatial_ref2 = SpatialRef::from_wkt("GEOGCS[\"WGS 84\",DATUM[\"WGS_1984\",SPHEROID[\"WGS 84\",6378137,298.257223563,AUTHORITY[\"EPSG\",7030]],TOWGS84[0,0,0,0,0,0,0],AUTHORITY[\"EPSG\",6326]],PRIMEM[\"Greenwich\",0,AUTHORITY[\"EPSG\",8901]],UNIT[\"DMSH\",0.0174532925199433,AUTHORITY[\"EPSG\",9108]],AXIS[\"Lat\",NORTH],AXIS[\"Long\",EAST],AUTHORITY[\"EPSG\",4326]]")?;
    println!(
        "Spatial ref from wkt to proj4:\n{:?}\n",
        spatial_ref2.to_proj4()?
    );
    let spatial_ref3 = SpatialRef::from_definition("urn:ogc:def:crs:EPSG:6.3:26986")?;
    println!(
        "Spatial ref from ogc naming to wkt:\n{:?}\n",
        spatial_ref3.to_wkt()?
    );
    let spatial_ref4 = SpatialRef::from_epsg(4326)?;
    println!(
        "Spatial ref from epsg code to wkt:\n{:?}\n",
        spatial_ref4.to_wkt()?
    );
    println!(
        "Spatial ref from epsg code to pretty wkt:\n{:?}\n",
        spatial_ref4.to_pretty_wkt()?
    );
    println!(
        "Comparison between identical SRS : {:?}\n",
        spatial_ref2 == spatial_ref4
    );
    let htransform = CoordTransform::new(&spatial_ref2, &spatial_ref1)?;
    let mut xs = [23.43, 23.50];
    let mut ys = [37.58, 37.70];
    println!("Before transformation :\n{:?} {:?}", xs, ys);
    htransform.transform_coords(&mut xs, &mut ys, &mut [0.0, 0.0])?;
    println!("After transformation :\n{:?} {:?}\n", xs, ys);
    let geom = Geometry::from_wkt(
        "POLYGON((23.43 37.58, 23.43 40.0, 25.29 40.0, 25.29 37.58, 23.43 37.58))",
    )?;
    println!("Polygon before transformation:\n{:?}\n", geom.wkt()?);
    geom.transform(&htransform)?;
    println!("Polygon after transformation:\n{:?}\n", geom.wkt()?);
    let spatial_ref5 = SpatialRef::from_epsg(4326)?;
    println!("To wkt: {:?}", spatial_ref5.to_wkt());
    spatial_ref5.morph_to_esri()?;
    println!("To esri wkt: {:?}", spatial_ref5.to_wkt());
    println!("To xml: {:?}", spatial_ref5.to_xml());

    Ok(())
}

fn main() {
    run().unwrap();
}

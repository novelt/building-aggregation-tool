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
fn main() {
    use gdal::metadata::Metadata;
    use gdal::raster::dataset::Dataset;
    use std::path::Path;

    let driver = gdal::raster::driver::Driver::get("mem").unwrap();
    println!("driver description: {:?}", driver.description());

    let path = Path::new("./fixtures/tinymarble.png");
    let dataset = Dataset::open(path, true).unwrap();
    println!("dataset description: {:?}", dataset.description());

    let key = "INTERLEAVE";
    let domain = "IMAGE_STRUCTURE";
    let meta = dataset.metadata_item(key, domain);
    println!("domain: {:?} key: {:?} -> value: {:?}", domain, key, meta);
}

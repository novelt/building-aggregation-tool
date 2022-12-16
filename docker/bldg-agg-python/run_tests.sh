#!/bin/bash

# This file is part of the Building Aggregration Tool
# Copyright (C) 2022 Novel-T
# 
# The Building Aggregration Tool is free software: you can redistribute it and/or modify
# it under the terms of the GNU General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
# 
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU General Public License for more details.
# 
# You should have received a copy of the GNU General Public License
# along with this program.  If not, see <http://www.gnu.org/licenses/>.


set -x
set -e

if [ ! -f /cargo_home/bin/cargo ]; then
  rsync -a --progress --verbose /root/.cargo/ /cargo_home
else
  echo "Not rsyncing, assume /cargo_home has everything we need since bin/cargo exists"
fi

export CARGO_TARGET_DIR=/rust/target
export CARGO_HOME=/cargo_home
export PATH=/cargo_home/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin

mkdir /test_results


declare -a test_dirs=( "geo_util" "geos" "gdal" "cmdline_tools" "zonal_stats" )

for td in "${test_dirs[@]}"
do

cd /rust/${td}
cargo test -- -Z unstable-options --format json | tee results.json
cat results.json | cargo2junit > /test_results/${td}_test.xml

done


# tarpauline instruments everything, includig dependencies.  To speed up builds, we use a different target dir
# this will run all the test it finds.  Ignore target because there are some rust files in there
export CARGO_TARGET_DIR=/rust/target2
cd /rust
# if the tests fail, we don't want the coverage to fail
cargo tarpaulin --verbose --out Xml \
--target-dir /rust/target2 \
--exclude-files /rust/target2 \
--exclude-files target \
--exclude-files proj \
--exclude-files test_projections \
--exclude-files test_deserialize \
--exclude-files grid3_shapefile \
|| true
cp cobertura.xml /test_results/all_cobertura.xml || true
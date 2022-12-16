#!/bin/sh

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

cd  /rust

#--x-min 5.9 \
#--x-max 5.91 \
#--y-min 46.11 \
#--y-max 47 \

#--ogr-conn-str "PG: host=db dbname=pop_census port=5432 user=postgres password=postgres" \

cargo run --bin cmdline_tools --release -- contours \
--input-raster /modules/CENSUS/input/eu_dem/eu_dem_v11_E40N20.TIF \
--x-min 5.9 \
--x-max 14 \
--y-min 42 \
--y-max 48 \
--pg-conn-str "postgresql://postgres:postgres@db:5432/pop_census" \
--polygon-layer-name "poly" \
--line-layer-name "contours"


# CREATE INDEX contours_idx ON data_work.contours USING GIST (shape)
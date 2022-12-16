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

# cargo install binding
cd "$(dirname "$0")"

bindgen ./wrapper.h --whitelist-function 'CPL.*' \
--whitelist-function 'CSL.*' \
--whitelist-function 'GDAL.*' --whitelist-function 'OGR.*' \
--whitelist-function 'OSR.*' --whitelist-function 'OCT.*' \
--whitelist-function 'VSI.*' --whitelist-type 'OGR.*' \
--ctypes-prefix libc --constified-enum-module '.*' > ./src/gdal_3_3.rs
# -- -x c++ -std=c++14
# -I /usr/include/linux -I /usr/lib/clang/6.0.0/include
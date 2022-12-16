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


cd /
#git clone --depth 1 --branch v3.2.0 https://github.com/OSGeo/gdal
curl -s -L -O  "https://github.com/OSGeo/gdal/archive/v3.2.0.tar.gz"
tar xvzf v3.2.0.tar.gz

curl -s -L -O https://github.com/Esri/file-geodatabase-api/raw/master/FileGDB_API_1.5.1/FileGDB_API_1_5_1-64gcc51.tar.gz
tar xvzf FileGDB_API_1_5_1-64gcc51.tar.gz

# Otherwise there are undefined refs since the compilation will link to this one instead of the library one
rm -rf /FileGDB_API-64gcc51/lib/libstdc++*

cd gdal-3.2.0/gdal
#export LD_LIBRARY_PATH=/proj/install/lib:$LD_LIBRARY_PATH
./configure --without-libtool \
            --with-hide-internal-symbols \
            --with-proj=/usr/local \
            --with-fgdb=/FileGDB_API-64gcc51 \
            --with-python=python3.8 \
            --with-libtiff=internal --with-rename-internal-libtiff-symbols \
            --with-geotiff=internal --with-rename-internal-libgeotiff-symbols \
            --with-pg \
            --with-spatialite \
            --with-libkml \
            --with-geos
make -j4
make install

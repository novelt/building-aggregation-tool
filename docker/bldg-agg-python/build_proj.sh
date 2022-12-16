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


git clone --depth 1 --single-branch --branch 6.3.2 https://github.com/OSGeo/proj.4 proj
cd proj
./autogen.sh
CXXFLAGS="-DPROJ_RENAME_SYMBOLS -O2" CFLAGS=$CXXFLAGS ./configure --prefix=/usr/local --disable-static
make -j4
make install
cd /usr/local/lib
# https://trac.osgeo.org/gdal/wiki/BuildingOnUnixGDAL25dev
# Rename the library to libinternalproj
mv libproj.so.15.3.2 libinternalproj.so.15.3.2
ln -s libinternalproj.so.15.3.2 libinternalproj.so.15
ln -s libinternalproj.so.15.3.2 libinternalproj.so
rm -f libproj.*

patchelf --set-soname libinternalproj.so libinternalproj.so
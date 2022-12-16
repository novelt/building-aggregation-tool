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


set -e

cd "$(dirname "$0")"

touch local.env

# rem --build-arg BUILDKIT_INLINE_CACHE=1
docker build --target builder --cache-from=novelt/bldg-agg-python-builder:latest --tag novelt/bldg-agg-python-builder:latest .. -f ./bldg-agg-python/Dockerfile
#if %errorlevel% neq 0 exit /b %errorlevel%

#rem --build-arg BUILDKIT_INLINE_CACHE=1
docker build --target bldg-agg-python --cache-from=novelt/bldg-agg-python-builder:latest --cache-from=novelt/bldg-agg-python:latest  --tag novelt/bldg-agg-python:latest .. -f ./bldg-agg-python/Dockerfile
#rem docker build --target bldg-agg-python .. -f ./bldg-agg-python/Dockerfile
#if %errorlevel% neq 0 exit /b %errorlevel%

docker-compose build
#if %errorlevel% neq 0 exit /b %errorlevel%

./binbash.sh
# -*- coding: utf-8 -*-

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

# -*- coding: utf-8 -*-

import os
from pathlib import Path


class Config(object):
    MODULE_NAME = "BLDG_AGG"
    MODULE_DIR = Path("/modules") / MODULE_NAME
    COUNTRY_CODE = os.environ["COUNTRY_CODE"].upper()

    BASE_DATA_DIR = MODULE_DIR / "input" / COUNTRY_CODE
    WORKING_FOLDER = MODULE_DIR / "working" / COUNTRY_CODE

    BUILDING_INPUT_DIR = BASE_DATA_DIR / "buildings"

    RASTER_INPUT_PATH = BASE_DATA_DIR / "ref_raster"

    SCHEMA_NAME = COUNTRY_CODE.lower()

    SETTLEMENT_GRID_SLICES_SCHEMA_NAME = SCHEMA_NAME + "_sliced_settlements"
    BLDG_SCHEMA_NAME = SCHEMA_NAME + "_buildings"

    # this will be overwritten by the command line --clean
    CLEAN = False

    # how much to buffer buildings, in meters
    BUFFER_SIZE = 50

    MAX_SETTLEMENT_LEVEL = 2

    # raster square density around which to draw contours
    CONTOUR_VALUE = 13

    # How big in area a contour has to be to be considered a BUA
    CONTOUR_MIN_BUA_AREA = 400000

    # When grouping buildings together, how close do they have to be in horizonal or vertical direction to be grouped together
    # This is in the reference raster coordinate system
    # can be over written by --group-distance=<number>
    GROUP_DISTANCE = 0.000833

    RASTER_WORK = WORKING_FOLDER / "rasters"
    RASTER_BUILDING_COUNT = RASTER_WORK / "bldg_count.tif"
    RASTER_EXPANDED_REF = RASTER_WORK / "ref_expanded.tif"

    # when splitting the buildings, how many chunks per row/col
    # the result will split the buildings into CHUNK_ROWS*CHUNK_COLS squares
    CHUNK_ROWS = 10
    CHUNK_COLS = 10

    BUILDINGS_PATH = WORKING_FOLDER / "buildings"

    # where to store the split/reprojected/fixed building geometries
    SPLIT_BUILDING_PATH = BUILDINGS_PATH / "split"

    INTERSECTED_BUILDING_SPLIT_BUILDING_PATH = BUILDINGS_PATH / "bldg_intersected"
    INTERSECTED_BUILDING_SPLIT_BUILDING_WORK_PATH = BUILDINGS_PATH / "bldg_intersected_work"

    # where to store the multipolygons containing buildings within
    GROUPED_BUILDINGS_BASE_PATH = WORKING_FOLDER / "grouped"
    FILLED_BUILDINGS_BASE_PATH = WORKING_FOLDER / "filled"
    BUFFERED_BUILDINGS_BASE_PATH = WORKING_FOLDER / "buffered"

    CENTROIDS_BUILDINGS_PATH = BUILDINGS_PATH / "centroids"

    UNIONED_PATH = WORKING_FOLDER / "union.fgb"
    UNIONED_NO_HOLES_PATH = WORKING_FOLDER / "union_no_holes.fgb"
    FINAL_SETTLEMENT_SHAPES_PATH = WORKING_FOLDER / "settlement_shapes.fgb"

    FINAL_SETTLEMENT_TABLE_NAME = "settlements"

    SETTLEMENTS_PARENT_TABLE = "sliced_settlements"

    BULDING_PARENT_TABLE = "building"

    CONTOUR_POLYGON_TABLE_NAME = "contours_polygons"
    #
    SETTLEMENTS_RASTERIZED_PATH = RASTER_WORK / f"{FINAL_SETTLEMENT_TABLE_NAME}.tif"

    GRID_SLICED_DIR = WORKING_FOLDER / "grid_sliced_settlements" / "sliced"
    GRID_SLICED_ALL = GRID_SLICED_DIR.parent / "all.fgb"

    FINAL_GRID_SLICED_DIR = WORKING_FOLDER / "grid_sliced_settlements" / "sliced_final"
    FINAL_GRID_SLICED_ALL = GRID_SLICED_DIR.parent / "all_final.fgb"

    FINAL_SETTLEMENTS_PATH = WORKING_FOLDER / "settlements.fgb"

    RASTER_CENTER_CSV_DIR = WORKING_FOLDER / "raster_center_csv"
    RASTER_CENTER_SHAPE_DIR = WORKING_FOLDER / "raster_center_shapes"

    POSTGRESQL_PORT = int(os.environ.get('POSTGRESQL_PORT', '5433'))
    POSTGRESQL_HOST = os.environ.get('POSTGRESQL_HOST', '127.0.0.1')
    POSTGRESQL_USERNAME = os.environ.get('POSTGRESQL_USERNAME', 'postgres')
    POSTGRESQL_PASSWORD = os.environ.get('POSTGRESQL_PASSWORD', 'P@ssw0rd')
    POSTGRESQL_DATABASE = MODULE_NAME.lower()
    POSTGRESQL_TABLESPACE = os.environ.get('POSTGRESQL_TABLESPACE', 'pg_default')


    LOG_PATH = WORKING_FOLDER / "logs" / "log.txt"
    # for rust
    LOG_LEVEL = "warn"
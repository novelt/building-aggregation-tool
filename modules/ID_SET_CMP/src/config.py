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

import os
from pathlib import Path


class Config(object):
    MODULE_NAME = "ID_SET_CMP"
    MODULE_DIR = Path("/modules") / MODULE_NAME
    COUNTRY_CODE = os.environ["COUNTRY_CODE"].upper()

    BASE_DATA_DIR = MODULE_DIR / "input" / COUNTRY_CODE
    WORKING_FOLDER = MODULE_DIR / "working" / COUNTRY_CODE

    SCHEMA_NAME = COUNTRY_CODE.lower()

    # this will be overwritten by the command line --clean
    CLEAN = False

    YEAR1_DIR = BASE_DATA_DIR / "year1"
    YEAR2_DIR = BASE_DATA_DIR / "year2"

    YEAR1_FIXED = WORKING_FOLDER / "year1_fixed.fgb"

    REF_RASTER_PATH = BASE_DATA_DIR / "ref_raster"

    RASTER_DIR = WORKING_FOLDER / "rasters"
    YEAR1_RASTER = RASTER_DIR / "year1.tif"
    YEAR1_ID_FIELD = "orig_fid"
    YEAR2_RASTER = RASTER_DIR / "year2.tif"
    YEAR2_ID_FIELD = "orig_fid"

    POSTGRESQL_PORT = int(os.environ.get('POSTGRESQL_PORT', '5433'))
    POSTGRESQL_HOST = os.environ.get('POSTGRESQL_HOST', '127.0.0.1')
    POSTGRESQL_USERNAME = os.environ.get('POSTGRESQL_USERNAME', 'postgres')
    POSTGRESQL_PASSWORD = os.environ.get('POSTGRESQL_PASSWORD', 'P@ssw0rd')
    POSTGRESQL_DATABASE = MODULE_NAME.lower()
    POSTGRESQL_TABLESPACE = os.environ.get('POSTGRESQL_TABLESPACE', 'pg_default')

    CSV_OUTPUT = WORKING_FOLDER / "csv"

    LOG_PATH = WORKING_FOLDER / "logs" / "log.txt"
    # for rust
    LOG_LEVEL = "warn"
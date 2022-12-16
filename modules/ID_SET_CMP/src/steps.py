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

import logging

from pathlib import Path
from typing import Tuple

from config import Config as cfg
from novelt.common_steps import check_clean_work_file, check_clean_work_dir
from novelt.lib import geo_db_utils, file_utils, db_utils
from novelt.lib.thread_utils import run_process_stream_output

log = logging.getLogger(__name__)


def print_input_info():
    """
    Parses the inputs needed for the tool and prints info related to it to the log
    """

    log.info("Tool to compare ids from year1 and year2")

    star_line = '*' * 80

    ref_raster_path = get_ref_raster()
    y1_input = get_year1_input()
    y2_input = get_year2_input()

    log_txt = f"""
    {star_line}
    Reference Raster: {ref_raster_path}
    {star_line}        
    Year 1 input {y1_input[0]} {y1_input[1]}
    Year 2 input {y2_input[0]} {y2_input[1]}\n{star_line}\n\n"""

    log.info(log_txt)


def step_create_new_database():
    """
    Creates an empty PostGIS database in the db docker container
    """

    geo_db_utils.create_database(
        cfg=cfg,
        drop_if_exists=False, add_postgis=True)


def get_ref_raster() -> Path:
    matches = []

    err_message = f"Expected 1 ref raster tif in {cfg.REF_RASTER_PATH}.  This raster can be taken from BLDG_AGG/working/<country code>/rasters/ref_expanded.tif after running the tool"

    if not cfg.REF_RASTER_PATH.exists():
        raise Exception(f"{err_message}.  Path not found.")

    for r in cfg.REF_RASTER_PATH.iterdir():
        if r.is_dir():
            continue
        if str(r.suffix).upper() != ".TIF":
            continue
        matches.append(r)

    if len(matches) != 1:
        raise Exception(err_message)

    return matches[0]

def get_year1_input() -> Tuple[Path, str]:
    err_msg = f"The year1 directory {cfg.YEAR1_DIR} should contain 1 layer containing the year1 input"
    lst = file_utils.get_vector_layers(cfg.YEAR1_DIR, err_msg)

    if len(lst) != 1:
        print(f"Found {len(lst)} entries")
        print(err_msg)

    return lst[0]

def get_year2_input() -> Tuple[Path, str]:
    err_msg = f"The year1 directory {cfg.YEAR2_DIR} should contain 1 layer containing the year1 input"
    lst = file_utils.get_vector_layers(cfg.YEAR2_DIR, err_msg)

    if len(lst) != 1:
        print(f"Found {len(lst)} entries")
        print(err_msg)

    return lst[0]


def run_fix(in_dataset, in_layer: str, output_path: Path):
    if check_clean_work_file(cfg, output_path):
        return

    rust_cmd_parts = [
        'cargo',
        'run',
        '--bin',
        'cmdline_tools',
        '--release',
        '--',
        'fix-geom',
        '--input-dataset',
        f'"{in_dataset}"',
        '--input-layer',
        f'{in_layer}',
        '--output-dataset',
        f'"{output_path}"',
    ]

    rust_cmd = ' '.join(rust_cmd_parts)

    log_path = cfg.LOG_PATH.parent / f"gdal.txt"
    log_path.unlink(missing_ok=True)

    run_process_stream_output(rust_cmd, cwd="/rust", env_override={
        "CPL_LOG": log_path
    })

def step_fix_year1_input():
    """
    Year1 input can have geometry errors
    """

    year1_input = get_year1_input()
    run_fix(year1_input[0], year1_input[1], cfg.YEAR1_FIXED)

def step_rasterize_year1_input():
    """
    Rasterize the year1 vector input
    """

    step_rasterize_common(cfg.YEAR1_RASTER, (cfg.YEAR1_FIXED, cfg.YEAR1_FIXED.stem), cfg.YEAR1_ID_FIELD)

def step_rasterize_year2_input():
    """
    Rasterize the year2 vector input
    """


    year2_input = get_year2_input()

    step_rasterize_common(cfg.YEAR2_RASTER, year2_input, cfg.YEAR2_ID_FIELD)



def step_rasterize_common(raster_output: Path, layer_path_name: Tuple[Path,str], id_field: str):


    if check_clean_work_file(cfg, raster_output):
        return

    ref_raster = get_ref_raster()

    rust_cmd_parts = [
        'cargo',
        'run',
        '--bin',
        'cmdline_tools',
        '--release',
        '--',
        'burn-polygon-to-raster',
        '--layer-name',
        f'"{layer_path_name[1]}"',
        '--ogr-conn-str',
        f'"{layer_path_name[0]}"',
        '--snap-raster',
        f'"{ref_raster}"',
        "--no-data-value=-1.0",
        "--data-type Int32",
        f"--burn-field {id_field}",
        '--output-raster',
        f'"{raster_output}"',
    ]

    rust_cmd = ' '.join(rust_cmd_parts)

    log_path = cfg.LOG_PATH.parent / f"gdal.txt"
    log_path.unlink(missing_ok=True)

    run_process_stream_output(rust_cmd, cwd="/rust", env_override={
        "CPL_LOG": log_path
    })

def step_create_new_database():
    """
    Creates an empty PostGIS database in the db docker container
    """

    geo_db_utils.create_database(
        cfg=cfg,
        drop_if_exists=False, add_postgis=True)

def step_squares_to_database(conn):
    """
    Exports rasters squares to database
    """

    year1_input = [cfg.YEAR1_FIXED, cfg.YEAR1_FIXED.stem]
    year2_input = get_year2_input()

    rust_cmd_parts = [
        'cargo',
        'run',
        '--bin',
        'cmdline_tools',
        '--release',
        '--',
        '--log-level trace',
        'id-set-cmp',
        '--year1-raster',
        f'"{cfg.YEAR1_RASTER}"',
        '--year2-raster',
        f'"{cfg.YEAR2_RASTER}"',
        '--y1-layer-name',
        f'"{year1_input[1]}"',
        '--y1-ogr-conn-str',
        f'"{year1_input[0]}"',
        '--y2-layer-name',
        f'"{year2_input[1]}"',
        '--y2-ogr-conn-str',
        f'"{year2_input[0]}"',
        f'--y1-id-field {cfg.YEAR1_ID_FIELD}',
        f'--y2-id-field {cfg.YEAR2_ID_FIELD}',
        '--pg-conn-str',
        f"\"{db_utils.get_sql_alchemy_connection_string(cfg)}\"",
        '--schema',
        f'"{cfg.SCHEMA_NAME}"',
    ]

    rust_cmd = ' '.join(rust_cmd_parts)

    run_process_stream_output(rust_cmd, cwd="/rust", )

def step_export_csvs(conn):
    """
    Exports CSVS
    """

    if check_clean_work_dir(cfg, cfg.CSV_OUTPUT):
        return

    run_process_stream_output(f"chmod 777 {cfg.CSV_OUTPUT}")

    output_1 = cfg.CSV_OUTPUT / "year1_lookup.csv"

    sql = f"""
    COPY (
    select year1_id, count(*) from {cfg.SCHEMA_NAME}.squares
    group by year1_id
    ) TO '{output_1}' DELIMITER ',' CSV HEADER;
    """

    db_utils.run_sql(conn, sql)

    #################################################

    output_2 = cfg.CSV_OUTPUT / "year2_lookup.csv"

    sql = f"""
        COPY (
        select year2_id, count(*) from {cfg.SCHEMA_NAME}.squares
        group by year2_id
        ) TO '{output_2}' DELIMITER ',' CSV HEADER;
        """

    db_utils.run_sql(conn, sql)

    #################################################

    output_2 = cfg.CSV_OUTPUT / "year2_lookup.csv"

    sql = f"""
                COPY (
                select year2_id, count(*) from {cfg.SCHEMA_NAME}.squares
                group by year2_id
                ) TO '{output_2}' DELIMITER ',' CSV HEADER;
                """

    db_utils.run_sql(conn, sql)

    #################################################

    output = cfg.CSV_OUTPUT / "year1_overlap.csv"

    sql = f"""
COPY (
    select year1_id, count(distinct year2_id) as y1_overlaps
    FROM {cfg.SCHEMA_NAME}.squares
   -- WHERE year2_id is not null
    GROUP BY year1_id
) TO '{output}' DELIMITER ',' CSV HEADER;
            """

    db_utils.run_sql(conn, sql)

    #################################################

    output = cfg.CSV_OUTPUT / "year2_overlap.csv"

    sql = f"""
    COPY (
    select year2_id, count(distinct year1_id) as y2_overlaps
    FROM {cfg.SCHEMA_NAME}.squares
    --WHERE year1_id is not null
    GROUP BY year2_id
                ) TO '{output}' DELIMITER ',' CSV HEADER;
                """

    db_utils.run_sql(conn, sql)

    #################################################



    main_csv = cfg.CSV_OUTPUT / "main.csv"

    sql = f"""
COPY (    
    WITH y1_counts AS (
        select year1_id, count(*) as y1count 
        FROM {cfg.SCHEMA_NAME}.squares
        GROUP BY year1_id
    ),
        y2_counts as (
        SELECT year2_id, count(*) as y2count 
        FROM {cfg.SCHEMA_NAME}.squares
        group by year2_id 
    ),
        y2_overlaps as (
        SELECT year2_id, count(distinct year1_id) as y2_overlaps 
        FROM {cfg.SCHEMA_NAME}.squares
        GROUP BY year2_id
     ),
        grouped as (
        SELECT
           COALESCE(year2_id::text, 'NA') || '_' || COALESCE(year1_id::text, 'NA') as "y2USI_Y1UID",
           COUNT(*) as "Count",
           year2_id AS "Y2UID",
           year1_id AS "Y1UID"
        FROM {cfg.SCHEMA_NAME}.squares
        GROUP BY year1_id, year2_id 
    )
    SELECT g."y2USI_Y1UID", 
        g."Count", 
        g."Y2UID", 
        g."Y1UID",
        y2c.y2count as "Y2_Count",
        y1c.y1count as "Y1_Count",
        y2o.y2_overlaps as "Y2_overlaps",
        Round(100.0 * g."Count" / y2c.y2count,2) as "Y2_ratio",
        Round(100.0 * g."Count" / y1c.y1count,2) as "Y1_ratio"
    FROM grouped g
    LEFT JOIN y1_counts y1c ON g."Y1UID" = y1c.year1_id
    LEFT JOIN y2_counts y2c ON g."Y2UID" = y2c.year2_id
    LEFT JOIN y2_overlaps y2o ON g."Y2UID" = y2o.year2_id    
) TO '{main_csv}' DELIMITER ',' CSV HEADER;     
    """

    db_utils.run_sql(conn, sql)


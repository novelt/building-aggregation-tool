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

import copy
import logging
import math
import queue
import sys
from functools import partial
from pathlib import Path
from typing import Tuple, List

import affine
import fiona
import rasterio
from osgeo import gdal

from config import Config as cfg
from novelt.common_steps import grid_slice_helper, import_settlements_to_be_helper, check_clean_work_dir, \
    check_clean_work_file, compile_building_agg, import_chunk
from novelt.lib import file_utils, db_utils, thread_utils, geo_db_utils
from novelt.lib.raster_utils import RasterStats
from novelt.lib.thread_utils import run_process_stream_output

log = logging.getLogger(__name__)

# how many bytes are in an empty FGB that has a geospatial index
EMPTY_FGB_SIZE_WITH_INDEX = 628

def get_ref_raster() -> Path:
    if not cfg.RASTER_INPUT_PATH.exists():
        log.error(f"""Directory {cfg.RASTER_INPUT_PATH} does not exist.  The tool requires a grid raster in this directory
        Recommended to use the worldpop Grid-cell surface area rasters available here: https://www.worldpop.org/geodata/listing?id=59""")
        sys.exit(1)

    files = list([f for f in cfg.RASTER_INPUT_PATH.iterdir() if f.suffix.upper() == ".TIF"])

    if len(files) != 1:
        log.error(
            f"""Expected only 1 file in the directory {cfg.RASTER_INPUT_PATH}.  Should contain 1 tif raster""")
        sys.exit(1)

    raster = files[0]

    return raster


def get_building_inputs() -> List[Tuple[Path, str]]:

    return file_utils.get_vector_layers(cfg.BUILDING_INPUT_DIR, "It should contain the geospatial files containing the buildings")


def get_projected_extent():
    output_txt = cfg.WORKING_FOLDER / "building_projected_extent.txt"

    if check_clean_work_file(cfg, output_txt):

        with output_txt.open('r') as f:
            contents = f.read()
        return [float(line) for line in contents.split("\n")]

    ref_raster = get_ref_raster()

    with rasterio.open(ref_raster) as r:
        raster_crs = r.crs

    rust_cmd_parts = [
        'cargo',
        'run',
        '--bin',
        'cmdline_tools',
        '--release',
        '--',
        'get-projected-extent',
        f"--output-file=\"{output_txt}\"",
        f"--output-proj=\'{raster_crs.to_wkt()}\'",
    ]

    bldg_inputs = get_building_inputs()

    for input_path, input_layer in bldg_inputs:
        rust_cmd_parts.append(f"-c \"{input_path}\"")
        rust_cmd_parts.append(f"-l \"{input_layer}\"")

    rust_cmd = ' '.join(rust_cmd_parts)

    run_process_stream_output(rust_cmd, cwd="/rust", )

    with output_txt.open('r') as f:
        contents = f.read()
    return [float(line) for line in contents.split("\n")]


def print_input_info():
    """
    Parses the inputs needed for the tool and prints info related to it to the log
    """

    ref_raster_path = get_ref_raster()

    ref_raster = rasterio.open(ref_raster_path)

    log_txt = ""

    star_line = '*' * 80

    log_txt += f"""
{star_line}
Reference Raster
{star_line}        
Using grid reference raster at {ref_raster_path}
Reference raster CRS = {ref_raster.crs}
Cell width           = {ref_raster.transform[0]:.6f}
Cell height          = {ref_raster.transform[4]:.6f}\n{star_line}\n\n"""

    input_sources = get_building_inputs()

    log_txt += f"""{star_line}\nBuildings\n{star_line}\n"""

    total_buildings = 0
    for path, layer_name in input_sources:
        with fiona.open(path, layer=layer_name) as recs:
            feature_count = len(recs)
        log_txt += f"Found {layer_name} in {path} with {feature_count:,} buildings\n"
        total_buildings += feature_count

    log_txt += f"Total building count: {total_buildings:,}\n{star_line}\n"

    log_txt += f"""
{star_line}
Config
{star_line}
Buildings polygons will increase in size by this distance.
Building buffer distance (meters) = {cfg.BUFFER_SIZE}

Building grouping distance (in reference raster CRS coordinates).
Buildings less than or equal to this distance will be grouped together.
Grouping distance = {cfg.GROUP_DISTANCE}
{star_line}   
    """

    log.info(log_txt)


def step_create_new_database():
    """
    Creates an empty PostGIS database in the db docker container
    """

    geo_db_utils.create_database(
        cfg=cfg,
        drop_if_exists=False, add_postgis=True)


def run_fix(in_dataset, in_layer, output_path):
    if output_path.exists():
        log.info(f"{output_path} exists, skipping...")
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

    run_process_stream_output(rust_cmd, cwd="/rust", )


def fix_reproject_split_buildings():
    """
This step will
1.  Fix any invalid geometry in the raw building input
2.  Reproject the geometry to the reference raster CRS
3.  Split the input into 100 chunks, splitting the reference raster extent into 10x10 equal sized squares

The chunks are written to the file system in the FlatGeoBuf format under the split directory
    """

    if check_clean_work_dir(cfg, cfg.SPLIT_BUILDING_PATH):
        return

    rust_cmd_parts = [
        'cargo',
        'run',
        '--bin',
        'bldg_agg',
        '--release',
        '--',
        'fix-reproject-split',
        f'--chunk-rows \"{cfg.CHUNK_ROWS}\"',
        f'--chunk-cols \"{cfg.CHUNK_COLS}\"',
        f"--snap-raster-path \"{cfg.RASTER_EXPANDED_REF}\"",
        f"--output-path \"{cfg.SPLIT_BUILDING_PATH}\"",
        "--use-centroid",
    ]

    bldg_input = get_building_inputs()

    for input_path, input_layer in bldg_input:
        rust_cmd_parts.append(f"-c \"{input_path}\"")
        rust_cmd_parts.append(f"-l \"{input_layer}\"")

    rust_cmd = ' '.join(rust_cmd_parts)

    log_path = cfg.LOG_PATH.parent / f"buildings.txt"

    log_path.unlink(missing_ok=True)

    run_process_stream_output(rust_cmd, cwd="/rust", env_override={
                                      "CPL_LOG": log_path
                                  })


def create_building_count_raster():
    """
    Converts building counts into a raster.  This is used to speed up the calculation of building count to settlement
    """

    if check_clean_work_file(cfg, cfg.RASTER_BUILDING_COUNT):
        return

    rust_cmd_parts = [
        'cargo',
        'run',
        '--bin',
        'cmdline_tools',
        '--release',
        '--',
        'burn-count-to-raster',
    ]

    for input_path in cfg.CENTROIDS_BUILDINGS_PATH.iterdir():

        if input_path.stat().st_size <= EMPTY_FGB_SIZE_WITH_INDEX:
            continue

        rust_cmd_parts.append(f"-c \"{input_path}\"")
        rust_cmd_parts.append(f"-l \"{input_path.stem}\"")

    rust_cmd_parts.extend([
        f"--snap-raster \"{cfg.RASTER_EXPANDED_REF}\"",
        f"--output-tif \"{cfg.RASTER_BUILDING_COUNT}\"",
    ])

    rust_cmd = ' '.join(rust_cmd_parts)

    run_process_stream_output(rust_cmd, cwd="/rust", )


def step_contour_lines(conn):
    """
    Creates contour polygons from the building count raster.

    These polygons will surround raster squares that have at least the number of buildings specified by
    `--contour-value`

    """

    if cfg.CLEAN:
        db_utils.drop_table(conn,
                            cfg.SCHEMA_NAME,
                            "contours",
                            True)

    db_utils.create_schema(conn, cfg.SCHEMA_NAME)

    cmd_parts = [
        'cargo',
        'run',
        '--release',
        '--bin cmdline_tools',
        '--',
        "contours",
        "--input-raster",
        f"\"{cfg.RASTER_BUILDING_COUNT}\"",
        "--schema-name",
        f"\"{cfg.SCHEMA_NAME}\"",
        "--pg-conn-str",
        f"\"{db_utils.get_sql_alchemy_connection_string(cfg)}\"",
        f"--line-layer-name \"contours\"",
        f"--contour-value {cfg.CONTOUR_VALUE}",

    ]

    cmd = ' '.join(cmd_parts)

    run_process_stream_output(cmd, cwd="/rust", env_override={

    })

def step_contour_poly_stats(conn):
    """
    Calculates m2 area of contour lines
    """
    col_names = db_utils.get_columns(conn, cfg.SCHEMA_NAME, cfg.CONTOUR_POLYGON_TABLE_NAME)

    if "area_m2" not in col_names:
        db_utils.run_sql(conn, f"""
       ALTER TABLE "{cfg.SCHEMA_NAME}"."{cfg.CONTOUR_POLYGON_TABLE_NAME}"
       ADD COLUMN area_m2 float 
               """)

    sql = f"""
       UPDATE "{cfg.SCHEMA_NAME}"."{cfg.CONTOUR_POLYGON_TABLE_NAME}" 
       SET area_m2 = ST_AREA(shape::geography, true)
               """
    db_utils.run_sql(conn, sql)

    db_utils.create_index(conn, cfg.SCHEMA_NAME, cfg.CONTOUR_POLYGON_TABLE_NAME, "shape", True)

def step_group_buildings():
    """
    Groups near buildings to multipolygons.  This will use the group distance.

    Note this distance is in degrees.  Example, this is roughly 130 meters

    `/run_bldg_agg.sh BLDG_AGG --country TGO --clean --group-distance=0.0012 4`

    Each MultiPolygon in the output will contain all building polygons within this distance to each other.
    """

    if check_clean_work_dir(cfg, cfg.GROUPED_BUILDINGS_BASE_PATH):
        return

    temp_dir = cfg.WORKING_FOLDER / "temp_group_to_multi"

    if temp_dir.exists():
        file_utils.remove_dir(temp_dir, cfg.WORKING_FOLDER)

    rust_cmd_parts = [
        'cargo',
        'build',
        '--bin',
        'cmdline_tools',
        '--release',
    ]

    rust_cmd = ' '.join(rust_cmd_parts)
    run_process_stream_output(rust_cmd, cwd="/rust")

    task_queue = queue.Queue()

    task_count = 0

    file_utils.mkdir_p(cfg.GROUPED_BUILDINGS_BASE_PATH)

    for fgb in cfg.SPLIT_BUILDING_PATH.iterdir():

        # skip empty partitions
        if fgb.stat().st_size <= EMPTY_FGB_SIZE_WITH_INDEX:
            continue

        task_count += 1

        split_temp_dir = temp_dir / fgb.stem
        file_utils.mkdir_p(split_temp_dir)

        grouped_path = cfg.GROUPED_BUILDINGS_BASE_PATH / fgb.name

        # Ideally should check the raster projection of the ref raster is in fact 4326
        rust_cmd_parts = [
            './cmdline_tools',
            'group-to-multi',
            f"--out-ogr-conn {grouped_path}",
            f"--out-ogr-layer {grouped_path.stem}",
            f"--in-ogr-conn \"{fgb}\"",
            f"--in-ogr-layer \"{fgb.stem}\"",
            f"--temp-dir \"{split_temp_dir}\"",
            f"--width {cfg.GROUP_DISTANCE}",
        ]

        rust_cmd = ' '.join(rust_cmd_parts)

        task_queue.put(partial(
            run_process_stream_output,
            rust_cmd, cwd="/rust/target/release", ))

    thread_utils.finish_threads_with_context(task_queue=task_queue,
                                             fn_context_create=None,
                                             max_num_processes=4, num_items_in_queue=task_count)


def step_fill_center_building_groups():
    """
    For each multipolygon of buildings, will replace buildings that are not near the center
    with a rasterized polygon of the interior.

    """

    if check_clean_work_dir(cfg, cfg.FILLED_BUILDINGS_BASE_PATH):
        return

    compile_building_agg()

    task_queue = queue.Queue()

    task_count = 0

    file_utils.mkdir_p(cfg.FILLED_BUILDINGS_BASE_PATH)

    for fgb in cfg.GROUPED_BUILDINGS_BASE_PATH.iterdir():
        task_count += 1

        # if fgb.stem != "buildings_66":
        #     continue

        filled_path = cfg.FILLED_BUILDINGS_BASE_PATH / fgb.name

        # Ideally should check the raster projection of the ref raster is in fact 4326
        rust_cmd_parts = [
            './bldg_agg',
            'fill-center-building-groups',
            f"--snap-raster-path \"{cfg.RASTER_EXPANDED_REF}\"",
            f"--out-ogr-conn {filled_path}",
            f"--out-ogr-layer {filled_path.stem}",
            f"--in-ogr-conn \"{fgb}\"",
            f"--in-ogr-layer \"{fgb.stem}\"",
        ]

        rust_cmd = ' '.join(rust_cmd_parts)

        task_queue.put(partial(
            run_process_stream_output,
            rust_cmd,
            cwd="/rust/target/release",
            # cwd="/rust/target/debug",
            # env_override = {"RUST_BACKTRACE" : "1"}
        ))

    thread_utils.finish_threads_with_context(task_queue=task_queue,
                                             fn_context_create=None,
                                             max_num_processes=4, num_items_in_queue=task_count)


def step_buffer_buildings():
    """
    Buffers the buildings, only the ones that are outside the contour

    The input is the filled multipolygon buildings (1 shape for the buildings and the filled interior)

    NOTE!  If this step fails due to memory, try increasing --chunk-rows and --chunk-cols and rerunning from the split step
    """

    if check_clean_work_dir(cfg, cfg.BUFFERED_BUILDINGS_BASE_PATH):
        return

    rust_cmd_parts = [
        'cargo',
        'build',
        '--bin',
        'buffer_shapes',
        '--release',
    ]

    rust_cmd = ' '.join(rust_cmd_parts)
    run_process_stream_output(rust_cmd, cwd="/rust")

    task_queue = queue.Queue()

    task_count = 0

    file_utils.mkdir_p(cfg.BUFFERED_BUILDINGS_BASE_PATH)

    for fgb in cfg.FILLED_BUILDINGS_BASE_PATH.iterdir():
        task_count += 1

        buffered_path = cfg.BUFFERED_BUILDINGS_BASE_PATH / fgb.name

        rust_cmd_parts = [
            './buffer_shapes',
            f"--in-ogr-conn {fgb}",
            f"--in-ogr-layer {fgb.stem}",
            f"--out-ogr-conn {buffered_path}",
            f"--out-ogr-layer {buffered_path.stem}",
            f"--buffer-meters {cfg.BUFFER_SIZE}",
            "--quad-segs 2"
        ]

        rust_cmd = ' '.join(rust_cmd_parts)

        task_queue.put(partial(
            run_process_stream_output,
            rust_cmd,
            cwd="/rust/target/release",
        ))

    thread_utils.finish_threads_with_context(task_queue=task_queue,
                                             fn_context_create=None,
                                             max_num_processes=8, num_items_in_queue=task_count)


def step_union_all_buffers():
    """
    Union/Dissolves all the buffered building geometry.  This ensures any intersecting geometries will become
    part of the same settlement geometry
    """
    if check_clean_work_file(cfg, cfg.UNIONED_NO_HOLES_PATH):
        return

    # Ideally should check the raster projection of the ref raster is in fact 4326
    rust_cmd_parts = [
        'cargo',
        'run',
        '--bin',
        'union_shapes',
        '--release',
        '--',
        f"--out-ogr-conn \"{cfg.UNIONED_NO_HOLES_PATH}\"",
        f"--out-ogr-layer \"{cfg.UNIONED_NO_HOLES_PATH.stem}\"",
        "--out-driver FlatGeoBuf",
        "--no-holes"
    ]

    for fgb in cfg.BUFFERED_BUILDINGS_BASE_PATH.iterdir():
        rust_cmd_parts.extend([
            f"-c \"{fgb}\"",
            f"-l \"{fgb.stem}\"",
        ])

    rust_cmd = ' '.join(rust_cmd_parts)

    run_process_stream_output(rust_cmd, cwd="/rust", env_override={
        # "PG_USE_COPY": "YES"
    })

def step_create_centroids():
    """
    Create centroids
    """

    if check_clean_work_dir(cfg, cfg.CENTROIDS_BUILDINGS_PATH):
        return

    compile_building_agg()

    task_queue = queue.Queue()

    task_count = 0

    for fgb in cfg.SPLIT_BUILDING_PATH.iterdir():
        task_count += 1

        out_path = cfg.CENTROIDS_BUILDINGS_PATH / fgb.name

        # Ideally should check the raster projection of the ref raster is in fact 4326
        rust_cmd_parts = [
            './bldg_agg',
            'create-centroids',
            f"--in-fgb \"{fgb}\"",
            f"--out-fgb \"{out_path}\"",
        ]

        rust_cmd = ' '.join(rust_cmd_parts)

        task_queue.put(partial(
            run_process_stream_output,
            rust_cmd,
            cwd="/rust/target/release",
            # cwd="/rust/target/debug",
            # env_override = {"RUST_BACKTRACE" : "1"}
        ))

    thread_utils.finish_threads_with_context(task_queue=task_queue,
                                             fn_context_create=None,
                                             max_num_processes=4, num_items_in_queue=task_count)


def create_settlement_table(conn):
    """
    xfer settlements to database
    """

    db_utils.create_schema(conn, cfg.SCHEMA_NAME)

    db_utils.drop_table(conn, cfg.SCHEMA_NAME, cfg.FINAL_SETTLEMENT_TABLE_NAME, cascade=True)

    rust_cmd_parts = [
        'ogr2ogr',
        '--config PG_USE_COPY YES',
        "-f PostgreSQL",
        '-lco OVERWRITE=YES',
        '-lco GEOMETRY_NAME=shape',
        '-lco FID=orig_fid',
        "-progress",
        f"\"{db_utils.get_ogr_connection_string(cfg)}\"",
        f'-nln {cfg.SCHEMA_NAME}.{cfg.FINAL_SETTLEMENT_TABLE_NAME}',
        f"\"{cfg.FINAL_SETTLEMENT_SHAPES_PATH}\"",
    ]

    rust_cmd = ' '.join(rust_cmd_parts)

    run_process_stream_output(rust_cmd, cwd="/rust", )


def step_area_bldg_count(conn):
    """
    Go through the unioned polygons / shapes
    calculate area in m2
    and total building count.

    These values are used to categorize the shapes into
    1.  Built-up Areas (> 3000 buildings)
    2.  Small Settlement Areas (> 50 buildings)
    3.  Hamlets (<= 50 buildings)
    """

    # for the building count, we could rasterize the union layer
    # then zonal stats with the building count raster

    if check_clean_work_file(cfg, cfg.SETTLEMENTS_RASTERIZED_PATH):
        return

    rust_cmd_parts = [
        'cargo',
        'run',
        '--release',
        '--bin cmdline_tools',
        '--',
        'burn-polygon-to-raster',
        '--layer-name',
        f'{cfg.SCHEMA_NAME}.{cfg.FINAL_SETTLEMENT_TABLE_NAME}',
        '--ogr-conn-str',
        f'"{db_utils.get_ogr_connection_string(cfg)}"',
        '--snap-raster',
        f'"{cfg.RASTER_EXPANDED_REF}"',
        # '--clean',
        '--burn-field orig_fid',
        '--output-raster',
        f'"{cfg.SETTLEMENTS_RASTERIZED_PATH}"',
    ]

    rust_cmd = ' '.join(rust_cmd_parts)

    run_process_stream_output(rust_cmd, cwd="/rust", )

    zs_csv_path = cfg.WORKING_FOLDER / f"{cfg.FINAL_SETTLEMENT_TABLE_NAME}.csv"

    rust_cmd_parts = [
        'cargo',
        'run',
        '--release',
        '--bin zonal_stats',
        '--',
        '--feature-raster',
        f"\"{cfg.SETTLEMENTS_RASTERIZED_PATH}\"",
        '--data-raster',
        f"\"{cfg.RASTER_BUILDING_COUNT}\"",
        '--summary-csv',
        f"\"{zs_csv_path}\"",
        '--clean'
    ]

    rust_cmd = ' '.join(rust_cmd_parts)

    log.debug("Running fast zonal stats: {}".format(rust_cmd))

    run_process_stream_output(rust_cmd, cwd="/rust")

    db_utils.import_zonalstats_csv_helper(
        conn, csv_path=zs_csv_path,
        csv_schema_name=cfg.SCHEMA_NAME,
        csv_table_name=f"zs_{cfg.FINAL_SETTLEMENT_TABLE_NAME}",
    )

    col_names = db_utils.get_columns(conn, cfg.SCHEMA_NAME, cfg.FINAL_SETTLEMENT_TABLE_NAME)

    if "bldg_count" not in col_names:
        db_utils.run_sql(conn, f"""
ALTER TABLE "{cfg.SCHEMA_NAME}"."{cfg.FINAL_SETTLEMENT_TABLE_NAME}"
ADD COLUMN bldg_count int 
        """)

    if "area_m2" not in col_names:
        db_utils.run_sql(conn, f"""
    ALTER TABLE "{cfg.SCHEMA_NAME}"."{cfg.FINAL_SETTLEMENT_TABLE_NAME}"
    ADD COLUMN area_m2 float 
            """)

    if "level" not in col_names:
        db_utils.run_sql(conn, f"""
    ALTER TABLE "{cfg.SCHEMA_NAME}"."{cfg.FINAL_SETTLEMENT_TABLE_NAME}"  
    ADD COLUMN level smallint  
            """)

    sql = f"""
        
        UPDATE "{cfg.SCHEMA_NAME}"."{cfg.FINAL_SETTLEMENT_TABLE_NAME}" AS a
        SET bldg_count = zs.square_sum,
        area_m2 = ST_AREA(shape::geography, true)
        FROM "{cfg.SCHEMA_NAME}".zs_{cfg.FINAL_SETTLEMENT_TABLE_NAME} zs 
        WHERE zs.feature_id = a.orig_fid ; 
            """
    db_utils.run_sql(conn, sql)

    sql = f"""
        UPDATE "{cfg.SCHEMA_NAME}"."{cfg.FINAL_SETTLEMENT_TABLE_NAME}"
        set level = NULL 
        """

    db_utils.run_sql(conn, sql)

    sql = f"""
    UPDATE "{cfg.SCHEMA_NAME}"."{cfg.FINAL_SETTLEMENT_TABLE_NAME}"
    set level = 2
    where bldg_count > 3000;
    """

    db_utils.run_sql(conn, sql)

    sql = f"""
        UPDATE "{cfg.SCHEMA_NAME}"."{cfg.FINAL_SETTLEMENT_TABLE_NAME}"
        set level = 1 -- type = 'ssa'
        where bldg_count > 50
        AND level IS NULL 
        """

    db_utils.run_sql(conn, sql)

    sql = f"""
            UPDATE "{cfg.SCHEMA_NAME}"."{cfg.FINAL_SETTLEMENT_TABLE_NAME}"
            set level = 0
            where level IS NULL 
            """

    db_utils.run_sql(conn, sql)


def step_make_buas_from_contours(conn):
    """
    If a HA or SSA intersects a contour of big enough area, the type is set to BUA.

    This is to handle HAs / SSAs that have a dense enough concentration of buildings

    The area of the contour must be at least the number of square meters specified by `--contour-min-area`.
    """

    sql = f"""
    UPDATE {cfg.SCHEMA_NAME}."{cfg.FINAL_SETTLEMENT_TABLE_NAME}" as u
    SET level = 2
    FROM {cfg.SCHEMA_NAME}.{cfg.CONTOUR_POLYGON_TABLE_NAME} c 
    WHERE 
        ST_Intersects(c.shape, u.shape)
        AND u.level < 2
        AND c.area_m2 >= {cfg.CONTOUR_MIN_BUA_AREA}
        AND c.is_hole IS FALSE
        --NOT exists a hole that completely contains the settlement 
        AND NOT EXISTS (
            SELECT 1 FROM {cfg.SCHEMA_NAME}.{cfg.CONTOUR_POLYGON_TABLE_NAME} hole
            WHERE ST_Contains(hole.shape, u.shape)
            AND hole.is_hole IS TRUE 
        );
"""

    count = db_utils.run_sql(conn, sql)

    log.debug(f"Updated {count} with\n{sql}")

def step_test_case(conn):
    """
    Tries a test for inner and outer shell intersecting
    """

    min_x = 43.16159
    min_y = 11.55538

    width = 0.01
    height = 0.01

    # try 3 things

    # first as inner and outer shell
    # second as 1 outer shell
    # third as just inner
    # 4th just other (to do st_difference)

    num_x_coords = 5
    num_y_coords = 5

    step_x = width / num_x_coords
    step_y = height / num_y_coords

    x_coords = [x * step_x + min_x for x in range(0, num_x_coords)]
    y_coords = [y * step_y + min_y for y in range(0, num_y_coords)]

    schema_name = "test"
    table_name = "polygon"

    db_utils.drop_schema(conn, schema_name)
    db_utils.create_schema(conn, schema_name)

    db_utils.run_sql(conn, f"""
    CREATE TABLE {schema_name}.{table_name} (
            id serial PRIMARY KEY NOT NULL,            
            name varchar  NOT NULL,            
            shape Geometry(Polygon, 4326)
    );
    
    INSERT INTO {schema_name}.{table_name}
    (name, shape)
    VALUES ( 'Inner/outer shell',
    ST_SetSrid(ST_MakePolygon( 'LINESTRING(
        {x_coords[0]} {y_coords[0]},
        {x_coords[1]} {y_coords[0]},
        {x_coords[2]} {y_coords[1]},
        {x_coords[3]} {y_coords[0]},
        {x_coords[4]} {y_coords[0]},
        {x_coords[4]} {y_coords[4]},
        {x_coords[0]} {y_coords[4]},
        {x_coords[0]} {y_coords[0]}
    )',
    ARRAY['LINESTRING(
        {x_coords[1]} {y_coords[2]},
        {x_coords[2]} {y_coords[3]},
        {x_coords[3]} {y_coords[2]},
        {x_coords[2]} {y_coords[1]},
        {x_coords[1]} {y_coords[2]}
    )'  ]
    ), 4326))    ;

    INSERT INTO {schema_name}.{table_name}
    (name, shape)
    VALUES ( 'Just outer polygon',
    ST_SetSrid(ST_MakePolygon( 'LINESTRING(
        {x_coords[0]} {y_coords[0]},
        {x_coords[1]} {y_coords[0]},
        {x_coords[2]} {y_coords[1]},
        {x_coords[3]} {y_coords[0]},
        {x_coords[4]} {y_coords[0]},
        {x_coords[4]} {y_coords[4]},
        {x_coords[0]} {y_coords[4]},
        {x_coords[0]} {y_coords[0]}
    )'), 4326))    ;
    
    INSERT INTO {schema_name}.{table_name}
    (name, shape)
    VALUES ( 'Just inner polygon',
    ST_SetSrid(ST_MakePolygon( 'LINESTRING(
        {x_coords[1]} {y_coords[2]},
        {x_coords[2]} {y_coords[1]},
        {x_coords[3]} {y_coords[2]},
        {x_coords[2]} {y_coords[3]},
        {x_coords[1]} {y_coords[2]}  
    )'), 4326))    ;
    
    """)


def step_adjust_ref_raster_size():
    """
    Make sure the reference raster is big enough to cover all buildings
    """

    if check_clean_work_file(cfg, cfg.RASTER_EXPANDED_REF):
        return

    ref_raster = get_ref_raster()

    with rasterio.open(ref_raster) as r:
        raster_crs = r.crs

    x_min, y_min, x_max, y_max = get_projected_extent()

    raster_dataset = gdal.Open(str(ref_raster), gdal.GA_ReadOnly)
    raster_stats = RasterStats.from_gdal_dataset(raster_dataset)
    raster_dataset = None
    new_raster_stats = copy.deepcopy(raster_stats)

    # we want to make sure to have 1 extra row/col that is 0 around the buildings, this is for the contours
    assert raster_stats.pixel_width > 0
    assert raster_stats.pixel_height < 0
    x_min = x_min - raster_stats.pixel_width
    x_max = x_max + raster_stats.pixel_width
    y_min = y_min + raster_stats.pixel_height
    y_max = y_max - raster_stats.pixel_height

    if x_min < new_raster_stats.origin_x:
        # origin_x + pw * N = new_origin_x
        # new_origin_x < x_min
        # (x_min - origin_x) / pw
        additional_left_cols = math.floor(new_raster_stats.get_col_as_float(x_min))
        assert additional_left_cols < 0
        new_raster_stats.origin_x = new_raster_stats.get_x_for_col(additional_left_cols)

        log.debug(f"Additional columns to the left: {additional_left_cols}")

        # check now we have the largest new_origin_x that is smaller than x_min
        assert new_raster_stats.get_col_as_float(x_min - new_raster_stats.pixel_width) < 0

    assert new_raster_stats.get_col_as_float(x_min) > 0

    x_max = max(x_max, raster_stats.right_x())

    if x_max > new_raster_stats.right_x():
        # need more columns
        additional_right_cols = math.ceil(new_raster_stats.get_col_as_float(x_max)) - new_raster_stats.num_cols

        assert additional_right_cols > 0
        log.debug(f"Additional right columns: {additional_right_cols}")
        new_raster_stats.num_cols += additional_right_cols

        # make sure we grew the smallest possible
        # print(f"x_max {x_max} should be between {new_origin_x + (new_num_cols-1) * pixel_width} and {new_origin_x + new_num_cols * pixel_width}")
        assert x_max - new_raster_stats.pixel_width < new_raster_stats.right_x()

    # print(f"x_max: {x_max} raster right side: {new_origin_x + num_cols * pixel_width} num cols {num_cols} => {new_num_cols}")
    assert x_max <= new_raster_stats.right_x()

    if y_max > new_raster_stats.origin_y:
        # make origin_y larger (north)
        additional_top_rows = math.floor(new_raster_stats.get_row_as_float(y_max))
        assert additional_top_rows < 0
        new_raster_stats.origin_y = new_raster_stats.get_y_for_row(additional_top_rows)

        log.debug(f"Additional top rows: {additional_top_rows}")
        # check we have the smallest new_origin_y that is larger than y_max
        assert new_raster_stats.get_row_as_float(y_max - new_raster_stats.pixel_height) < 0

    # print(f"y_max: {y_max} origin_y: {origin_y} new_origin_y: {new_origin_y}")
    assert new_raster_stats.get_row_as_float(y_max) > 0

    y_min = min(y_min, raster_stats.bottom_y())

    if y_min < new_raster_stats.bottom_y():
        # need more rows
        additional_bottom_rows = math.ceil(new_raster_stats.get_row_as_float(y_min)) - new_raster_stats.num_rows
        log.debug(f"Additional bottom rows: {additional_bottom_rows}")

        assert additional_bottom_rows > 0
        new_raster_stats.num_rows += additional_bottom_rows

        # check we added the smallest # of rows possible
        assert new_raster_stats.get_row_as_float(y_min + new_raster_stats.pixel_height) >= new_raster_stats.num_rows

        # print(f"y_min: {y_min} {new_raster_stats.get_row_as_float(y_min)} num rows {new_raster_stats.num_rows}")
        assert new_raster_stats.get_row_as_float(y_min) <= new_raster_stats.num_rows
    assert y_min >= new_raster_stats.bottom_y()

    log.info(f"Adjusting/Expanding worldpop raster to admin extent to {cfg.RASTER_EXPANDED_REF}")

    new_dataset = rasterio.open(
        cfg.RASTER_EXPANDED_REF,
        'w',
        driver='GTiff',
        height=new_raster_stats.num_rows,
        width=new_raster_stats.num_cols,
        count=1,
        dtype=rasterio.uint8,
        crs=raster_crs,
        transform=
        affine.Affine.from_gdal(
            new_raster_stats.origin_x,
            new_raster_stats.pixel_width,
            0,

            new_raster_stats.origin_y,
            0,
            new_raster_stats.pixel_height),
    )

    new_dataset.close()


def step_grid_slice_settlements(conn):
    """
    Slices the buffered/unioned buildings by the reference raster grid.  This is to improve intersection performance
    because the extents will be smaller and the vertex accounts much lower.

    This will produce the raw output in working/<Country code>/grid_sliced_settlements/all.fgb

    """
    if check_clean_work_file(cfg, cfg.GRID_SLICED_ALL):
        return

    step_grid_slice_settlements_helper(cfg.UNIONED_NO_HOLES_PATH, cfg.GRID_SLICED_ALL, False)

def step_split_grid_slice_settlements(conn):
    """
    Splits the grid sliced settlements into chunks
    """

    if check_clean_work_dir(cfg, cfg.GRID_SLICED_DIR):
        return

    step_split_grid_slice_settlements_helper(cfg.GRID_SLICED_ALL, cfg.GRID_SLICED_DIR)


def step_grid_slice_settlements_final(conn):
    """
    Slices the buffered/unioned buildings by the reference raster grid.  This is to improve intersection performance
    because the extents will be smaller and the vertex accounts much lower.

    This will produce the raw output in working/<Country code>/grid_sliced_settlements/all_final.fgb

    """
    if check_clean_work_file(cfg, cfg.FINAL_GRID_SLICED_ALL):
        return

    step_grid_slice_settlements_helper(cfg.FINAL_SETTLEMENT_SHAPES_PATH, cfg.FINAL_GRID_SLICED_ALL, True)


def step_split_grid_slice_settlements_final(conn):
    """
    Splits the finalized shapes grid sliced settlements into chunks
    """

    if check_clean_work_dir(cfg, cfg.FINAL_GRID_SLICED_DIR):
        return

    step_split_grid_slice_settlements_helper(cfg.FINAL_GRID_SLICED_ALL, cfg.FINAL_GRID_SLICED_DIR)


def step_grid_slice_settlements_helper(settlement_fgb_path: Path, output_fgb_path: Path, add_id_field: bool):
    """
    Slices the buffered/unioned buildings by the reference raster grid.  This is to improve intersection performance
    because the extents will be smaller and the vertex accounts much lower.

    This will produce the raw output in working/<Country code>/grid_sliced_settlements/all.fgb

    """

    rust_cmd_parts = [
        'cargo',
        'run',
        '--bin',
        'fast_intersection',
        '--release',
        '--',
        f"--log-level \"{cfg.LOG_LEVEL}\"",
        'prepare',
        f'--in-ogr-conn "{settlement_fgb_path}"',
        f'--in-ogr-layer "{settlement_fgb_path.stem}"',
        f'--ref-raster "{cfg.RASTER_EXPANDED_REF}"',
        f'--output-path "{output_fgb_path}"',

    ]

    if add_id_field:
        rust_cmd_parts.append('--id-field orig_fid')

    rust_cmd = ' '.join(rust_cmd_parts)

    run_process_stream_output(rust_cmd, cwd="/rust", env_override={
        "RUST_BACKTRACE": "1"
    })

def step_split_grid_slice_settlements_helper(in_fgb: Path, out_slice_dir: Path):
    """
    Splits the settlements
    """


    rust_cmd_parts = [
        'cargo',
        'run',
        '--bin',
        'bldg_agg',
        '--release',
        '--',
        'fix-reproject-split',
        f"--snap-raster-path \"{cfg.RASTER_EXPANDED_REF}\"",
        f"--output-path \"{out_slice_dir}\"",
        f"-c \"{in_fgb}\"",
        f"-l \"{in_fgb.stem}\"",
        f"--chunk-rows={cfg.CHUNK_ROWS}",
        f"--chunk-cols={cfg.CHUNK_COLS}",
        f"-f grid_index",
        f"-f orig_fid",
    ]

    rust_cmd = ' '.join(rust_cmd_parts)

    log_path = cfg.LOG_PATH.parent / f"{in_fgb.stem}.txt"

    log_path.unlink(missing_ok=True)

    run_process_stream_output(rust_cmd, cwd="/rust",
                              env_override={
                                  "CPL_LOG": log_path
                              })

def step_import_settlement_slices_to_db(conn):
    """
    Imports the grid sliced current settlements to the database
    """

    import_settlements_to_be_helper(conn, cfg, cfg.SETTLEMENTS_PARENT_TABLE,
                                    cfg.SETTLEMENT_GRID_SLICES_SCHEMA_NAME,
                                    cfg.GRID_SLICED_DIR)


def step_include_raster_centers(conn):
    """
    Creates CSVs & binary files on which additional shapes need to be created
    """

    if check_clean_work_dir(cfg, cfg.RASTER_CENTER_CSV_DIR):
        return

    compile_building_agg()

    task_queue = queue.Queue()

    task_count = 0

    for centroid_fgb in cfg.CENTROIDS_BUILDINGS_PATH.iterdir():

        if ".fgb" != centroid_fgb.suffix:
            continue

        csv_out = cfg.RASTER_CENTER_CSV_DIR / centroid_fgb.with_suffix(".csv").name
        bin_out = csv_out.with_suffix(".bin")

        settlement_fgb = cfg.GRID_SLICED_DIR / centroid_fgb.name

        rust_cmd_parts = [
            './bldg_agg',
            'check-raster-center',
            f"--bldg-centroid-fgb \"{centroid_fgb}\"",
            f"--out-csv \"{csv_out}\"",
            f"--out-bin \"{bin_out}\"",
            f"--bldg-count-raster \"{cfg.RASTER_BUILDING_COUNT}\"",
            f"--settlement-fgb \"{settlement_fgb}\""
        ]


        rust_cmd = ' '.join(rust_cmd_parts)

        task_queue.put(partial(
            run_process_stream_output,
            rust_cmd, cwd="/rust/target/release", ))
        task_count += 1


    thread_utils.finish_threads_with_context(task_queue=task_queue,
                                             fn_context_create=None,
                                             max_num_processes=4, num_items_in_queue=task_count)


def step_create_corner_shapes(conn):
    """
    Creates fgbs for the corners
    """

    if check_clean_work_dir(cfg, cfg.RASTER_CENTER_SHAPE_DIR):
        return

    compile_building_agg()

    task_queue = queue.Queue()

    task_count = 0

    for bin_path in cfg.RASTER_CENTER_CSV_DIR.iterdir():

        if ".bin" != bin_path.suffix:
            continue

        out_fgb = cfg.RASTER_CENTER_SHAPE_DIR / bin_path.with_suffix(".fgb").name

        rust_cmd_parts = [
            './bldg_agg',
            'create-corners',
            f"--out-fgb \"{out_fgb}\"",
            f"--in-bin \"{bin_path}\"",
            f"--snap-raster \"{cfg.RASTER_BUILDING_COUNT}\"",
        ]

        rust_cmd = ' '.join(rust_cmd_parts)

        task_queue.put(partial(
            run_process_stream_output,
            rust_cmd, cwd="/rust/target/release", ))
        task_count += 1

    thread_utils.finish_threads_with_context(task_queue=task_queue,
                                             fn_context_create=None,
                                             max_num_processes=4, num_items_in_queue=task_count)



def step_union_center_shapes():
    """
    Union/Dissolves all the additional corners created
    to make sure the settlements intersect the reference raster
    centers
    """
    if check_clean_work_file(cfg, cfg.FINAL_SETTLEMENT_SHAPES_PATH):
        return

    # Ideally should check the raster projection of the ref raster is in fact 4326
    rust_cmd_parts = [
        'cargo',
        'run',
        '--bin',
        'union_shapes',
        '--release',
        '--',
        f"--out-ogr-conn \"{cfg.FINAL_SETTLEMENT_SHAPES_PATH}\"",
        f"--out-ogr-layer \"{cfg.FINAL_SETTLEMENT_SHAPES_PATH.stem}\"",
        f"-c \"{cfg.UNIONED_NO_HOLES_PATH}\"",
        f"-l \"{cfg.UNIONED_NO_HOLES_PATH.stem}\"",
        f"--log-level \"{cfg.LOG_LEVEL}\"",
        "--out-driver FlatGeoBuf"
    ]

    for fgb in cfg.RASTER_CENTER_SHAPE_DIR.iterdir():

        if fgb.suffix != ".fgb":
            continue

        if fgb.stat().st_size <= EMPTY_FGB_SIZE_WITH_INDEX:
            continue

        rust_cmd_parts.extend([
            f"-c \"{fgb}\"",
            f"-l \"{fgb.stem}\"",
        ])

    rust_cmd = ' '.join(rust_cmd_parts)

    run_process_stream_output(rust_cmd, cwd="/rust", env_override={
        # "PG_USE_COPY": "YES"
    })


def step_intersect_buildings(conn):
    """
    Go through each building file and intersect them with the settlements

    This is done to verify each building is inside a settlement
    """

    # debug
    # file_utils.remove_dir(cfg.INTERSECTED_BUILDING_SPLIT_BUILDING_PATH, cfg.WORKING_FOLDER)

    if check_clean_work_dir(cfg, cfg.INTERSECTED_BUILDING_SPLIT_BUILDING_PATH):
        return

    if cfg.CLEAN:
        file_utils.remove_dir(cfg.INTERSECTED_BUILDING_SPLIT_BUILDING_WORK_PATH, cfg.WORKING_FOLDER)

    # build once
    rust_cmd_parts = [
        'cargo',
        'build',
        '--bin',
        'fast_intersection',
        '--release',
    ]

    rust_cmd = ' '.join(rust_cmd_parts)

    run_process_stream_output(rust_cmd, cwd="/rust", )

    task_queue = queue.Queue()

    task_count = 0

    # First we are doing intersection prep, this serializes the buildings because we want to be able to load neighboring
    # chunks in case we have intersections in another chunk
    for split_fgb in cfg.SPLIT_BUILDING_PATH.iterdir():

        task_count += 1

        task_queue.put(partial(
            run_intersect_helper,
            split_fgb,
            True
        ))

    thread_utils.finish_threads_with_context(task_queue=task_queue,
                                             fn_context_create=None,
                                             max_num_processes=4, num_items_in_queue=task_count)

    # debugging
    # return

    task_queue = queue.Queue()

    task_count = 0

    for split_fgb in cfg.SPLIT_BUILDING_PATH.iterdir():

        task_count += 1

        task_queue.put(partial(
            run_intersect_helper,
            split_fgb,
            False
        ))

    thread_utils.finish_threads_with_context(task_queue=task_queue,
                                             fn_context_create=None,
                                             max_num_processes=4, num_items_in_queue=task_count)


def run_intersect_helper(split_fgb: Path, is_prep):
    output_path = cfg.INTERSECTED_BUILDING_SPLIT_BUILDING_PATH / split_fgb.name

    in_chunk_number = int(split_fgb.stem.replace("chunk_", ""))

    rust_cmd_parts = [
        './fast_intersection',
        'intersect',
        f"--in-path={split_fgb}",
        f"--in-chunk-num={in_chunk_number}",
        # f"--ref-raster \"{cfg.REF_RASTER_PATH}\"",
        f"--output-path \"{output_path}\"",
        f"--chunk-rows={cfg.CHUNK_ROWS}",
        f"--chunk-cols={cfg.CHUNK_COLS}",
        f"--common-work-path={cfg.INTERSECTED_BUILDING_SPLIT_BUILDING_WORK_PATH}",
        f"--int-chunk-dir={cfg.FINAL_GRID_SLICED_DIR}",
        f"--id-field=orig_fid",
        f"--out-field=set_orig_fid"
    ]

    if is_prep:
        rust_cmd_parts.append(f"--mode-int-prep")

    rust_cmd = ' '.join(rust_cmd_parts)

    run_process_stream_output(rust_cmd, cwd="/rust/target/release", )


def import_buildings_to_db(conn):
    """
    Imports buildings to database, with partitioning
    """

    if cfg.CLEAN:
        db_utils.drop_schema(conn, cfg.BLDG_SCHEMA_NAME)

        db_utils.drop_table(conn, cfg.SCHEMA_NAME, cfg.BULDING_PARENT_TABLE)

    if db_utils.table_exists(conn, cfg.SCHEMA_NAME, cfg.BULDING_PARENT_TABLE):
        log.info(f" {cfg.SCHEMA_NAME}.{cfg.BULDING_PARENT_TABLE} already exists")
        return

    db_utils.create_schema(conn, cfg.SCHEMA_NAME)
    db_utils.create_schema(conn, cfg.BLDG_SCHEMA_NAME)

    db_utils.run_sql(conn, f"""
    CREATE TABLE {cfg.SCHEMA_NAME}.{cfg.BULDING_PARENT_TABLE} (
        id serial primary key,
        chunk_number int,
        shape Geometry(MultiPolygon, 4326) NOT NULL,
        set_orig_fid int        
    )
    """)

    task_queue = queue.Queue()

    task_count = 0

    print(f"Checking {cfg.INTERSECTED_BUILDING_SPLIT_BUILDING_PATH} directory")
    for idx, fgb in enumerate(cfg.INTERSECTED_BUILDING_SPLIT_BUILDING_PATH.iterdir()):

        if idx % 100 == 0:
            print(f"Done with {idx} partitions")

        # empty fgbs will still have some space for the geosptial index, by inspection there are 628 bytes

        if fgb.stat().st_size <= EMPTY_FGB_SIZE_WITH_INDEX:
            continue

        chunk_number = int(fgb.stem.replace("chunk_", ""))
        partition_table_name = fgb.stem

        db_utils.run_sql(conn, f"""
        CREATE TABLE {cfg.BLDG_SCHEMA_NAME}.{partition_table_name} (
        CHECK (chunk_number = {chunk_number})
        ) INHERITS ({cfg.SCHEMA_NAME}.{cfg.BULDING_PARENT_TABLE})
        """)

        task_count += 1

        task_queue.put(partial(
            import_chunk,
            cfg,
            fgb, cfg.BLDG_SCHEMA_NAME, partition_table_name, chunk_number, ""))

    thread_utils.finish_database_threads(cfg=cfg, task_queue=task_queue,
                                         max_num_processes=4, num_items_in_queue=task_count)

def check_building_counts(conn):
    """
    Runs some sql queries to check building counts
    """

    # sql_check_bldg_count = f"""
    # SELECT count(*) FROM
    # {cfg.SCHEMA_NAME}.{cfg.BULDING_PARENT_TABLE};
    # """
    #
    # r = db_utils.get_results(conn, sql_check_bldg_count)
    #
    # db_bldg_count = r[0][0]

    sql_check_bldg_count = f"""
        SELECT sum(bldg_count) FROM 
        {cfg.SCHEMA_NAME}.{cfg.FINAL_SETTLEMENT_TABLE_NAME};
        """

    r = db_utils.get_results(conn, sql_check_bldg_count)

    db_sett_bldg_count = r[0][0]

    input_sources = get_building_inputs()

    total_buildings = 0
    for path, layer_name in input_sources:
        with fiona.open(path, layer=layer_name) as recs:
            feature_count = len(recs)
        total_buildings += feature_count

# This is too slow
# Buildings in database:          {db_bldg_count:,}
    log.info(f"""Building counts:

Building counts in settlements: {db_sett_bldg_count:,}
Buildings in inputs:            {total_buildings:,}
""")

    # sql that is useful to find settlements that don't match
    # the actual building intersect count
    sql_debug_find_mismatechs = """

    drop table tgo.bldg_g;

    create table tgo.bldg_g as
    select set_orig_fid, count(*) as bldg_count
    from
         tgo.building
    group by set_orig_fid;

    select ST_X(st_centroid(shape)) || ', ' || ST_Y(st_centroid(shape)), * from tgo.bldg_g g
    full outer join tgo.settlements s on s.orig_fid = g.set_orig_fid
    where coalesce(s.bldg_count, -1) != coalesce(g.bldg_count, -2)
    ;"""

def step_export_settlements_to_fgb(conn):
    """
    Exports settlement table to a fgb
    """

    if check_clean_work_file(cfg, cfg.FINAL_SETTLEMENTS_PATH):
        return

    cmd_parts = [
            'ogr2ogr',
            '--config PG_USE_COPY YES',
            "-f FlatGeobuf",
            "-progress",           
            "-nlt MULTIPOLYGON",
            f"-nln {cfg.FINAL_SETTLEMENTS_PATH.stem}",
            f"\"{cfg.FINAL_SETTLEMENTS_PATH}\"",
            f"\"{db_utils.get_ogr_connection_string(cfg)}\"",
            f"\"{cfg.SCHEMA_NAME}.{cfg.FINAL_SETTLEMENT_TABLE_NAME}\""
        ]

    ogr2ogr_cmd = ' '.join(cmd_parts)
    
    log.info(ogr2ogr_cmd)

    run_process_stream_output(ogr2ogr_cmd, cwd="/rust", )    
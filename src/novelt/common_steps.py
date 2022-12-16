import logging
import math
import queue
from functools import partial
from pathlib import Path

from novelt.lib import db_utils, thread_utils, file_utils
from novelt.lib.thread_utils import run_process_stream_output

log = logging.getLogger(__name__)

def grid_slice_helper(conn, cfg,
                      ref_raster: Path,
                      dir_name: Path, grid_slice_schema,
                      settlement_table_name: str):

    db_utils.create_schema(conn, grid_slice_schema)

    for level in range(0, cfg.MAX_SETTLEMENT_LEVEL+1):
        # debugging
        # if level != 2:
        #     continue

        min_max_rs = db_utils.get_results(conn, f"""
SELECT MIN(orig_fid), MAX(orig_fid)
FROM {cfg.SCHEMA_NAME}.{settlement_table_name}
WHERE level = {level}
        """)

        min_id = min_max_rs[0][0]
        max_id = min_max_rs[0][1]

        settlements = max_id - min_id + 1

        number_of_chunks = 1

        if level == 2:
            number_of_chunks = 10

        chunk_size = math.ceil(settlements / number_of_chunks)

        log.info(f"Id range for level {level} is {min_id} to {max_id}.  Number of settlements: {settlements}.  Chunk size: {chunk_size}")

        # inclusive range
        cur_min_id = min_id
        cur_max_id = min_id + chunk_size - 1
        cur_chunk = 1

        out_files = []

        while True:
            view_name = f"cur_level_{level}_{cur_chunk}"

            db_utils.run_sql(conn, f"""
        DROP VIEW IF EXISTS {grid_slice_schema}.{view_name};          
        CREATE VIEW {grid_slice_schema}.{view_name} 
        AS SELECT * FROM {cfg.SCHEMA_NAME}.{settlement_table_name}
        WHERE level = {level}
        AND orig_fid >= {cur_min_id}
        AND orig_fid <= {cur_max_id}
        --AND id = 70747
        --AND id = 77160
                        """)

            out_path = dir_name / f"level_{level}_{cur_chunk}.fgb"

            out_files.append(out_path)

            out_path.parent.mkdir(parents=True, exist_ok=True)

            rust_cmd_parts = [
                'cargo',
                'run',
                '--bin',
                'fast_intersection',
                '--release',
                '--',
                'prepare',
                f'--in-ogr-conn "{db_utils.get_ogr_connection_string(cfg)}"',
                f'--in-ogr-layer "{grid_slice_schema}.{view_name}"',
                f'--ref-raster "{ref_raster}"',
                f'--output-path "{out_path}"',
                f'--id-field orig_fid',
            ]

            rust_cmd = ' '.join(rust_cmd_parts)

            run_process_stream_output(rust_cmd, cwd="/rust", env_override={
                "RUST_BACKTRACE": "1"
            })

            cur_min_id += chunk_size
            cur_max_id += chunk_size
            cur_chunk += 1

            if cur_min_id > max_id:
                break

        chunks_output = dir_name / f"chunks_level_{level}"

        chunks_output.mkdir(parents=True, exist_ok=True)

        rust_cmd_parts = [
            'cargo',
            'run',
            '--bin',
            'bldg_agg',
            '--release',
            '--',
            'fix-reproject-split',
            f"--snap-raster-path \"{ref_raster}\"",
            f"--output-path \"{chunks_output}\"",
            f"--chunk-rows={cfg.CHUNK_ROWS}",
            f"--chunk-cols={cfg.CHUNK_COLS}",
            f"-f grid_index",
            f"-f orig_fid",
        ]

        for o in out_files:
            rust_cmd_parts.append(f"-c \"{o}\"",)
            rust_cmd_parts.append(f"-l \"{o.stem}\"", )

        rust_cmd = ' '.join(rust_cmd_parts)

        log_path = cfg.LOG_PATH.parent / f"chunks_level_{level}.txt"

        log_path.unlink(missing_ok=True)

        run_process_stream_output(rust_cmd, cwd="/rust",
                                  env_override={
                                      "CPL_LOG": log_path
                                  })

def import_settlements_to_be_helper(conn, cfg, parent_table_name: str,
                                    slice_schema_name: str,
                                    sliced_directory: Path
                                    )    :

    if cfg.CLEAN:
        db_utils.drop_schema(conn, slice_schema_name)

        db_utils.drop_table(conn, cfg.SCHEMA_NAME, parent_table_name, cascade=True)


    if db_utils.table_exists(conn, cfg.SCHEMA_NAME, parent_table_name):
        log.info(f" {cfg.SCHEMA_NAME}.{parent_table_name} already exists")
        return

    db_utils.create_schema(conn, cfg.SCHEMA_NAME)
    db_utils.create_schema(conn, slice_schema_name)

    db_utils.run_sql(conn, f"""
    CREATE TABLE {cfg.SCHEMA_NAME}.{parent_table_name} (
        orig_fid int,        
        shape Geometry(MultiPolygon, 4326) NOT NULL,
        grid_index int,    
        chunk_number int,
        level smallint
    )
    """)

    task_queue = queue.Queue()

    task_count = 0

    log.debug(f"Checking {sliced_directory} directory")
    for level_dir in sliced_directory.iterdir():

        if not level_dir.is_dir():
            continue

        level = int(level_dir.name.split("_")[2])

        for idx, fgb in enumerate(level_dir.iterdir()):

            if idx % 100 == 0:
                print(f"Done with {idx} partitions")

            # empty fgbs will still have some space for the geosptial index, by inspection there are 628 bytes
            if fgb.stat().st_size <= 628:
                continue

            chunk_number = int(fgb.stem.replace("chunk_", ""))
            partition_table_name = f"level_{level}_{fgb.stem}"

            db_utils.run_sql(conn, f"""
            CREATE TABLE {slice_schema_name}.{partition_table_name} (
            CHECK (chunk_number = {chunk_number})
            ) INHERITS ({cfg.SCHEMA_NAME}.{parent_table_name})
            """)

            task_count += 1

            task_queue.put(partial(
                import_chunk,
                cfg,
                fgb, slice_schema_name, partition_table_name,
                chunk_number,
                f" , level = {level}"
            ))

    thread_utils.finish_database_threads(cfg=cfg, task_queue=task_queue,
                                             max_num_processes=4, num_items_in_queue=task_count)


def import_chunk(cfg, from_file: Path, to_schema: str, to_table: str, chunk_num: int, extra_sql, connection):
    rust_cmd_parts = [
        'ogr2ogr',
        '--config PG_USE_COPY YES',
        "-f PostgreSQL",
        #"-progress",
        f"\"{db_utils.get_ogr_connection_string(cfg)}\"",
        f'-nln {to_schema}.{to_table}',
        f"\"{from_file}\"",
    ]

    rust_cmd = ' '.join(rust_cmd_parts)

    run_process_stream_output(rust_cmd, cwd="/rust", )

    db_utils.run_sql(connection, f"""
    UPDATE {to_schema}.{to_table}
    SET chunk_number = {chunk_num}
    {extra_sql}
    """)

    db_utils.create_index(connection, to_schema, to_table, "shape", True)


def check_clean_work_dir(cfg, folder_path: Path):
    if cfg.CLEAN:
        file_utils.remove_dir(folder_path, cfg.WORKING_FOLDER)

    if folder_path.exists():
        log.info(f"{folder_path} exists, no need to perform step")
        return True

    folder_path.mkdir(parents=True, exist_ok=True)

    return False

def check_clean_work_file(cfg, file_path: Path):
    if cfg.CLEAN:
        file_utils.remove_file(file_path)
        file_utils.remove_dir(file_path, cfg.WORKING_FOLDER)

    if file_path.exists():
        log.info(f"{file_path} exists, no need to perform step...")
        return True

    file_path.parent.mkdir(parents=True, exist_ok=True)

    return False

def compile_building_agg():
    rust_cmd_parts = [
        'cargo',
        'build',
        '--bin',
        'bldg_agg',
        '--release',
    ]

    rust_cmd = ' '.join(rust_cmd_parts)
    run_process_stream_output(rust_cmd, cwd="/rust")
# coding=utf-8
import logging

import fiona
import psycopg2
import psycopg2.extras
from psycopg2.extensions import ISOLATION_LEVEL_AUTOCOMMIT
from psycopg2.sql import SQL, Literal

from novelt.lib.db_utils import get_results, create_db_connection
from novelt.lib.thread_utils import run_command

log = logging.getLogger(__name__)
trace_log = logging.getLogger(__name__ + "_trace")

trace_log.setLevel(logging.CRITICAL)
fiona.log.setLevel(logging.ERROR)


def launch_fast_zonalstats(
        feature_tif_path,
        pop_raster_tif_path,
        zs_csv_path):
    """

    :param feature_tif_path:
    :param pop_raster_tif_path:
    :param zs_csv_path: The path to store the aggregrated figures
    :return:
    """

    rust_cmd_parts = [
        'cargo',
        'run',
        '--bin',
        'zonal_stats',
        '--release',
        '--',
        f"\"{feature_tif_path}\"",
        f"\"{pop_raster_tif_path}\"",
        f"\"{zs_csv_path}\"",
    ]

    my_env = {}
    fast_zonal_stats_command = ' '.join(rust_cmd_parts)

    log.debug("Running fast zonal stats: {}".format(fast_zonal_stats_command))

    run_command(fast_zonal_stats_command, cwd="/rust_pop_util")






def drop_schema(conn, schema_name, cascade=False):
    cur = conn.cursor()

    sql = """
		DROP SCHEMA IF EXISTS %s 
		""" % (schema_name,)

    if cascade:
        sql = """
		DROP SCHEMA IF EXISTS %s CASCADE  
		""" % (schema_name,)

    log.debug(sql)
    cur.execute(sql)

    conn.commit()







def get_column_names(conn, schema_name, table_name):
    recs = get_results(conn, rf"""
SELECT col1.column_name 
	FROM information_schema.columns col1
		
	WHERE col1.table_name     ilike '{table_name}'	
		AND col1.table_schema ilike '{schema_name}'
	
""")

    columns = [r[0] for r in recs]

    return columns


def get_shared_column_names(conn, schema1, schema2, table1, table2, columns_to_ignore):
    if len(columns_to_ignore) == 0:
        columns_to_ignore = ['filler_invalid_column_ignore']

    columns_set = SQL(', ').join([Literal(c) for c in columns_to_ignore]).as_string(conn)

    recs = get_results(
        conn,
        sql=rf"""
SELECT col1.column_name 
	FROM information_schema.columns col1
		INNER JOIN information_schema.columns col2 
			ON col1.column_name = col2.column_name 
		
	WHERE col1.table_name     ilike '{table1}'	
		AND col1.table_schema ilike '{schema1}'
		AND col2.table_name   ilike '{table2}'
		AND col2.table_schema ilike '{schema2}'
		AND col1.column_name NOT IN ({columns_set})
""")

    columns = [r[0] for r in recs]

    return columns







def create_database(cfg, drop_if_exists=False,
                    add_postgis=True):
    with psycopg2.connect(
            database='postgres',
            host=cfg.POSTGRESQL_HOST,
            port=cfg.POSTGRESQL_PORT,
            user=cfg.POSTGRESQL_USERNAME,
            password=cfg.POSTGRESQL_PASSWORD
    ) as conn:

        conn.autocommit = True
        conn.set_isolation_level(ISOLATION_LEVEL_AUTOCOMMIT)
        cur = conn.cursor()

        # First check if exists
        sql = """
		SELECT 1 AS result FROM pg_database
		WHERE datname=%s
		"""

        dbName = cfg.POSTGRESQL_DATABASE

        cur.execute(sql, (dbName,))

        database_exists = cur.fetchone() is not None

        if database_exists:
            if drop_if_exists:
                sql = rf"""
				SELECT pg_terminate_backend(pg_stat_activity.pid) 
				FROM pg_stat_activity 
				WHERE pg_stat_activity.datname = '{dbName}'  AND pid <> pg_backend_pid();
				"""
                cur.execute(sql)

                sql = fr"""
				DROP DATABASE {dbName} ;
				"""

                cur.execute(sql)
                conn.commit()
            else:
                log.debug("Database already exists")
                return True

        db_owner = cfg.POSTGRESQL_USERNAME

        table_space = cfg.POSTGRESQL_TABLESPACE or 'pg_default'

        sql = """
		CREATE DATABASE %s OWNER %s TABLESPACE %s ENCODING 'UTF8' TEMPLATE template0
		""" % (dbName, db_owner, table_space)

        log.debug(sql)
        print(sql,'sql')
        conn.commit()
        cur.execute(sql)
        conn.commit()

    if add_postgis:
        with create_db_connection(cfg) as conn:
            add_postgis_extension(conn)


def add_postgis_extension(conn):
    cur = conn.cursor()

    sql = """
	CREATE EXTENSION IF NOT EXISTS postgis
	"""

    log.debug(sql)
    cur.execute(sql)
    conn.commit()









def quote_command_line_arg(arg):
    replace_with_double_quotes = arg.replace('"', '""')
    return f'"{replace_with_double_quotes}"'

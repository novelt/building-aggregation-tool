# coding=utf-8

from novelt.lib import geo_db_utils, db_utils
import pytest
from novelt.config.test_config import Config as cfg
import os
import queue

from novelt.lib.db_utils import table_exists, get_results
from novelt.lib.test_utils import create_point_table, TestConstants, conn


def test_create_point_table_with_sql(conn):
    create_point_table(conn)

    r = get_results(
        conn,
        f"select count(*) from {TestConstants.TEST_SCHEMA}.{TestConstants.TABLE_NAME_POINTS}")

    assert r[0][0] == 100


def test_import_export_shapefile(conn):
    create_point_table(conn)

    shapefile_path = cfg.TEMP_DIR / "another sub dir" / "points.shp"

    geo_db_utils.import_geometry_from_postgis_to_shapefile(cfg, cfg.LOCAL_PREFIX,
                                                           shapefile_path=shapefile_path,
                                                           schema_name=TestConstants.TEST_SCHEMA,
                                                           postgis_table_name=TestConstants.TABLE_NAME_POINTS)

    assert not table_exists(conn, schema_name=TestConstants.TEST_SCHEMA, table_name="test_points_imported")

    geo_db_utils.import_geometry_from_shapefile_to_postgis(
        cfg, cfg.LOCAL_PREFIX, shapefile_path, schema_name=TestConstants.TEST_SCHEMA,
        postgis_table_name="test_points_imported"
    )

    assert table_exists(conn, schema_name=TestConstants.TEST_SCHEMA, table_name="test_points_imported")

    assert geo_db_utils.get_row_count(conn, schema_name=TestConstants.TEST_SCHEMA, table_name="test_points_imported") == 100

    # now use shp2pgsql

    assert geo_db_utils.get_row_count(conn, schema_name=TestConstants.TEST_SCHEMA,
                                      table_name="test2") is None

    shp_file_sql = geo_db_utils.import_shapefile(
        cfg,
        shapefile_path,
        TestConstants.TEST_SCHEMA,
        table_name="test2"
    )

    db_utils.run_sql_file(cfg, db_params_prefix=cfg.LOCAL_PREFIX,
                          file_path=shp_file_sql)

    assert geo_db_utils.get_row_count(conn, schema_name=TestConstants.TEST_SCHEMA,
                                      table_name="test2") == 100

    # def test_get_fgdb_row_count(self):
    #
    #     test_fgdb_path = os.path.join(os.path.dirname(__file__), '..', 'unit_test_data',
    #                                   'simple_work_fgdb',
    #                                   'work.gdb')
    #     row_count = dbLib.get_row_count_with_ogrinfo(cfg=cfg, src_path = test_fgdb_path,
    #                                                  table_name = cfg.FC_FE_HamletAreas)
    #
    #
    #     self.assertEqual(10, row_count)

    # def test_get_cell_size(self):
    #
    #     test_raster_path = os.path.join(os.path.dirname(__file__), '..', 'unit_test_data',
    #                                   'ref_pop_raster',
    #                                   'popGridRefRaster.tif')
    #     cell_dim1, cell_dim2 = dbLib.get_raster_cell_size(cfg, raster_path = test_raster_path, return_single_value = False)
    #
    #     self.assertEqual(0.000451, cell_dim1)
    #     self.assertEqual(-0.000451, cell_dim2)
    #
    # def test_get_sql_to_run(self):
    #
    #     sql = "SELECT {{BOB}} FROM {{FC_SSABUFFER}}"
    #
    #     sql_replaced = dbLib.replace_sql_tokens(cfg, sql = sql, BOB="Sam")
    #
    #     self.assertEqual("SELECT Sam FROM VTS_GPRefLyrSSABuffers", sql_replaced)
    #
    # def test_replace_sql_tokens(self):
    #
    #     class TestBaseCfg(object):
    #
    #         BASE_PROP = '3'
    #         BASE_PROP_2 = 3.14
    #
    #     class TestCfg(TestBaseCfg):
    #
    #         PROP_3 = 45
    #
    #     sql = dbLib.replace_sql_tokens(TestCfg, 'SELECT {{BASE_PROP_2}} {{PROP_3}}')
    #
    #     self.assertEqual('SELECT 3.14 45', sql)
    #
    # def test_get_indexes(self):
    #
    #     UNIT_TEST_SCHEMA_NAME = 'unit_tests'
    #     with dbLib.create_db_connection_prefix(cfg, db_params_prefix = cfg.LOCAL_PREFIX) as conn:
    #         dbLib.drop_schema(conn, schema_name = UNIT_TEST_SCHEMA_NAME, cascade = True)
    #
    #         cur = conn.cursor()
    #
    #         dbLib.create_schema(conn, schema_name = UNIT_TEST_SCHEMA_NAME)
    #
    #         cur.execute("""
    #         CREATE TABLE {schema_name}.TEST_PARENT(
    #             id serial PRIMARY KEY ,
    #             code varchar
    #         );
    #
    #         CREATE TABLE {schema_name}.test_child(
    #             id serial PRIMARY KEY ,
    #             code varchar,
    #             parent_code varchar,
    #             test_c varchar UNIQUE,
    #             test_i int UNIQUE,
    #             test_d DOUBLE PRECISION UNIQUE
    #         );
    #         """.format(schema_name=UNIT_TEST_SCHEMA_NAME))
    #
    #         indexes = dbLib.get_indexes(conn, schema_name = UNIT_TEST_SCHEMA_NAME,
    #                           table_name = 'test_child')
    #
    #         print(indexes)
    #         self.assertEqual(4, len(indexes), msg="3 uniques + the primary")
    #
    #         indexes = dbLib.get_indexes(conn, schema_name = UNIT_TEST_SCHEMA_NAME,
    #                                     table_name = 'test_child',
    #                                     column_name = 'test_c')
    #
    #         self.assertEqual(1, len(indexes), msg = "3 uniques + the primary")
    #         self.assertEqual('test_c', indexes[0].column_name)
    #
    # def test_add_foreign_keys(self):
    #     UNIT_TEST_SCHEMA_NAME = 'unit_tests'
    #     with dbLib.create_db_connection_prefix(cfg, db_params_prefix = cfg.LOCAL_PREFIX) as conn:
    #         dbLib.drop_schema(conn, schema_name = UNIT_TEST_SCHEMA_NAME, cascade = True)
    #
    #         cur = conn.cursor()
    #
    #         dbLib.create_schema(conn, schema_name = UNIT_TEST_SCHEMA_NAME)
    #
    #         cur.execute("""
    #         CREATE TABLE {schema_name}.TEST_PARENT(
    #             id serial PRIMARY KEY ,
    #             code varchar
    #         );
    #
    #         CREATE TABLE {schema_name}.test_child1(
    #             id serial PRIMARY KEY ,
    #             code varchar,
    #             parent_code varchar,
    #             test_c varchar UNIQUE,
    #             test_i int UNIQUE,
    #             test_d DOUBLE PRECISION UNIQUE
    #         );
    #
    #         CREATE TABLE {schema_name}.test_child2(
    #             id serial PRIMARY KEY ,
    #             code varchar,
    #             parent_code varchar,
    #             test_c varchar UNIQUE,
    #             test_i int UNIQUE,
    #             test_d DOUBLE PRECISION UNIQUE
    #         );
    #
    #         CREATE TABLE {schema_name}.test_child3(
    #             id serial PRIMARY KEY ,
    #             code varchar,
    #             parent_code varchar,
    #             test_c varchar UNIQUE,
    #             test_i int UNIQUE,
    #             test_d DOUBLE PRECISION UNIQUE
    #         );
    #         """.format(schema_name = UNIT_TEST_SCHEMA_NAME))
    #
    #         for i in range(1, 4):
    #             dbLib.add_foreign_key(conn, schema_name = UNIT_TEST_SCHEMA_NAME,
    #                               table_name = 'test_child' + str(i),
    #                                   column_name = 'parent_code',
    #                                   ref_schema_name = UNIT_TEST_SCHEMA_NAME,
    #                                   ref_table_name =  'test_parent',
    #                                   ref_column_name = 'code')
    #
    #         i_info_list = dbLib.get_indexes(conn, schema_name = UNIT_TEST_SCHEMA_NAME,
    #                           table_name = 'test_parent',
    #                           column_name = 'code',
    #                           con_type = 'f')
    #
    #         self.assertEqual(3, len(i_info_list))
    #
    # def test_run_multithreaded_query(self):
    #     UNIT_TEST_SCHEMA_NAME = 'unit_tests'
    #     with dbLib.create_db_connection_prefix(cfg, db_params_prefix = cfg.LOCAL_PREFIX) as conn:
    #         dbLib.drop_schema(conn, schema_name = UNIT_TEST_SCHEMA_NAME, cascade = True)
    #
    #         cur = conn.cursor()
    #
    #         dbLib.create_schema(conn, schema_name = UNIT_TEST_SCHEMA_NAME)
    #
    #         cur.execute(SQL("""
    #                 CREATE TABLE {schema_name}.TEST_PARENT(
    #                     id serial PRIMARY KEY ,
    #                     code varchar,
    #                     an_int int,
    #                     a_double double precision
    #                 );""").format(schema_name=Identifier(UNIT_TEST_SCHEMA_NAME)))
    #
    #     task_queue = Queue.Queue()
    #
    #     task_queue.put(threadLib.DatabaseQueryTask(
    #         base_cfg = cfg,
    #
    #         sql = SQL("""
    #         INSERT INTO {schema_name}.test_parent
    #         ( {0} )
    #         VALUES ( %(code)s, %(an_int)s, %(a_double)s)
    #         """).format(
    #             SQL(",").join( [Identifier(s) for s in ["code", "an_int", "a_double"]]),
    #             schema_name=Identifier(UNIT_TEST_SCHEMA_NAME)),
    #
    #             code= '3',
    #             an_int= 4,
    #             a_double= 3.14
    #
    #     ))
    #
    #     threadLib.finish_database_threads(cfg, task_queue, cfg.WORKDB_PREFIX)
    #
    #
    # def test_temp(self):
    #     with dbLib.create_db_connection_prefix(cfg, db_params_prefix = cfg.LOCAL_PREFIX) as conn:
    #         dbLib.add_foreign_key(
    #             conn = conn,
    #             schema_name = cfg.SCHEMA_FEATURE_CLASS,
    #             table_name = cfg.FC_Boundary_VaccWards,
    #             column_name = 'lgacode',
    #             ref_schema_name = cfg.SCHEMA_FEATURE_CLASS,
    #             ref_table_name = cfg.FC_Boundary_VaccLGAs,
    #             ref_column_name = 'lgacode',
    #             drop_existing_constraints = True
    #         )

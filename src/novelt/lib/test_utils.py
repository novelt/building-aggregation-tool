import logging
import sys

import novelt.lib.geo_db_utils as geo_db_utils
import pytest
from novelt.config.test_config import Config as cfg
import os
import queue

from novelt.lib import db_utils
from novelt.lib.db_utils import run_sql, table_exists, create_db_connection

log = logging.getLogger(__name__)


class TestConstants(object):
    TEST_SCHEMA = "test"
    TABLE_NAME_POINTS = "test_points"
    TABLE_NAME_POLYGONS = "test_polygons"


@pytest.fixture(scope="module")
def conn():
    geo_db_utils.create_database(
        cfg=cfg,
        db_params_prefix=cfg.LOCAL_PREFIX,
        drop_if_exists=True, add_postgis=True)

    conn = create_db_connection(cfg, cfg.LOCAL_PREFIX)

    db_utils.create_schema(conn, TestConstants.TEST_SCHEMA, comment="Schema with test data for unit tests")

    return conn


@pytest.fixture(scope="module")
def test_logging():
    root = logging.getLogger()
    root.setLevel(logging.DEBUG)

    handler = logging.StreamHandler(sys.stdout)
    handler.setLevel(logging.DEBUG)
    formatter = logging.Formatter('%(asctime)s - %(name)s - %(levelname)s - %(message)s')
    handler.setFormatter(formatter)
    root.addHandler(handler)


def create_point_table(conn):

    if table_exists(conn, schema_name=TestConstants.TEST_SCHEMA,
                                 table_name=TestConstants.TABLE_NAME_POINTS):
        return

    run_sql(conn, rf"""
    CREATE TABLE {TestConstants.TEST_SCHEMA}.{TestConstants.TABLE_NAME_POINTS} (
    id serial NOT NULL,        
    shape Geometry(Point, 4326)
    )
        """)

    for i in range(0, 10):
        for j in range(0, 10):
            run_sql(conn, rf"""
    INSERT INTO {TestConstants.TEST_SCHEMA}.{TestConstants.TABLE_NAME_POINTS} (shape)
    VALUES (ST_SetSRID(ST_MakePoint({i}, {j}), 4326));                    
                """)
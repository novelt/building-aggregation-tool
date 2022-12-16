import logging
from typing import Union

import psycopg2
from psycopg2._psycopg import Error as PsError, cursor
from psycopg2.extras import DictCursor
from psycopg2.sql import SQL, Identifier, Literal, Composed

log = logging.getLogger(__name__)
trace_log = logging.getLogger(__name__ + "_trace")

trace_log.setLevel(logging.CRITICAL)




def create_db_connection(cfg):
    """
    :param cfg: class containing db connection attributes
    :return: A psycop2 connection
    """



    """

    """
    return psycopg2.connect(
        database=cfg.POSTGRESQL_DATABASE,
        host=cfg.POSTGRESQL_HOST,
        port=cfg.POSTGRESQL_PORT,
        user=cfg.POSTGRESQL_USERNAME,
        password=cfg.POSTGRESQL_PASSWORD,
        connect_timeout=120,
        #tcp_user_timeout=60*60*1000,
        keepalives_idle=30,
        keepalives_interval=3,
        options='-c statement_timeout=3600000'
    )




def sql_as_string(sql: Union[str, Composed], cur):
    if hasattr(sql, "as_string"):
        return sql.as_string(cur)
    else:
        return sql


def execute_sql(conn, sql: Union[str, Composed], query_execute_args=None, suppress_logging=False,
                use_dict_cursor=False) -> cursor:
    """
    Runs SQL

    :param conn:
    :param sql: Either a string or SQL built with psycopg2's SQL class http://initd.org/psycopg/docs/sql.html
    :param query_execute_args: passed directly to execute
    :param suppress_logging:
    :param use_dict_cursor If true, the rows returned can be accessed by column name
    :return:
    """
    cursor_args = {}

    if use_dict_cursor:
        cursor_args['cursor_factory'] = DictCursor

    cur = conn.cursor(**cursor_args)

    if not suppress_logging:
        trace_log.debug(f"Running sql:\n{sql_as_string(sql, cur)}")

    try:
        cur.execute(sql, query_execute_args)

        return cur
    except PsError as ex:

        log.warning(f"Sql:\n{ex}\n\nCaught Programming SQL error: {sql_as_string(sql, cur)}")
        conn.rollback()

        raise


def run_sql(conn, sql: Union[str, Composed], query_execute_args=None, suppress_logging=False) -> int:
    """
    Runs SQL with a commit

    :param conn:
    :param sql: Either a string or SQL built with psycopg2's SQL class http://initd.org/psycopg/docs/sql.html
    :param query_execute_args: passed directly to execute
    :param suppress_logging:
    :return:
    """

    cur = execute_sql(conn, sql, query_execute_args, suppress_logging)

    rowcount = cur.rowcount

    if rowcount > 1:
        trace_log.debug("Number of rows affected: {}".format(rowcount))

    conn.commit()
    return rowcount


def table_exists(conn, schema_name: str, table_name: str):
    cur = conn.cursor()

    assert schema_name is not None

    cur.execute(SQL("""SELECT 1 FROM information_schema.tables WHERE table_name = {} AND table_schema = {}""")
                .format(Literal(table_name), Literal(schema_name)))

    result_count = len(cur.fetchall())

    return result_count > 0


def get_results(conn, sql: Union[str, Composed], msg_if_no_results: Union[None, bool] = None):
    cur = execute_sql(conn, sql)

    recs = cur.fetchall()

    if recs is None or len(recs) < 1:
        if msg_if_no_results:
            log.warning(f"Problem with sql: {sql_as_string(sql, cur)}")

        return None

    return recs


def get_single_value(conn, sql: Union[str, Composed], msg_if_no_results: Union[None, bool] = None):
    cur = execute_sql(conn, sql)

    rec = cur.fetchone()

    if rec is None or len(rec) < 1:
        if msg_if_no_results:
            log.warning(f"Problem with sql: {sql_as_string(sql, cur)}")

        return None

    return rec[0]


def create_schema(conn, schema_name: str, comment: str = None):
    cur = conn.cursor()

    cur.execute(
        SQL("""
        CREATE SCHEMA IF NOT EXISTS {schema_name};

        """).format(
            schema_name=Identifier(schema_name)
        ))

    if comment:
        cur.execute(SQL("""
        COMMENT ON SCHEMA {} IS {}
        """).format(Identifier(schema_name), Literal(comment)))

    conn.commit()




def drop_table(conn, schema_name, table_name, cascade=False):
    sql = SQL("""
		DROP TABLE IF EXISTS {}.{}
		""").format(Identifier(schema_name), Identifier(table_name))

    if cascade:
        sql += SQL(" CASCADE")

    run_sql(conn, sql)


def get_row_count(conn, schema_name, table_name, return_value_on_table_not_exist=None):
    if not table_exists(conn, schema_name, table_name):
        return return_value_on_table_not_exist

    sql = SQL("""
	SELECT count(*) FROM {}.{}
	""").format(Identifier(schema_name), Identifier(table_name))

    return get_single_value(conn, sql)





def get_columns(conn, schema_name, table_name):

    sql = SQL("""
SELECT column_name 
FROM information_schema.columns
WHERE table_schema = {}
        AND table_name = {}
    
""").format(Literal(schema_name), Literal(table_name))

    recs = get_results(conn, sql)

    if recs is None:
        return []

    columns = [r[0] for r in recs]

    return columns


def create_index(conn, schema_name, table_name, column_name, is_geom, check_if_index_exists=True):
    if check_if_index_exists:
        index_records = get_indexes(
            conn, schema_name=schema_name,
            table_name=table_name,
            column_name=column_name
        )
        trace_log.debug("Found %i existing indexes" % (len(index_records),))

        if index_records:
            # log.debug("Existing index %s" % (index_name,))
            # log.debug("Index already exists, called %s" % (index_name,))
            return

    index_name = "idx_%s_%s" % (table_name, column_name)
    index_name = index_name.lower()

    index_type = 'BTREE'
    if is_geom:
        index_type = "GIST"

    sql = SQL("CREATE INDEX {} ON {}.{} USING {} ({}) ").format(
        Identifier(index_name),
        Identifier(schema_name),
        Identifier(table_name),
        SQL(index_type),
        Identifier(column_name)
    )

    run_sql(conn, sql, suppress_logging=True)


def get_indexes(conn, schema_name, table_name, column_name=None, con_type=None):
    """
    dictionary with keys
    column name,
    schema name,
    table name,
    index name,
    constraint name (can be null)
    contype (p primary key, f foreign key on another table, u unique constraint
    """

    sql = SQL("""
	SELECT 
	a.attname AS column_name, 
	n.nspname AS schema_name, 
	t.relname AS table_name, 
	i.relname AS index_name, 
	con.conname AS constraint_name, 
	con.contype  
FROM pg_index ix LEFT JOIN pg_constraint con ON ix.indexrelid = con.conindid
LEFT JOIN pg_class i ON i.oid = ix.indexrelid
LEFT JOIN pg_class t ON t.oid = indrelid
LEFT JOIN pg_namespace n ON n.oid = t.relnamespace
LEFT JOIN pg_attribute a ON a.attrelid = t.oid AND a.attnum = ANY(ix.indkey)
WHERE n.nspname ILIKE COALESCE({}, n.nspname)
	AND t.relname ILIKE {}
	AND a.attname ILIKE COALESCE({}, a.attname)
	AND con.contype IS NOT DISTINCT FROM COALESCE({}, con.contype)

	""").format(
        Literal(schema_name.lower()),
        Literal(table_name.lower()),
        Literal(column_name.lower() if column_name else None),
        Literal(con_type)
    )

    cur = execute_sql(conn, sql, suppress_logging=True, use_dict_cursor=True)
    ret = cur.fetchall()
    trace_log.debug(f"Checked indexes with\n{sql_as_string(sql, cur)}.  Returning {len(ret)} results")

    return ret




def import_zonalstats_csv_helper(
        conn, csv_path,
        csv_schema_name, csv_table_name,
        extra_comment=None):
    """
    Imports the CSV containing the zonal stats as a CSV
    """

    if extra_comment is None:
        extra_comment = ""

    sql = rf"""
    DROP TABLE IF EXISTS {csv_schema_name}.{csv_table_name};

    CREATE TABLE {csv_schema_name}.{csv_table_name}
    (
        feature_id INT PRIMARY KEY,
        square_count INT,
        square_sum DOUBLE PRECISION    
    );

    COMMENT ON TABLE {csv_schema_name}.{csv_table_name} IS 'raw data from the csv file. {extra_comment}';
    COMMENT ON COLUMN {csv_schema_name}.{csv_table_name}.feature_id IS 'matches id column from feature class'; 
    COMMENT ON COLUMN {csv_schema_name}.{csv_table_name}.square_count IS 'How many raster squares were counted (even if they had no data)';
    COMMENT ON COLUMN {csv_schema_name}.{csv_table_name}.square_sum IS 'Sum of intersecting raster squares';

    """

    run_sql(conn, sql)

    copy_sql = f"""
COPY {csv_schema_name}.{csv_table_name} (feature_id, square_count, square_sum)
FROM STDIN WITH CSV DELIMITER ','
    """

    with open(csv_path, 'r') as csv_file:

        cur = conn.cursor()
        cur.copy_expert(copy_sql, csv_file)
        conn.commit()



def get_sql_alchemy_connection_string(cfg):
    return r'postgresql://%s:%s@%s:%s/%s' % (

        cfg.POSTGRESQL_USERNAME,
        cfg.POSTGRESQL_PASSWORD,
        cfg.POSTGRESQL_HOST,
        cfg.POSTGRESQL_PORT,
        cfg.POSTGRESQL_DATABASE,
    )


def get_ogr_connection_string(cfg):
    return r'PG: host=%s dbname=%s port=%s user=%s password=%s' % (
        cfg.POSTGRESQL_HOST,
        cfg.POSTGRESQL_DATABASE,
        cfg.POSTGRESQL_PORT,
        cfg.POSTGRESQL_USERNAME,
        cfg.POSTGRESQL_PASSWORD,
    )



def drop_existing_connections(conn, db_name):
    whereSql = """
    WHERE pg_stat_activity.datname = LOWER(%s)  AND pid <> pg_backend_pid()
    and query not like 'autovacuum%%';
    """

    sql = """
            SELECT pg_cancel_backend(pg_stat_activity.pid) 
            FROM pg_stat_activity 

            """ + whereSql

    cur = conn.cursor()
    cur.execute(sql, (db_name,))

    row_count = cur.rowcount
    log.info("Canceled connections: %i" % row_count)
    conn.commit()

    sql = """
            SELECT pg_terminate_backend(pg_stat_activity.pid) 
            FROM pg_stat_activity 
            """ + whereSql

    cur.execute(sql, (db_name,))

    row_count = cur.rowcount
    log.info("Terminated connections: %i" % row_count)

    conn.commit()

    sql = """
            SELECT pid, query, now()-query_start AS query_duration
            FROM pg_stat_activity 

        """ + whereSql
    cur.execute(sql, (db_name,))
    recs = cur.fetchall()

    if len(recs) > 0:
        for r in recs:
            log.warning("Unkilled session. PID: %i Duration: %s Query: %s" % (r[0], r[2], r[1][0:500]))
            # This is pretty extreme, causes instability
            # os.system("TaskKill /F /PID %i" % (r[0]))
        raise Exception("Unable to kill all existing database processes.  " +
                        "It may be necessary to restart the service postgresql")
    else:
        log.info(f"All existing sessions dropped from {db_name}")


def drop_schema(conn, schema_name):
    log.info(f"Dropping schema {schema_name}")

    # Cleanup any existing tables
    table_names = get_results(conn, f"""
    SELECT table_name FROM information_schema.tables
    where table_schema = '{schema_name}'
    and table_type = 'BASE TABLE'
    """)
    if table_names is None:
        table_names = []
    table_names = [t[0] for t in table_names]

    # do this as cascade deleting the schema can be too long
    for table_name in table_names:
        drop_table(conn, schema_name, table_name, cascade=True)

    run_sql(conn, f"""
    DROP SCHEMA IF EXISTS {schema_name} CASCADE
    """)
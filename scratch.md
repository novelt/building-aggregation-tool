
## QGIS starting on WSL2

https://www.qgis.org/en/site/forusers/alldownloads.html#debian-ubuntu

"C:\Program Files\VcXsrv\vcxsrv.exe" :0 -ac -terminate -lesspointer -multiwindow -clipboard -wgl -dpi auto -xkblayout us -xkbvariant dvorak-intl -xkbmodel pc105

Note this is in WSL2, and not a docker container

export DISPLAY=$(cat /etc/resolv.conf | grep nameserver | awk '{print $2}'):0

qgis &


## Rsync commands

mkdir -p /home/eric/git/bldg-agg/modules/BLDG_AGG
rsync --progress --verbose -r /home/eric/git/pop_model/modules/BLDG_AGG/ /home/eric/git/bldg-agg/modules/BLDG_AGG

mkdir -p /home/eric/git/bldg-agg/modules/BLDG_CHECK
rsync --progress --verbose -r /home/eric/git/pop_model/country_specific/BLDG_CHECK/src/ /home/eric/git/bldg-agg/modules/BLDG_CHECK/src

rsync --progress --verbose -r /home/eric/git/pop_model/rust/ /home/eric/git/bldg-agg/rust --exclude target

rsync --progress --verbose -r /home/eric/git/pop_model/docker/ /home/eric/git/bldg-agg/docker --exclude target


rsync --progress --verbose -r  /home/eric/git/pop_model/src/ /home/eric/git/bldg-agg/src


rsync --progress --verbose -r  /mnt/d/GRID/geopc2/BLDG_AGG/input/ /home/eric/git/bldg-agg/modules/BLDG_AGG/input 

rsync --progress --verbose -r /mnt/d/GRID/geopc2/BLDG_CHECK/input/ /home/eric/git/bldg-agg/modules/BLDG_CHECK/input

## SQL Query

```
select cur_settlement_level, new_settlement_level, count(*) from tgo.building
group by cur_settlement_level, new_settlement_level
order by cur_settlement_level, new_settlement_level;
```

## Building check input

Option 1 -- force user to create 1 layer per settlement level

Option 2 -- detect possible input formats




## faster intersection

For each settlement

for each polygon
Rasterize the settlement
Rasterize the outer boundary

this will be slow for hamlet areas


Building intersection -- 

contains
intersects

with centroid, polygon, or extent center

Intersect with each raster square


# Split algorithm

# fgb to db

```
ogr2ogr \
--config PG_USE_COPY YES \
-overwrite \
-f PostgreSQL \
-oo VERIFY_BUFFERS=NO \
        -lco OVERWRITE=YES \
        -lco GEOMETRY_NAME=shape \
        -lco FID=id \
        -progress \
        'PG: host=db dbname=bldg_agg port=5432 user=postgres password=postgres' \
        -nln test_debug \
        //modules/BLDG_AGG/working/NGA/grouped/chunk_70.fgb
```


# Building on Geopc1 / Geopc2

These machines use WSL1 and windows

Build using <repo>\docker\bldg-agg-python\build.bat

Run using <repo>\docker\binbash.bat

Use syncthing to transfer input directories from windows to geopc1 / geopc2
 

# Copying db output to file

From bash prompt after running bldg_agg

Replace tgo with nga

```
ogr2ogr \
-f "ESRI Shapefile" \
/tmp/tgo_output.shp \
"PG: host=db dbname=bldg_agg port=5432 user=postgres password=postgres" \
-nln "tgo.union" \
-progress
```

# Seeing QGIS

Port -- 25434
Host -- localhost
Db name -- bldg_agg

# 

ogr2ogr \
-f "ESRI Shapefile" \
/tmp/tgo_output.shp \
"PG: host=db dbname=bldg_agg port=5432 user=postgres password=postgres" \
-nln "tgo.union" \
-progress \
-nlt MULTIPOLYGON \
-overwrite

ogr2ogr -f "ESRI Shapefile" /tmp/bwa_output.shp "PG: host=db dbname=bldg_agg port=5432 user=postgres password=postgres" "bwa.union" -progress -nlt MULTIPOLYGON -overwrite

ogrinfo /tmp/bwa_output.shp -so bwa_output
ogrinfo /tmp/tgo_output.shp -so tgo_output


ogr2ogr \
-f "Flatgeobuf" \
/modules/BLDG_CMP/input/NGA/buildings/microsoft/nigeria.fgb \
/modules/BLDG_CMP/input/NGA/buildings/microsoft/nigeria.geojsonl \
-progress \
-overwrite




# Used for hf analysis

ogr2ogr \
-f "Flatgeobuf" \
/modules/BLDG_CHECK/input/NGA/geopode/hf.fgb \
"PG: host=subu21.ad.novel-t.ch port=5432 dbname=grid3_master user=gatekeeper password=gatekeeper" \
"master.fc_poi_health_facilities_raw_latest" \
-progress \
-overwrite


ogr2ogr \
-f "Postgres" \
"PG: host=db port=5432 dbname=bldg_check user=postgres password=postgres" \
/modules/BLDG_CHECK/input/NGA/geopode/hf.fgb \
-nln "nga_hf.hf" \
-progress \
-overwrite


create schema nga_hf;

create table nga_hf.settlements
as select * from nga.new_import;

select * from nga_hf.settlements;

select * from nga_hf.hf;

alter table nga_hf.hf
RENAME COLUMN "wkb_geometry" to "shape";

ALTER TABLE nga_hf.settlements
ADD COLUMN "centroid_geo" Geography(Point);

ALTER TABLE nga_hf.hf
ADD COLUMN "shape_geo" Geography(Point);

ALTER TABLE nga_hf.settlements
ADD COLUMN "hf_id" int REFERENCES nga_hf.hf (ogc_fid);

ALTER TABLE nga_hf.settlements
ADD COLUMN "hf_distance" float;

UPDATE nga_hf.settlements
SET centroid = ST_Centroid(shape);

UPDATE nga_hf.settlements
set centroid_geo = centroid::geography;

update nga_hf.hf
set shape_geo = hf.shape::geography;

CREATE INDEX hf_centroid_geo_geom_idx ON nga_hf.settlements USING gist (centroid_geo);

CREATE INDEX hf_centroid_gg_geom_idx ON nga_hf.hf USING gist (shape_geo);

drop index nga_hf.hf_centroid_gg_geom_idx;

select schemaname, tablename,indexname,tablespace,indexdef  from pg_indexes
where schemaname = 'nga_hf'
;

select centroid, * from nga_hf.settlements;

create unique index s_id on nga_hf.settlements (id);

UPDATE nga_hf.settlements as s
SET hf_distance = sq.hf_distance,
    hf_id = sq.hf_id
FROM (
    SELECT id, sq_lat.hf_id, sq_lat.dist as hf_distance
    FROM nga_hf.settlements sq_s
    CROSS JOIN LATERAL (
     SELECT hf.ogc_fid as hf_id, hf.shape_geo <-> sq_s.centroid_geo AS dist
     FROM nga_hf.hf hf
     --WHERE ST_DWithin(hf.shape_geo, sq_s.centroid_geo, 30000, false)
     ORDER BY dist
     LIMIT 1
    )  sq_lat
    WHERE sq_s.hf_id is null
    LIMIT 400000
) sq
WHERE sq.id = s.id;

select count(*) from nga_hf.settlements
where hf_id is null;

SELECT id, sq_lat.hf_id, sq_lat.dist as hf_distance
    FROM nga_hf.settlements sq_s
    CROSS JOIN LATERAL (
     SELECT hf.ogc_fid as hf_id, hf.shape::geography <-> sq_s.shape::geography AS dist
     FROM nga_hf.hf hf
     ORDER BY dist
     LIMIT 1
    )  sq_lat
    WHERE sq_s.hf_id is null
    LIMIT 5;

update nga_hf.settlements
set hf_id = null;

# Map preperation

Run the 3857 step, then move to qgis_server


```
sudo mv ~/git/bldg-agg/modules/BLDG_CMP/working/NGA/map_data/buildings/*.fgb ~/git/bldg-agg/modules/BLDG_CMP/qgis_server/buildings

sudo chmod -R 777  ~/git/bldg-agg/modules/BLDG_CMP/qgis_server/buildings -v

```

running qgis server

```
docker run --rm \
--publish=8380:80 \
--volume=/home/eric/git/bldg-agg/modules/BLDG_CMP/qgis_server:/etc/qgisserver \
camptocamp/qgis-server:3.22

docker run --rm --publish=8380:80 --volume=D:\git\bldg-agg\modules\BLDG_CMP\qgis_server:/etc/qgisserver camptocamp/qgis-server:3.22

docker run --rm --publish=8380:80 --volume=D:\git\bldg-agg\modules\BLDG_CMP\qgis_server:/etc/qgisserver camptocamp/qgis-server:latest

and wfs server (step in bldg cmp)

/build/run_bldg_agg.sh BLDG_CMP --country NGA 8

and the web app

cd /home/eric/git/bldg-agg/modules/BLDG_CMP/web
npm run start

or

In docker container

cd /modules/BLDG_CMP/web
npm run start &

might need npm install


--detach   camptocamp/qgis-server
```

```
/home/eric/git/bldg-agg/modules/BLDG_CMP/web
```

SCP
```
scp noveltadmin:123456++@10.1.1.125:/home/noveltadmin/eric.txt /tmp/eric.txt
```


# Work

cd /rust/fast_intersection

rm -rf /modules/temp/example && \
cargo run -- \
--log-level trace \
prepare \
--in-ogr-conn "PG: host=db dbname=bldg_agg port=5432 user=postgres password=postgres" \
--in-ogr-layer "tgo.selected" \
--ref-raster /modules/BLDG_AGG/working/TGO/rasters/ref_expanded.tif \
--output-path /modules/temp/example \
 > /modules/BLDG_AGG/working/out.txt  

 cargo run --release -- prepare \
--in-ogr-conn "PG: host=db dbname=bldg_check port=5432 user=postgres password=postgres" \
--in-ogr-layer "tgo.current" \
--dbg-out-ogr-conn "PG: host=db dbname=bldg_check port=5432 user=postgres password=postgres" \
--ref-raster /modules/BLDG_AGG/working/TGO/rasters/ref_expanded.tif \
--output-path /modules/temp/example

rm -f /modules/BLDG_CHECK/working/TGO/grid_sliced/current/level_0.fgb &&
 cargo run --release -- prepare \
 --in-ogr-conn 'PG: host=db dbname=bldg_check port=5432 user=postgres password=postgres' \
 --in-ogr-layer tgo.cur_level_0 \
 --ref-raster /modules/BLDG_AGG/working/TGO/rasters/ref_expanded.tif \
 --output-path /modules/BLDG_CHECK/working/TGO/grid_sliced/current/level_0.fgb \
 --id-field id && ogr2ogr \
--config PG_USE_COPY YES \
-overwrite \
-f PostgreSQL \
        -lco OVERWRITE=YES \
        -lco GEOMETRY_NAME=shape \
        -lco FID=id \
        -progress \
        'PG: host=db dbname=bldg_check port=5432 user=postgres password=postgres' \
        -nln tgo.test_debug \
        /modules/BLDG_CHECK/working/TGO/grid_sliced/current/level_0.fgb

# Check running queries

```
select query, wait_event,
       EXTRACT(EPOCH FROM (now() - query_start)) AS difference, *
from pg_stat_activity
where datname is not null
```

# Delete a database

```
select pg_terminate_backend(pid) from pg_stat_activity
where datname= 'bldg_agg';

drop database bldg_agg;
```

# Copy from geopc2

```
ogr2ogr \
-f "FlatGeoBuf" \
/modules/ID_SET_CMP/input/NGA/year1/settlements.fgb \
"PG: host=t1700-02.ad.novel-t.ch dbname=bldg_agg port=25434 user=postgres password=postgres" \
-nln "settlements" \
-progress \
-nlt MULTIPOLYGON \
-overwrite \
"nga.settlements"
```

```
mkdir -p /modules/ID_SET_CMP/input/NGA/year2 && \
rm -f /modules/ID_SET_CMP/input/NGA/year2/settlements.fgb && \
ogr2ogr \
-f "FlatGeoBuf" \
/modules/ID_SET_CMP/input/NGA/year2/settlements.fgb \
"PG: host=db dbname=bldg_agg port=5432 user=postgres password=postgres" \
-nln "settlements" \
-progress \
-sql "select * from nga.settlements" \
-nlt MULTIPOLYGON \
-overwrite 
```

```
mkdir -p /modules/ID_SET_CMP/input/NGA/year1 && \
rm -f /modules/ID_SET_CMP/input/NGA/year1/settlements_bldg_agg_year1.fgb && \
ogr2ogr \
-f "FlatGeoBuf" \
/modules/ID_SET_CMP/input/NGA/year1/settlements_bldg_agg_year1.fgb \
"PG: host=db dbname=bldg_agg port=5432 user=postgres password=postgres" \
-nln "settlements" \
-progress \
-sql "select * from nga.settlements" \
-nlt MULTIPOLYGON \
-overwrite 
```

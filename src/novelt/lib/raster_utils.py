import logging
from dataclasses import dataclass

import numpy as np
from osgeo import gdal

from novelt.lib.db_utils import get_results

log = logging.getLogger(__name__)

# Surpress rasterio verbosity
from rasterio import log as rasterio_log
rasterio_log.setLevel(logging.ERROR)

@dataclass
class RasterStats:
    """
    Stats helper using gdal directly
    """
    pixel_width: float = None
    pixel_height: float = None
    num_cols: int = None
    num_rows: int = None
    origin_x: float = None
    origin_y: float = None

    @staticmethod
    def from_gdal_dataset(dataset):
        geotransform = dataset.GetGeoTransform()

        return RasterStats(
            num_cols=dataset.RasterXSize,
            num_rows = dataset.RasterYSize,
            origin_x=geotransform[0],
            origin_y=geotransform[3],
            pixel_width = geotransform[1],
            pixel_height = geotransform[5],
        )

    def get_col_as_float(self, x):
        return (x - self.origin_x) / self.pixel_width

    def get_x_for_col(self, col):
        # left of the square
        return self.origin_x + col * self.pixel_width

    def right_x(self):
        return self.get_x_for_col(self.num_cols)

    def get_row_as_float(self, y):
        assert self.pixel_height < 0
        return (y - self.origin_y) / self.pixel_height

    def get_y_for_row(self, row):
        # top of the square
        assert self.pixel_height < 0
        return self.origin_y + row * self.pixel_height

    def bottom_y(self):
        return self.get_y_for_row(self.num_rows)


def get_raster_square_size(conn, sql_get_shape, tile_size_m, srid=4326):
    """
    Given sql to get the shape of the size of a raster, returns how big to make the raster square
    
    Does an average of the distances between the corners.
    
    sql should return 1 row with 1 column called shape in 4326 srid

    @:return raster_tile_size_x, raster_tile_size_y, x_min, x_max, y_min, y_max.  Note the height is generally 'stable' but the width will increase as you go towards
    the poles since to get to a tile_size you need more projected coordinates (for spherical projections)

    """

    # We use worldpops covariant size


    sql = f"""
SELECT 

  --top
  (x_max-x_min) / ( ST_Distance(top_left, top_right) / {tile_size_m}  ),

  --bottom
  (x_max-x_min) / ( ST_Distance(bot_left, bot_right) /{tile_size_m}  ),

  --left side
  (y_max-y_min) / ( ST_Distance(top_left, bot_left) /{tile_size_m}  ),

  --top side
  (y_max-y_min) / ( ST_Distance(top_right, bot_right) /{tile_size_m}  ),
  
  x_min, x_max, y_min, y_max

FROM
(
    SELECT 
        ST_Transform( ST_SetSrid( ST_MakePoint(x_min, y_min),{srid}), 4326)::geography as top_left, 
        ST_Transform( ST_SetSrid( ST_MakePoint(x_min, y_max),{srid}), 4326)::geography as bot_left, 
        ST_Transform( ST_SetSrid( ST_MakePoint(x_max, y_min),{srid}), 4326)::geography as top_right, 
        ST_Transform( ST_SetSrid( ST_MakePoint(x_max, y_max),{srid}), 4326)::geography as bot_right ,
        x_min, x_max, y_min, y_max
    FROM
    (
        SELECT 
            ST_XMin(shape) as x_min, 
            ST_XMax(shape) as x_max, 
            ST_YMin(shape) as y_min, 
            ST_YMax(shape) as y_max 
        FROM (
            {sql_get_shape}
        ) sq1 
  ) sq2
) sq3  
    """

    values = get_results(conn, sql)

    values=values[0]

    return ( values[0] + values[1] ) / 2, ( values[2] + values[3] ) / 2, values[4], values[5], values[6], values[7]


def get_raster_sum(raster_path):
    ds = gdal.Open(str(raster_path))
    raster_data = np.array(ds.GetRasterBand(1).ReadAsArray())

    no_data_value = ds.GetRasterBand(1).GetNoDataValue()

    log.info(f"No data value {no_data_value}.  {no_data_value-1}")

    # for overflow
    if no_data_value > 1:
        raster_data[ raster_data >= (no_data_value-1) ] = np.nan
    elif no_data_value < 0:
        # since we are dealing with population, anything negative is invalid 
        raster_data[ raster_data < 0 ] = np.nan
    else:
        raster_data[raster_data <= (no_data_value + 1)] = np.nan

    return np.nansum(raster_data)


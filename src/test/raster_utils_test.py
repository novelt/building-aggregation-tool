
import sys
import pytest
import rasterio
import numpy as np
from pathlib import Path
from novelt.lib.raster_utils import (
    create_raster,
    resize_raster_by_percent,
    resize_raster_by_pixel,
    extend_raster_by_geometry
)
from novelt.lib.vector_utils import Geometry



def get_temp_raster():
    return Path(__file__).parent / 'temp' / f'{sys._getframe(1).f_code.co_name}.tif'



def test_create_raster_file():
    file_raster = Path(__file__).parent / 'temp' / 'test_raster.tif'
    file_raster.unlink(missing_ok=True)
    raster = create_raster(
        origin=[0,90],
        pixel_width=1.0,
        pixel_height=1.0,
        width=180,
        height=90,
        value=1,
        srid=4326,
        dtype='int8',
        raster_out=file_raster
    )
    assert raster
    assert isinstance(raster, Path)
    assert raster == file_raster
    assert file_raster.exists()
    with rasterio.open(raster,'r') as rh:
        assert rh.width == 180
        assert rh.height == 90
        assert rh.crs.to_epsg() == 4326
        x,y = rh.res
        assert abs(x) + abs(y) == 2
        assert rh.profile['dtype'] == 'int8'
        assert rh.read(window=rasterio.windows.Window(0, 0, 1, 1)).item(0) == 1
        assert rh.read(window=rasterio.windows.Window(rh.width - 1, rh.height - 1, 1, 1)).item(0) == 1
        with pytest.raises(IndexError):
            rh.read(window=rasterio.windows.Window(rh.width, rh.height, 1, 1)).item(0) == 0
    file_raster.unlink(missing_ok=True)



def test_create_raster_memory():
    raster = create_raster(
        origin=[-10,10],
        pixel_width=0.5,
        pixel_height=0.5,
        width=20,
        height=20,
        value=0,
        nodata=0,
        srid=4326,
        dtype='int8'
    )
    assert raster
    assert isinstance(raster, rasterio.MemoryFile)
    with rasterio.open(raster, 'r') as rh:
        assert rh.width == 20
        assert rh.height == 20
        assert rh.crs.to_epsg() == 4326
        x, y = rh.res
        assert abs(x) + abs(y) == 1
        assert rh.profile['dtype'] == 'int8'
        assert rh.read(window=rasterio.windows.Window(0,0,1,1)).item(0) == 0
        assert rh.read(window=rasterio.windows.Window(rh.width-1, rh.height-1, 1, 1)).item(0) == 0
        with pytest.raises(IndexError):
            rh.read(window=rasterio.windows.Window(rh.width, rh.height, 1, 1)).item(0) == 0



def test_create_raster_ndarray():
    raster = create_raster(
        origin=[-10,10],
        pixel_width=0.3,
        pixel_height=0.8,
        value=np.full([35,66], 1589.14, dtype='float32'),
        nodata=0,
        srid=4326,
    )
    assert raster
    assert isinstance(raster, rasterio.MemoryFile)
    with rasterio.open(raster, 'r') as rh:
        assert rh.count == 1
        assert rh.width == 35
        assert rh.height == 66
        x, y = rh.res
        assert abs(x) + abs(y) == 1.1
        assert rh.profile['dtype'] == 'float32'
        assert round(rh.read(window=rasterio.windows.Window(0,0,1,1)).item(0), 2) == 1589.14
        assert round(rh.read(window=rasterio.windows.Window(34, 65, 1, 1)).item(0), 2) == 1589.14
        with pytest.raises(IndexError):
            rh.read(window=rasterio.windows.Window(35, 66, 1, 1)).item(0) == 0



def test_resize_raster_by_pixel_smaller():
    pixel = -2
    raster = get_temp_raster()
    raster_resized = Path(str(get_temp_raster()).replace('.tif', '_resized.tif'))
    raster = create_raster(
        raster_out=raster,
        origin=[0, 0],
        pixel_width=1.0,
        pixel_height=1.0,
        value=np.random.rand(10,10),
        nodata=-1,
        srid=4326,
    )
    resize_raster_by_pixel(
        raster_source=raster,
        raster_out=raster_resized,
        pixel=pixel
    )
    assert raster.exists()
    assert raster_resized.exists()
    with rasterio.open(raster, 'r') as original:
        with rasterio.open(raster_resized, 'r') as resized:
            assert resized.count == 1
            assert original.width == resized.width - 2 * pixel
            assert original.height == resized.height - 2 * pixel
            assert original.read(window=rasterio.windows.Window(-pixel, -pixel, 1, 1)).item(0) == \
                   resized.read(window=rasterio.windows.Window(0, 0, 1, 1)).item(0)
            assert original.read(
                window=rasterio.windows.Window(original.width + pixel - 1, original.height + pixel - 1, 1, 1)
            ).item(0) == resized.read(window=rasterio.windows.Window(resized.width - 1, resized.height - 1, 1, 1)).item(0)
    raster.unlink(missing_ok=True)
    raster_resized.unlink(missing_ok=True)



def test_resize_raster_by_pixel_larger():
    pixel = 2
    value = 3
    raster = get_temp_raster()
    raster_resized = Path(str(get_temp_raster()).replace('.tif', '_resized.tif'))
    raster = create_raster(
        raster_out=raster,
        origin=[0, 0],
        pixel_width=1.0,
        pixel_height=1.0,
        value=np.random.rand(10,10),
        nodata=-1,
        srid=4326,
    )
    resize_raster_by_pixel(
        raster_source=raster,
        raster_out=raster_resized,
        pixel=pixel,
        value = value
    )
    assert raster.exists()
    assert raster_resized.exists()
    with rasterio.open(raster, 'r') as original:
        with rasterio.open(raster_resized, 'r') as resized:
            assert resized.count == 1
            assert original.width == resized.width - 2 * pixel
            assert original.height == resized.height - 2 * pixel
            assert original.read(window=rasterio.windows.Window(0, 0, 1, 1)).item(0) == \
                   resized.read(window=rasterio.windows.Window(pixel, pixel, 1, 1)).item(0)
            assert original.read(
                window=rasterio.windows.Window(original.width - 1, original.height - 1, 1, 1)
            ).item(0) == resized.read(window=rasterio.windows.Window(resized.width - pixel - 1, resized.height - pixel - 1, 1, 1)).item(0)
            assert resized.read(window=rasterio.windows.Window(0, 0, 1, 1)).item(0) == value
            assert resized.read(window=rasterio.windows.Window(resized.width - pixel, resized.height - pixel, 1, 1)).item(0) == value
    raster.unlink(missing_ok=True)
    raster_resized.unlink(missing_ok=True)



def test_resize_raster_by_percent_smaller():
    scalar = 0.5
    raster = get_temp_raster()
    raster_resized = Path(str(get_temp_raster()).replace('.tif', '_resized.tif'))
    raster = create_raster(
        raster_out=raster,
        origin=[0, 0],
        pixel_width=1.0,
        pixel_height=1.0,
        value=np.random.rand(10,10),
        nodata=-1,
        srid=4326,
    )
    resize_raster_by_percent(
        raster_source=raster,
        raster_out=raster_resized,
        scalar=scalar
    )
    assert raster.exists()
    assert raster_resized.exists()
    with rasterio.open(raster, 'r') as original:
        with rasterio.open(raster_resized, 'r') as resized:
            assert resized.count == 1
            assert resized.width == 6
            assert resized.height == 6
            assert original.width == resized.width + 4
            assert original.height == resized.height + 4
    raster.unlink(missing_ok=True)
    raster_resized.unlink(missing_ok=True)



def test_resize_raster_by_percent_larger():
    scalar = 2.0
    value = 4
    raster = get_temp_raster()
    raster_resized = Path(str(get_temp_raster()).replace('.tif', '_resized.tif'))
    raster = create_raster(
        raster_out=raster,
        origin=[0, 0],
        pixel_width=1.0,
        pixel_height=1.0,
        value=np.random.rand(10,10),
        nodata=-1,
        srid=4326,
    )
    resize_raster_by_percent(
        raster_source=raster,
        raster_out=raster_resized,
        scalar=scalar,
        value=value
    )
    assert raster.exists()
    assert raster_resized.exists()
    with rasterio.open(raster, 'r') as original:
        with rasterio.open(raster_resized, 'r') as resized:
            assert resized.count == 1
            assert resized.width == round(scalar * original.width)
            assert resized.height == round(scalar * original.height)
            assert original.width == resized.width * 1 / scalar
            assert original.height == resized.height * 1 / scalar
            assert resized.read(window=rasterio.windows.Window(0, 0, 1, 1)).item(0) == value
            assert resized.read(window=rasterio.windows.Window(resized.width - 1, resized.height - 1, 1, 1)).item(0) == value
    raster.unlink(missing_ok=True)
    raster_resized.unlink(missing_ok=True)



def test_resize_raster_by_extent_1():
    raster = get_temp_raster()
    point = Geometry([-1,1])
    value=7
    raster_extended = Path(str(get_temp_raster()).replace('.tif', '_extended.tif'))
    raster = create_raster(
        raster_out=raster,
        origin=[0, 0],
        pixel_width=1.0,
        pixel_height=1.0,
        value=np.random.rand(10, 10),
        nodata=-1,
        srid=4326,
    )
    extend_raster_by_geometry(
        raster_source=raster,
        raster_out=raster_extended,
        geometry=point,
        value=value
    )
    assert raster.exists()
    assert raster_extended.exists()
    with rasterio.open(raster, 'r') as original:
        with rasterio.open(raster_extended, 'r') as extended:
            assert extended.count == 1
            assert extended.width == 11
            assert extended.height == 11
            assert extended.read(window=rasterio.windows.Window(0, 0, 1, 1)).item(0) == value
            assert extended.read(
                window=rasterio.windows.Window(extended.width - 1, extended.height - 1, 1, 1)
            ).item(0) == original.read(
                window=rasterio.windows.Window(original.width - 1, original.height - 1, 1, 1)
            ).item(0)
    raster.unlink(missing_ok=True)
    raster_extended.unlink(missing_ok=True)



def test_resize_raster_by_extent_2():
    raster = get_temp_raster()
    point = Geometry([12,2])
    raster_extended = Path(str(get_temp_raster()).replace('.tif', '_extended.tif'))
    raster = create_raster(
        raster_out=raster,
        origin=[0, 0],
        pixel_width=1.0,
        pixel_height=1.0,
        value=np.random.rand(10, 10),
        nodata=-1,
        srid=4326,
    )
    extend_raster_by_geometry(
        raster_source=raster,
        raster_out=raster_extended,
        geometry=point,
    )
    assert raster.exists()
    assert raster_extended.exists()
    with rasterio.open(raster, 'r') as original:
        with rasterio.open(raster_extended, 'r') as extended:
            assert extended.count == 1
            assert extended.width == 12
            assert extended.height == 12
            assert extended.read(window=rasterio.windows.Window(0, 0, 1, 1)).item(0) == original.nodata
            assert extended.read(window=rasterio.windows.Window(0, 1, 1, 1)).item(0) == original.nodata
            assert extended.read(window=rasterio.windows.Window(0, 2, 1, 1)).item(0) != original.nodata
            assert extended.read(
                window=rasterio.windows.Window(extended.width - 1, extended.height - 1, 1, 1)
            ).item(0) != original.read(
                window=rasterio.windows.Window(original.width - 1, original.height - 1, 1, 1)
            ).item(0)
            assert extended.read(
                window=rasterio.windows.Window(0, extended.height - 1, 1, 1)
            ).item(0) == original.read(
                window=rasterio.windows.Window(0, original.height - 1, 1, 1)
            ).item(0)
            assert extended.read(
                window=rasterio.windows.Window(0, 2, 1, 1)
            ).item(0) == original.read(
                window=rasterio.windows.Window(0, 0, 1, 1)
            ).item(0)
    raster.unlink(missing_ok=True)
    raster_extended.unlink(missing_ok=True)


# Test create_raster_from_source
# tests = [
#     dict(
#         extent=buildings_extent.scale(1.1),
#         raster='.temp/raster_extended.tif'
#     ),
#     dict(
#         extent=buildings_extent.scale(0.5),
#         raster='.temp/raster_shrinked.tif'
#     ),
#     dict(
#         extent=Geometry([-16, 13, -2, 18.5]),
#         raster='.temp/raster_partial_left.tif'
#     ),
#     dict(
#         extent=Geometry([-14.8984, 17.2, 0.557, 22.22]),
#         raster='.temp/raster_partial_left2.tif'
#     ),
#     dict(
#         extent=Geometry([0.25, 7, 7, 18.5]),
#         raster='.temp/raster_partial_right.tif'
#     ),
#     dict(
#         extent=Geometry([-60, 7, -35, 20]),
#         raster='.temp/raster_outside.tif'
#     ),
#     # dict(
#     #     extent=Geometry([-200, 7, -150, 20]),
#     #     raster='.temp/raster_cross_idl_1.tif'
#     # ),
#     # dict(
#     #     extent=Geometry([150, 7, 200, 20]),
#     #     raster='.temp/raster_cross_idl_2.tif'
#     # )
# ]
#
# # Write shapes
# from novelt.lib.raster_utils import create_raster_from_source
# buildings_extent.to_file('.temp/buildings.shp', True)
# for t in tests:
#     print('####################################################################################')
#     print('Creating new raster:', t.get('raster'), 'from extent', t.get('extent'))
#     t.get('extent').to_file(t.get('raster').replace('.tif', '.shp'), True)
#     create_raster_from_source(
#         raster_source=Path(cfg.POPULATION_RASTER_PATH),
#         raster_out=t.get('raster'),
#         window=t.get('extent'),
#         #value=range(0,2)
#     )
# print('####################################################################################')
# # create_raster_from_source(
# #     raster_source=Path(cfg.POPULATION_RASTER_PATH),
# #     raster_out='.temp/copy.tif',
# #     window=get_raster_bounds(Path(cfg.POPULATION_RASTER_PATH))
# # )
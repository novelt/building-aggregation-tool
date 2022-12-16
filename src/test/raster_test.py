import pytest
from pathlib import Path
from novelt.lib.raster_utils import create_raster


def test_create_raster_file():
    file_raster = Path(__file__).parent / 'test_raster.tif'

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
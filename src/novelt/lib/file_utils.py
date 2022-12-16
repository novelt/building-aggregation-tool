import logging
import os
import shutil
import sys
import time
from typing import Generator, List, Tuple
from pathlib import Path

import fiona

log = logging.getLogger(__name__)


def read_file_to_string(file_path, preserve_new_lines = True):
    with open(file_path, 'r') as myfile:
        data=myfile.read()

        if not preserve_new_lines :
            data = data.replace('\r', '')
            data = data.replace('\n', '')

    return data


def mkdir_p(path, retryCount=2):
    try:
        os.makedirs(path, exist_ok=True)
    except Exception as ex:
        if retryCount>0:
            time.sleep(2)
            mkdir_p(path, retryCount=retryCount-1)


def remove_file(file_path):
    try:
        os.remove(file_path)
    except OSError:
        pass

    if os.path.isfile(file_path):
        raise Exception("Unable to remove file: {}".format(file_path))


def remove_dir(dir_path, dir_prefix, raise_exception_on_error=True):
    """
    Does a recursive delete.
    Pass a dirPrefix to enforce that the dir starts with it to make sure
    you aren't deleting C:\
    """

    # Already deleted
    if not dir_path.exists():
        return

    # https://github.com/novelt/pop_model/issues/176
    # raise Exception(f"due to docker locking issues, please remove the directory \"{dir_path}\" in windows explorer")

    if dir_prefix is not None and not str(dir_path).startswith(str(dir_prefix)):
        raise Exception(f"Directory {dir_path} does not start with {dir_prefix}")

    shutil.rmtree(dir_path, False)

    if raise_exception_on_error and dir_path.exists():
        raise Exception(f"{dir_path} still exists" )

# https://stackoverflow.com/questions/33135038/how-do-i-use-os-scandir-to-return-direntry-objects-recursively-on-a-directory/33135143
def get_sub_directories(path: Path) -> Generator[Path, Path, None]:
    """Recursively yield DirEntry objects for given directory.
    Returns all sub directories that contain no directories
    """
    for entry in os.scandir(path):
        if entry.is_dir(follow_symlinks=False):
            yield from get_sub_directories(Path(entry.path))

    yield path


def get_vector_layers(dir_name: Path, err_msg: str = "") -> List[Tuple[Path, str]]:
    if not dir_name.exists():
        log.error(
            f"{dir_name} does not exist.  {err_msg}")
        sys.exit(1)

    sub_directories = list(get_sub_directories(dir_name))

    input_sources = []

    log.debug(f"Sub directories: {sub_directories}")

    input_file_paths: List[Path] = []
    for sub_dir in sub_directories:
        if sub_dir.suffix.upper() == ".GDB":
            pass
            # input_file_paths.append(sub_dir)
            # the base directory will be included in sub_directories, so this
            # gdb will be added in the base directory iterdir
        else:
            input_file_paths.extend(sub_dir.iterdir())

    if len(input_file_paths) == 0:
        log.error(
            f"{dir_name} does not contain any geospatial files / directories.  {err_msg}")
        sys.exit(1)

    for f in input_file_paths:

        if ".Identifier" in f.name:
            continue

        ext = f.suffix.upper()
        # geojsonl is really slow, so it should be converted first in a previous step
        if ext in [".XML", ".SHX", ".SBX", ".PRJ", ".CPG", ".SBN",
                   ".ZIP", ".DBF", ".PDF", ".GEOJSONL", ".CSV"]:
            continue

        if ext != ".GDB" and f.is_dir():
            continue

        log.debug(f"Looking at {f}")
        layer_names = fiona.listlayers(f)

        for l in layer_names:
            if "_deleted" in str(l):
                log.info(f"Skipping layer named deleted: {l}")
                continue
            input_sources.append((f, str(l)))

    if len(input_sources) == 0:
        joined_str = ', '.join([str(p) for p in input_file_paths])
        log.error(
            f"Did not find any layers in {dir_name} in inputs {joined_str}.  {err_msg} ")
        sys.exit(1)

    return input_sources

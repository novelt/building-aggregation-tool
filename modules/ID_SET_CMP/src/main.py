# -*- coding: utf-8 -*-

# This file is part of the Building Aggregration Tool
# Copyright (C) 2022 Novel-T
# 
# The Building Aggregration Tool is free software: you can redistribute it and/or modify
# it under the terms of the GNU General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
# 
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU General Public License for more details.
# 
# You should have received a copy of the GNU General Public License
# along with this program.  If not, see <http://www.gnu.org/licenses/>.

import argparse
import os
import sys
from pathlib import Path

# Add common directory to sys.path
from novelt.lib.thread_utils import run_process_stream_output

COMMON_DIR = Path(__file__).parent.parent.parent.parent / 'src'
assert COMMON_DIR.exists()
sys.path.append(str(COMMON_DIR))

from novelt.lib.gdm_utils import run_step, run_gdm, init_log


def gdm_process(start_step, end_step, args):


    init_log(cfg)

    current_step_num = 0

    step_list = [
        steps.print_input_info,

        steps.step_fix_year1_input,

        steps.step_rasterize_year1_input,
        steps.step_rasterize_year2_input,

        steps.step_create_new_database,

        steps.step_squares_to_database,

        steps.step_export_csvs

    ]

    for step_fn in step_list:
        current_step_num = run_step(cfg, current_step_num, start_step, end_step, step_fn)
    return current_step_num


# ------------------------------------------------------------
# Main program
# ------------------------------------------------------------
if __name__ == '__main__':
    parser = argparse.ArgumentParser(description='Aggregates building footprints into settlement extents')

    parser.add_argument('--clean', action='store_true',
                        help='If set, will set clean flag.  This will remove any output for a given step.')

    parser.add_argument("start_step", type=int, nargs="?", default=1, help="Which step to start running, defaults to 1")
    parser.add_argument("stop_step", type=int, nargs="?", help="Stop executing after running this step")
    parser.add_argument("--group-distance", type=float, nargs="?", default=0.0008333,
                        help="How far apart in the reference raster CRS should buildings be considered part of the same settlement"
                        )

    parser.add_argument("--country", type=str, help="3 letter country ISO code", required=True)

    parser.add_argument("--contour-value", type=int, default=12, nargs="?", help="In contour step that draws a contour around dense building squares in the building count raster, defines how many buildings must be in these squares")

    parser.add_argument("--contour-min-area", type=int, default=400000, nargs="?", help="Area in square meters of how large the contour area must be to consider a settlement a BUA")

    parser.add_argument('--gen-docs', action='store_true', help="If set will generate step documentation")

    parser.add_argument("--chunk-rows", type=int, default=10, nargs="?", help="When buildings are split into square chunks, how many rows.  Defaults to 10.  Increase if a building related step fails due to memory/timeout issues.")
    parser.add_argument("--chunk-cols", type=int, default=10, nargs="?", help="When buildings are split into square chunks, how many columns")

    parser.add_argument("--log-level", type=str, default="warn",
                        help="Error/Warn/Info/Debug/Trace")

    args = parser.parse_args()

    # Given script parameters
    start_step = args.start_step

    if not args.stop_step:
        args.stop_step = args.start_step

    end_step = args.stop_step

    os.environ["COUNTRY_CODE"] = args.country.upper()

    if args.gen_docs:
        os.environ["GDM_GENERATE_DOCS"] = "true"
        start_step = 1
        end_step = 999

    # Import here only after the COUNTRY_CODE has been set because the config will depend on the country code passed in
    from config import Config as cfg
    import steps

    # set the values from config that were passed in by command line
    cfg.CLEAN = args.clean
    cfg.GROUP_DISTANCE = args.group_distance
    cfg.CONTOUR_VALUE = args.contour_value
    cfg.CONTOUR_MIN_BUA_AREA = args.contour_min_area
    cfg.CHUNK_ROWS = args.chunk_rows
    cfg.CHUNK_COLS = args.chunk_cols
    cfg.LOG_LEVEL = args.log_level

    gdm_process(start_step, end_step, args)

    if args.gen_docs:
        run_process_stream_output(f"""mkdir -p "{cfg.MODULE_DIR / 'docs'}" """)
        doc_output_path = cfg.MODULE_DIR / "docs" / "bldg_agg_script_steps.html"
        run_process_stream_output(f"""pandoc --from gfm --to html --standalone --output "{doc_output_path}" --metadata pagetitle="Building Aggregration Steps" "{cfg.MODULE_DIR}/working/doc.md" """)

        print(f"Documentation generated in {doc_output_path}")
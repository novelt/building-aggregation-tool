# coding=utf-8
import argparse
import logging
import inspect

from novelt.lib import file_utils, logger_utils, geo_db_utils
import sys
import time
import os
import re

from slack import WebClient

slack_api_token = os.environ.get('SLACK_API_TOKEN')
slack_client = None 
if slack_api_token is not None and len(slack_api_token.strip()) > 0:
    slack_client = WebClient(token=slack_api_token)

log = logging.getLogger(__name__)


def format_seconds(seconds):
    m, s = divmod(seconds, 60)
    h, m = divmod(m, 60)
    return "%dh:%02dm:%02ds" % (h, m, s)

def init_log(cfg):
    # Init logger
    # log = logger_utils.InitLogger(cfg.LogPath)
    file_utils.mkdir_p(cfg.LOG_PATH.parent)

    print("Saving log to %s" % cfg.LOG_PATH)

    logger = logger_utils.init_log(log_name=None,
                                   console_level=logging.DEBUG,
                                   file_level=logging.DEBUG,
                                   log_path=cfg.LOG_PATH,
                                   log_format_str="%(asctime)s %(filename)s:%(lineno)d %(levelname)s %(name)s ==> %(message)s\n")


def run_gdm(gdm_func):

    # logging.getLogger('gts.postgis').setLevel(logging.WARNING)

    parser = argparse.ArgumentParser(description='Compute zonal stats of settlement parts')


    parser.add_argument('--clean', action='store_true',
                        help='If true, will set clean flag')

    parser.add_argument("start_step", type=int)
    parser.add_argument("stop_step", type=int, nargs="?")

    parser.add_argument("--country", type=str)

    args = parser.parse_args()


    # Given script parameters
    start_step = args.start_step

    if not args.stop_step:
        args.stop_step = args.start_step


    end_step = args.stop_step


    log.info('Start python script with parameters: ' + ' '.join(sys.argv))
    # log.info(f"From {args.start_step} to {args.stop_step}")
    #
    # sys.exit(0)

    current_step_num = gdm_func(start_step, end_step, args)

    if current_step_num <= end_step:
        log.info("GDM finished running to stop step %s" % current_step_num)
        sys.exit(7)


DOC_GET_DESC_REGEX = re.compile(r"""
        ^
        (.*?)
        \s*
        \n
        \s*?
        (?:\n|\Z)    #A new line or end of string in non capturing groups  
        .*
        """, re.VERBOSE | re.DOTALL)


def run_step(cfg, current_step_num, start_step, stop_step, step_fn, step_desc = None):

    if step_fn.__doc__ is None:
        raise Exception(f"Need to document step #{current_step_num + 1}: {step_fn.__name__}")
    m = DOC_GET_DESC_REGEX.match(step_fn.__doc__)
    if m is None:
        raise Exception(f"Docstring did not match regex step #{current_step_num + 1}: {step_fn.__name__}")

    step_desc = DOC_GET_DESC_REGEX.match(step_fn.__doc__).group(1)
    current_step_num += 1

    if start_step <= current_step_num <= stop_step:

        if 'GDM_GENERATE_DOCS' in os.environ:
            cfg.DocFilePath = cfg.MODULE_DIR / "working" / 'doc.md'

            if current_step_num == 1:
                if os.path.isfile(cfg.DocFilePath):
                    os.remove(cfg.DocFilePath)

            with open(cfg.DocFilePath, "a") as myfile:
                # myfile.write("-" * 60 + "\n")

                step_name = ''.join([s.capitalize() for s in step_fn.__name__.replace('step_', '').split('_')])

                # markdown autonumbers
                myfile.write(f"1. **{step_name}**\n" )

                pdoc = step_fn.__doc__
                if pdoc is None:
                    pdoc = "TODO"

                pdoc = pdoc.replace('\t', '').strip()

                pdoc = '\n'.join(['     ' + s.strip() for s in pdoc.split('\n')])

                myfile.write("\n%s\n" % pdoc)

                return current_step_num

        fnData = inspect.signature(step_fn)

        start_msg = fr"""
------------------------------------------------------------
			{current_step_num} - {step_fn.__name__}: {step_desc} 
------------------------------------------------------------""" 

        log.info(start_msg)
        
        if slack_client is not None:
            slack_client.chat_postMessage(channel='#pop-model-status', text=f"Bldg Agg -- *{cfg.MODULE_NAME} {cfg.SCHEMA_NAME.upper()}*\n```{start_msg}```")

        try:

            startTime = time.time()



            if "current_step_num" in fnData.parameters:
                step_fn(current_step_num)
            elif "conn" in fnData.parameters:

                conn = geo_db_utils.create_db_connection(cfg)
                step_fn(conn)
                try:
                    conn.close()
                except Exception as ex:
                    log.info(f"Connection not closed: {ex}")

            else:
                step_fn()

            stepDurationSecs = time.time() - startTime

            completed_msg = "Completed Step #%s - %s in %s" % (
                    current_step_num,
                    step_desc,
                    format_seconds(stepDurationSecs))
            log.info(completed_msg)
            if slack_client is not None:
                slack_client.chat_postMessage(channel='#pop-model-status', text=f"Bldg Agg -- *{cfg.MODULE_NAME} {cfg.SCHEMA_NAME.upper()}*\n```{completed_msg}```")

        except Exception as e:
            log.error("Caught exception")

            log.exception(e)

            if slack_client is not None:
                slack_client.chat_postMessage(channel='#pop-model-status', text=f"Exception! *{cfg.MODULE_NAME} {cfg.SCHEMA_NAME.upper()}* ```{e}```")


            sys.exit(1)

    return current_step_num



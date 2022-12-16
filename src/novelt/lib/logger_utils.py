# -*- coding: utf-8 -*-
import logging
import logging.handlers
import os
from novelt.lib import file_utils


class LogFilter(logging.Filter):

    def filter(self, record):
        if record.name.endswith("trace") and record.levelno <= logging.DEBUG:
            return False

        return True


class SummaryLogFilter(logging.Filter):

    def filter(self, record):

        if record.name.endswith("trace"):
            return False

        if record.levelno <= logging.DEBUG:
            return False

        return True


def init_log(log_name = None,
             console_level = logging.INFO,
             file_level = logging.DEBUG,
             log_path = None,
             log_format_str = "%(asctime)s %(filename)s:%(lineno)d %(levelname)s %(name)s ==> %(message)s\n"):
    if log_path is not None and not os.path.exists(os.path.dirname(log_path)):
        file_utils.mkdir_p(os.path.dirname(log_path))

    log_formatter = logging.Formatter(log_format_str)
    # logFormatter = logging.Formatter("%(asctime)s %(filename)s:%(lineno)d %(levelname)s %(name)s %(levelno)s %(message)s")
    log = logging.getLogger(log_name)

    if log_path is not None:
        file_handler = logging.handlers.RotatingFileHandler(log_path, mode = 'a', maxBytes = 1000000, backupCount = 10)

        # fileHandler = logging.FileHandler(logFile, mode = 'a+')
        file_handler.setFormatter(log_formatter)
        file_handler.setLevel(file_level)

        log.addHandler(file_handler)

        log_folder = os.path.dirname(log_path)

        if False:

            file_handler = logging.FileHandler(os.path.join(log_folder, "summary.log"),
                                           mode = 'a',
                                           encoding = "utf-8"
                                           )

        file_handler = logging.handlers.RotatingFileHandler(os.path.join(log_folder, "summary.log"), mode='a',
                                                            maxBytes=1000000, backupCount=3)

        file_handler.setFormatter(log_formatter)
        file_handler.setLevel(file_level)

        # Optional
        file_handler.addFilter(SummaryLogFilter())

        log.addHandler(file_handler)

    console_handler = logging.StreamHandler()
    console_handler.setFormatter(log_formatter)
    console_handler.setLevel(console_level)
    console_handler.addFilter(LogFilter())
    log.addHandler(console_handler)

    log.setLevel(min(console_level, file_level))

    logging.getLogger("rasterio").setLevel(logging.INFO)

    return log



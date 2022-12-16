import sys

from novelt.lib import gdm_utils

from novelt.config.test_config import Config as cfg
from novelt.lib.gdm_utils import run_step

test_array = []


def step1():
    """
    step 1
    :return:
    """
    test_array.append(1)


def step2():
    """
    step 2
    """
    test_array.append(2)


def step3():
    """
    step 3
    """
    test_array.append(3)


def step4():
    """
    step 4
    """
    test_array.append(4)


def step5():
    """
    step 5
    """
    test_array.append(5)


def gdm_process(start_step, end_step):

    current_step_num = 0

    step_list = [
        step1, step2, step3, step4, step5

    ]

    for step_fn in step_list:
        current_step_num = run_step(cfg, current_step_num, start_step, end_step, step_fn)
    return current_step_num

def fake_sys_exit(i):
    pass

# ------------------------------------------------------------
# Main program
# ------------------------------------------------------------
def test_run_gdm():

    sys.exit = fake_sys_exit

    global test_array

    sys.argv[1] = "3"
    sys.argv[2] = "5"
    gdm_utils.run_gdm(cfg, gdm_process)

    assert test_array == [3,4,5]

    test_array = []

    sys.argv[1] = "1"
    sys.argv[2] = "1"
    gdm_utils.run_gdm(cfg, gdm_process)

    assert test_array == [1]

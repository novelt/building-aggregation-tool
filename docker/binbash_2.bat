%~d0
cd %~dp0

type nul >> local.env

set COMPOSE_PROJECT_NAME=bldg_agg

docker-compose -f docker-compose.yml -f docker-compose-dev.yml run --rm  bldg-agg-python /bin/bash
rem docker-compose -f docker-compose.yml -f docker-compose-dev.yml run --rm bldg-agg-python /bin/bash
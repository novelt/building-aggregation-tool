%~d0
cd %~dp0

type nul >> local.env

set COMPOSE_PROJECT_NAME=bldg_agg

set PATH=C:\Windows\system32;C:\Windows;C:\Windows\System32\Wbem;;C:\WINDOWS\System32\OpenSSH\;C:\Program Files\Git\cmd;C:\Program Files\nodejs\;C:\Program Files\Pandoc\;C:\Program Files\PuTTY\;C:\Program Files\Docker\Docker\resources\bin;C:\ProgramData\DockerDesktop\version-bin;C:\Users\eg\.cargo\bin;C:\Users\eg\AppData\Local\Microsoft\WindowsApps;C:\Users\eg\AppData\Roaming\npm;C:\Program Files\PostgreSQL\11\lib;C:\Program Files\PostgreSQL\11\bin;C:\OSGeo4W64\lib;C:\OSGeo4W64\bin

docker-compose -f docker-compose.yml -f docker-compose-dev.yml run --rm --service-ports bldg-agg-python /bin/bash
rem docker-compose -f docker-compose.yml -f docker-compose-dev.yml run --rm bldg-agg-python /bin/bash
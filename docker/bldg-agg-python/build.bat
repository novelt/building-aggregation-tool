%~d0
cd %~dp0..

rem --build-arg BUILDKIT_INLINE_CACHE=1
docker build --target builder --cache-from=novelt/bldg-agg-python-builder:latest --tag novelt/bldg-agg-python-builder:latest .. -f ./bldg-agg-python/Dockerfile
if %errorlevel% neq 0 exit /b %errorlevel%

rem --build-arg BUILDKIT_INLINE_CACHE=1
docker build --target bldg-agg-python --cache-from=novelt/bldg-agg-python-builder:latest --cache-from=novelt/bldg-agg-python:latest  --tag novelt/bldg-agg-python:latest .. -f ./bldg-agg-python/Dockerfile
rem docker build --target bldg-agg-python .. -f ./bldg-agg-python/Dockerfile
if %errorlevel% neq 0 exit /b %errorlevel%

docker-compose build
if %errorlevel% neq 0 exit /b %errorlevel%

call binbash.bat
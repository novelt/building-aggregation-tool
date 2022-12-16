# # This file is part of the Building Aggregration Tool
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

# Build FileGDB & GDAL with Python support and numpy
FROM ubuntu:20.04 AS builder

# Used much of
# https://github.com/OSGeo/gdal/blob/master/gdal/docker/ubuntu-full/Dockerfile
# Setup build env for PROJ
RUN apt-get update -y \
    && DEBIAN_FRONTEND=noninteractive apt-get install -y --fix-missing --no-install-recommends \
            software-properties-common build-essential ca-certificates \
            git make cmake wget unzip libtool automake \
            zlib1g-dev libsqlite3-dev pkg-config sqlite3 libcurl4-gnutls-dev \
            libtiff5-dev \
    && rm -rf /var/lib/apt/lists/*

# Setup build env for GDAL
RUN apt-get update -y \
    && DEBIAN_FRONTEND=noninteractive apt-get install -y --fix-missing --no-install-recommends \
       libcharls-dev libopenjp2-7-dev \
       python3-dev python3-numpy \
       liblzma-dev libgeos-dev \
       curl libxml2-dev libexpat-dev libxerces-c-dev \
       libnetcdf-dev \
       libspatialite-dev swig \
       libfreexl-dev libepsilon-dev \
       liblcms2-2 libpcre3-dev libdap-dev libfyba-dev \
       libkml-dev libmysqlclient-dev libogdi-dev \
       libzstd-dev \
       libpq-dev libssl-dev libboost-dev \
       autoconf automake bash-completion libarmadillo-dev \
       libdeflate-dev \
    && rm -rf /var/lib/apt/lists/*

# deflate and liblzma-dev is a raster compression algo

## Begin custom
RUN apt-get update -y \
    && DEBIAN_FRONTEND=noninteractive apt-get install -y --fix-missing --no-install-recommends \
       # Added for proj
       dos2unix \
    && rm -rf /var/lib/apt/lists/*

# RUN apt install -y python3.8-distutils
COPY ./docker/bldg-agg-python/build_kml.sh ./docker/bldg-agg-python/build_geos.sh \
    ./docker/bldg-agg-python/bh-gdal.sh ./docker/bldg-agg-python/bh-proj.sh \
    ./docker/bldg-agg-python/bh-static-proj.sh \
    /
RUN dos2unix build_kml.sh && dos2unix build_geos.sh && \
    dos2unix bh-gdal.sh && dos2unix bh-proj.sh && dos2unix bh-static-proj.sh

RUN chmod +x build_kml.sh build_geos.sh bh-gdal.sh bh-proj.sh bh-static-proj.sh

#Build File Geodatabase

RUN mkdir -p /build_thirdparty/usr/lib

ARG WITH_FILEGDB=yes
RUN if echo "$WITH_FILEGDB" | grep -Eiq "^(y(es)?|1|true)$"  ; then ( \
  wget -q https://github.com/Esri/file-geodatabase-api/raw/master/FileGDB_API_1.5.1/FileGDB_API_1_5_1-64gcc51.tar.gz \
  && tar -xzf FileGDB_API_1_5_1-64gcc51.tar.gz \
  && chown -R root:root FileGDB_API-64gcc51 \
  && mv FileGDB_API-64gcc51 /usr/local/FileGDB_API \
  && rm -rf /usr/local/FileGDB_API/lib/libstdc++* \
  && cp /usr/local/FileGDB_API/lib/* /build_thirdparty/usr/lib \
  && cp /usr/local/FileGDB_API/include/* /usr/include \
  && rm -rf FileGDB_API_1_5_1-64gcc51.tar.gz \
  ) ; fi

ARG WITH_DEBUG_SYMBOLS=no

# Build PROJ

ARG PROJ_VERSION=7.2.0
ARG PROJ_INSTALL_PREFIX=/usr/local
# https://github.com/OSGeo/gdal/blob/master/gdal/docker/ubuntu-full/bh-proj.sh
RUN /bh-proj.sh

RUN /bh-static-proj.sh

ARG GDAL_VERSION=v3.2.3
ARG GDAL_RELEASE_DATE
ARG GDAL_BUILD_IS_RELEASE

# Build GDAL
# https://github.com/OSGeo/gdal/blob/master/gdal/docker/ubuntu-full/bh-gdal.sh
# seems -with-geos and --with-pg are redundant
RUN /bh-gdal.sh

# For now use libgeos-dev which is @ 3.8.0
# RUN ./build_geos.sh
#RUN ./build_proj.sh
#RUN ./build_kml.sh5
#RUN ./build_gdal.sh

# Build Python image with required modules
FROM ubuntu:20.04 AS bldg-agg-python


# Get Python and Python libs
# RUN apt update && apt install -y software-properties-common && add-apt-repository -y ppa:deadsnakes/ppa

# prevent timezone questions
RUN apt update && DEBIAN_FRONTEND=noninteractive apt install -y dos2unix sqlite3 \
python3.8 python3.8-distutils \
libsqlite3-mod-spatialite wget libspatialindex-dev libpq-dev \
texlive-latex-base texlive-fonts-recommended texlive-latex-extra \
gettext-base curl netcat \
llvm-dev libclang-dev clang libc++-dev \
python3-pip \
libdeflate0 libgeos-dev libssl-dev graphviz libtiff-dev rsync libcgal-dev \
nodejs npm \
libatk-bridge2.0-0 libgtk-3-0 libgbm-dev libasound2


# Install PostGIS (to have client tools)
COPY ./docker/bldg-agg-python/install_postgis.sh /
RUN dos2unix install_postgis.sh && chmod +x install_postgis.sh && ./install_postgis.sh

# Add this at the end since it will likely change more than the above
COPY ./docker/bldg-agg-python/wait-for.sh /

# Install DOC tools
RUN wget --quiet https://github.com/jgm/pandoc/releases/download/2.9/pandoc-2.9-linux-amd64.tar.gz && \
    tar xvzf pandoc-2.9-linux-amd64.tar.gz --strip-components 1 -C /usr/local

COPY ./docker/bldg-agg-python/custom_entrypoint.sh /usr/local/bin/
RUN chmod +x /usr/local/bin/custom_entrypoint.sh /wait-for.sh && \
    dos2unix /usr/local/bin/custom_entrypoint.sh /wait-for.sh

# Install Python modules
# RUN apt update && apt install -y
RUN pip3 install \
psycopg2==2.8.6 python-dotenv pyproj pytest pytest-cov \
numpy pyyaml pandas geopandas shapely sphinx matplotlib \
contextily==1.0rc2 descartes diskcache SQLAlchemy GeoAlchemy2 yamlloader xlrd slackclient \
scipy pyreadstat keras tflearn tensorflow sklearn jupyter \
rtree pytablewriter pytablewriter[excel] xlsxwriter graphviz pygeos seaborn Pygments==2.6.1 alphashape networkx


# Get Rust
RUN curl https://sh.rustup.rs -sSf | bash -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"
#Rust needs a username
ENV USER="root"
# not used, just to trigger a rebuild
ENV RUST_VERSION="1.54"

# Attempt to order layers starting with less frequently varying ones

COPY --from=builder  /build_thirdparty/usr/ /usr/
#COPY --from=builder  /build/usr/share/java /usr/share/java

ARG PROJ_DATUMGRID_LATEST_LAST_MODIFIED
ARG PROJ_INSTALL_PREFIX=/usr/local
COPY --from=builder  /build${PROJ_INSTALL_PREFIX}/share/proj/ ${PROJ_INSTALL_PREFIX}/share/proj/
COPY --from=builder  /build${PROJ_INSTALL_PREFIX}/include/ ${PROJ_INSTALL_PREFIX}/include/
COPY --from=builder  /build${PROJ_INSTALL_PREFIX}/bin/ ${PROJ_INSTALL_PREFIX}/bin/
COPY --from=builder  /build${PROJ_INSTALL_PREFIX}/lib/ ${PROJ_INSTALL_PREFIX}/lib/
COPY --from=builder /projstatic/build/lib/libproj.a /usr/local/lib

RUN ldconfig
#     && projsync --system-directory --all

COPY --from=builder  /build/usr/share/gdal/ /usr/share/gdal/
COPY --from=builder  /build/usr/include/ /usr/include/
COPY --from=builder  /build_gdal_python/usr/ /usr/
COPY --from=builder  /build_gdal_version_changing/usr/ /usr/
#
RUN ldconfig

ENV PYTHONPATH=/usr/local/lib/python3.8/dist-packages:/usr/local/lib/python3.8/site-packages:/usr/local/lib/python3/dist-packages:/src \
    ENV_FILE=local_docker.env

RUN cargo install bindgen-cli cargo2junit cargo-tarpaulin

COPY ./src /src
COPY ./rust /rust
# COPY ./modules /modules
COPY ./docker/bldg-agg-python/run_bldg_agg.sh  ./docker/bldg-agg-python/run_tests.sh /
RUN dos2unix /*.sh
RUN chmod 755 /*.sh

ENTRYPOINT ["custom_entrypoint.sh"]
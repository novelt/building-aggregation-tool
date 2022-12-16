#!/bin/sh

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

set -eu

if [ "${GDAL_VERSION}" = "master" ]; then
    GDAL_VERSION=$(curl -Ls https://api.github.com/repos/OSGeo/gdal/commits/HEAD -H "Accept: application/vnd.github.VERSION.sha")
    export GDAL_VERSION
    GDAL_RELEASE_DATE=$(date "+%Y%m%d")
    export GDAL_RELEASE_DATE
fi

if [ -z "${GDAL_BUILD_IS_RELEASE:-}" ]; then
    export GDAL_SHA1SUM=${GDAL_VERSION}
fi

mkdir gdal
wget -q "https://github.com/OSGeo/gdal/archive/${GDAL_VERSION}.tar.gz" \
    -O - | tar xz -C gdal --strip-components=1



(
    cd gdal/gdal
    if test "${RSYNC_REMOTE:-}" != ""; then
        echo "Downloading cache..."
        rsync -ra "${RSYNC_REMOTE}/gdal/" "$HOME/"
        echo "Finished"

        # Little trick to avoid issues with Python bindings
        printf "#!/bin/sh\nccache gcc \$*" > ccache_gcc.sh
        chmod +x ccache_gcc.sh
        printf "#!/bin/sh\nccache g++ \$*" > ccache_g++.sh
        chmod +x ccache_g++.sh
        export CC=$PWD/ccache_gcc.sh
        export CXX=$PWD/ccache_g++.sh

        ccache -M 1G
    fi

    ./configure --prefix=/usr \
    --without-libtool \
    --with-hide-internal-symbols \
    --with-python \
    --with-spatialite \
    --with-mysql \
    --with-proj="/build${PROJ_INSTALL_PREFIX-/usr/local}" \
    --with-libtiff=internal --with-rename-internal-libtiff-symbols \
    --with-geotiff=internal --with-rename-internal-libgeotiff-symbols \
    --with-pg \
    --with-spatialite \
    --with-geos \
    --with-fgdb=/usr/local/FileGDB_API

    make "-j$(nproc)"
    make install DESTDIR="/build"

    if [ -n "${RSYNC_REMOTE:-}" ]; then
        ccache -s

        echo "Uploading cache..."
        rsync -ra --delete "$HOME/.ccache" "${RSYNC_REMOTE}/gdal/"
        echo "Finished"

        rm -rf "$HOME/.ccache"
        unset CC
        unset CXX
    fi
)

rm -rf gdal
mkdir -p /build_gdal_python/usr/lib
mkdir -p /build_gdal_python/usr/bin
mkdir -p /build_gdal_version_changing/usr/include
mv /build/usr/lib/python3            /build_gdal_python/usr/lib
mv /build/usr/lib                    /build_gdal_version_changing/usr
mv /build/usr/include/gdal_version.h /build_gdal_version_changing/usr/include
mv /build/usr/bin/*.py               /build_gdal_python/usr/bin
mv /build/usr/bin                    /build_gdal_version_changing/usr

if [ "${WITH_DEBUG_SYMBOLS}" = "yes" ]; then
    # separate debug symbols
    for P in /build_gdal_version_changing/usr/lib/* /build_gdal_python/usr/lib/python3/dist-packages/osgeo/*.so /build_gdal_version_changing/usr/bin/*; do
        if file -h "$P" | grep -qi elf; then
            F=$(basename "$P")
            mkdir -p "$(dirname "$P")/.debug"
            DEBUG_P="$(dirname "$P")/.debug/${F}.debug"
            objcopy -v --only-keep-debug --compress-debug-sections "$P" "${DEBUG_P}"
            strip --strip-debug --strip-unneeded "$P"
            objcopy --add-gnu-debuglink="${DEBUG_P}" "$P"
        fi
    done
else
    for P in /build_gdal_version_changing/usr/lib/*; do strip -s "$P" 2>/dev/null || /bin/true; done
    for P in /build_gdal_python/usr/lib/python3/dist-packages/osgeo/*.so; do strip -s "$P" 2>/dev/null || /bin/true; done
    for P in /build_gdal_version_changing/usr/bin/*; do strip -s "$P" 2>/dev/null || /bin/true; done
fi
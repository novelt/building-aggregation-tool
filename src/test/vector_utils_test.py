# import pytest
#
# from shapely.geometry import Point
# from shapely.geometry import LineString
# from shapely.geometry import MultiPoint
# from novelt.lib.vector_utils import Geometry, GeometryCollection
#
#
# p1 = Point(0, 0)
# p2 = Point(1, 1)
# p3 = Point(1, 3)
# l1 = LineString(((0,0), (1,1)))
# e1 = Geometry([-180,-90,180,90], srid=4326)
# g1 = Geometry(p1)
# g2 = Geometry(p2)
# c1 = GeometryCollection([p1, p2])
# c0 = GeometryCollection()
# c2 = GeometryCollection([g1, g2])
# c3 = GeometryCollection([p1, l1], srid=10020)
# c4 = GeometryCollection(MultiPoint((p1, p2)))
# c5 = GeometryCollection(MultiPoint((p1, p3)), srid=1111)
# c6 = GeometryCollection(srid=2222)
# c7 = GeometryCollection(e1)
# c8 = GeometryCollection(source=conn, schema='data_work', table='settlements')
# c9 = GeometryCollection(source='/modules/MLI/input/manual_polygons/subset.shp')
#
#
# print('------------')
#
# for i,c in enumerate([c1, c2, c3,c4,c5,c6,c7,c8,c9]):
#     print(f'#{i+1}', 'SOURCE', c._source, '(MEMORY', c.is_memory, 'FILE', c.is_file, 'DATABASE', c.is_database, ')')
#     print('LENGTH', len(c))
#     print('GEOMETRIES', c.geometries, c.srid),
#     print('TYPE', c.geometry_type)
#     bounds = c.bounds.transform(srid=3857)
#     if bounds:
#         print(bounds.wkt)
#     print('------------')
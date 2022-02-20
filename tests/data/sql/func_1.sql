CREATE OR REPLACE FUNCTION geo.fn_do_any_coordinates_fall_inside(geom geometry(MultiPolygonZM), coordinates text[][])
    RETURNS boolean AS $$
SELECT
    -- Haha - a bit shit! Obvious need for immediate attention
    CASE array_length(coordinates, 1)
        WHEN 1 THEN
            ST_CONTAINS(geom, ST_POINT(coordinates[1][2]::double precision, coordinates[1][1]::double precision))
        WHEN 2 THEN
                ST_CONTAINS(geom, ST_POINT(coordinates[1][2]::double precision, coordinates[1][1]::double precision)) OR
                ST_CONTAINS(geom, ST_POINT(coordinates[2][2]::double precision, coordinates[2][1]::double precision))
        WHEN 3 THEN
                ST_CONTAINS(geom, ST_POINT(coordinates[1][2]::double precision, coordinates[1][1]::double precision)) OR
                ST_CONTAINS(geom, ST_POINT(coordinates[2][2]::double precision, coordinates[2][1]::double precision)) OR
                ST_CONTAINS(geom, ST_POINT(coordinates[3][2]::double precision, coordinates[3][1]::double precision))
        WHEN 4 THEN
                ST_CONTAINS(geom, ST_POINT(coordinates[1][2]::double precision, coordinates[1][1]::double precision)) OR
                ST_CONTAINS(geom, ST_POINT(coordinates[2][2]::double precision, coordinates[2][1]::double precision)) OR
                ST_CONTAINS(geom, ST_POINT(coordinates[3][2]::double precision, coordinates[3][1]::double precision)) OR
                ST_CONTAINS(geom, ST_POINT(coordinates[4][2]::double precision, coordinates[4][1]::double precision))
        WHEN 5 THEN
                ST_CONTAINS(geom, ST_POINT(coordinates[1][2]::double precision, coordinates[1][1]::double precision)) OR
                ST_CONTAINS(geom, ST_POINT(coordinates[2][2]::double precision, coordinates[2][1]::double precision)) OR
                ST_CONTAINS(geom, ST_POINT(coordinates[3][2]::double precision, coordinates[3][1]::double precision)) OR
                ST_CONTAINS(geom, ST_POINT(coordinates[4][2]::double precision, coordinates[4][1]::double precision)) OR
                ST_CONTAINS(geom, ST_POINT(coordinates[5][2]::double precision, coordinates[5][1]::double precision))
        WHEN 6 THEN
                ST_CONTAINS(geom, ST_POINT(coordinates[1][2]::double precision, coordinates[1][1]::double precision)) OR
                ST_CONTAINS(geom, ST_POINT(coordinates[2][2]::double precision, coordinates[2][1]::double precision)) OR
                ST_CONTAINS(geom, ST_POINT(coordinates[3][2]::double precision, coordinates[3][1]::double precision)) OR
                ST_CONTAINS(geom, ST_POINT(coordinates[4][2]::double precision, coordinates[4][1]::double precision)) OR
                ST_CONTAINS(geom, ST_POINT(coordinates[5][2]::double precision, coordinates[5][1]::double precision)) OR
                ST_CONTAINS(geom, ST_POINT(coordinates[6][2]::double precision, coordinates[6][1]::double precision))
        WHEN 7 THEN
                ST_CONTAINS(geom, ST_POINT(coordinates[1][2]::double precision, coordinates[1][1]::double precision)) OR
                ST_CONTAINS(geom, ST_POINT(coordinates[2][2]::double precision, coordinates[2][1]::double precision)) OR
                ST_CONTAINS(geom, ST_POINT(coordinates[3][2]::double precision, coordinates[3][1]::double precision)) OR
                ST_CONTAINS(geom, ST_POINT(coordinates[4][2]::double precision, coordinates[4][1]::double precision)) OR
                ST_CONTAINS(geom, ST_POINT(coordinates[5][2]::double precision, coordinates[5][1]::double precision)) OR
                ST_CONTAINS(geom, ST_POINT(coordinates[6][2]::double precision, coordinates[6][1]::double precision)) OR
                ST_CONTAINS(geom, ST_POINT(coordinates[7][2]::double precision, coordinates[7][1]::double precision))
        WHEN 8 THEN
                ST_CONTAINS(geom, ST_POINT(coordinates[1][2]::double precision, coordinates[1][1]::double precision)) OR
                ST_CONTAINS(geom, ST_POINT(coordinates[2][2]::double precision, coordinates[2][1]::double precision)) OR
                ST_CONTAINS(geom, ST_POINT(coordinates[3][2]::double precision, coordinates[3][1]::double precision)) OR
                ST_CONTAINS(geom, ST_POINT(coordinates[4][2]::double precision, coordinates[4][1]::double precision)) OR
                ST_CONTAINS(geom, ST_POINT(coordinates[5][2]::double precision, coordinates[5][1]::double precision)) OR
                ST_CONTAINS(geom, ST_POINT(coordinates[6][2]::double precision, coordinates[6][1]::double precision)) OR
                ST_CONTAINS(geom, ST_POINT(coordinates[7][2]::double precision, coordinates[7][1]::double precision)) OR
                ST_CONTAINS(geom, ST_POINT(coordinates[8][2]::double precision, coordinates[8][1]::double precision))
        WHEN 9 THEN
                ST_CONTAINS(geom, ST_POINT(coordinates[1][2]::double precision, coordinates[1][1]::double precision)) OR
                ST_CONTAINS(geom, ST_POINT(coordinates[2][2]::double precision, coordinates[2][1]::double precision)) OR
                ST_CONTAINS(geom, ST_POINT(coordinates[3][2]::double precision, coordinates[3][1]::double precision)) OR
                ST_CONTAINS(geom, ST_POINT(coordinates[4][2]::double precision, coordinates[4][1]::double precision)) OR
                ST_CONTAINS(geom, ST_POINT(coordinates[5][2]::double precision, coordinates[5][1]::double precision)) OR
                ST_CONTAINS(geom, ST_POINT(coordinates[6][2]::double precision, coordinates[6][1]::double precision)) OR
                ST_CONTAINS(geom, ST_POINT(coordinates[7][2]::double precision, coordinates[7][1]::double precision)) OR
                ST_CONTAINS(geom, ST_POINT(coordinates[8][2]::double precision, coordinates[8][1]::double precision)) OR
                ST_CONTAINS(geom, ST_POINT(coordinates[9][2]::double precision, coordinates[9][1]::double precision))
        WHEN 10 THEN
                ST_CONTAINS(geom, ST_POINT(coordinates[1][2]::double precision, coordinates[1][1]::double precision)) OR
                ST_CONTAINS(geom, ST_POINT(coordinates[2][2]::double precision, coordinates[2][1]::double precision)) OR
                ST_CONTAINS(geom, ST_POINT(coordinates[3][2]::double precision, coordinates[3][1]::double precision)) OR
                ST_CONTAINS(geom, ST_POINT(coordinates[4][2]::double precision, coordinates[4][1]::double precision)) OR
                ST_CONTAINS(geom, ST_POINT(coordinates[5][2]::double precision, coordinates[5][1]::double precision)) OR
                ST_CONTAINS(geom, ST_POINT(coordinates[6][2]::double precision, coordinates[6][1]::double precision)) OR
                ST_CONTAINS(geom, ST_POINT(coordinates[7][2]::double precision, coordinates[7][1]::double precision)) OR
                ST_CONTAINS(geom, ST_POINT(coordinates[8][2]::double precision, coordinates[8][1]::double precision)) OR
                ST_CONTAINS(geom, ST_POINT(coordinates[9][2]::double precision, coordinates[9][1]::double precision)) OR
                ST_CONTAINS(geom, ST_POINT(coordinates[10][2]::double precision, coordinates[10][1]::double precision))
        ELSE
            FALSE
        END
$$
    LANGUAGE SQL;
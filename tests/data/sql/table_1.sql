CREATE TABLE geo.states
(
    gid serial NOT NULL,
    statefp character varying(2),
    statens character varying(8),
    affgeoid character varying(11),
    geoid character varying(2),
    stusps character varying(2),
    name character varying(100),
    lsad character varying(2),
    aland double precision,
    awater double precision,
    geom geometry(MultiPolygonZM),
    CONSTRAINT pk_geo_states PRIMARY KEY (gid)
);
CREATE OR REPLACE FUNCTION collab.sp_new_tax(country character varying(2), state character varying(10), definition json)
    RETURNS INT AS $$
DECLARE
    cid int;
    sid int;
    new_id int;
BEGIN

    SELECT INTO cid id FROM reference_data.countries WHERE iso=country;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'country % not found', country;
    END IF;

    IF state = 'FED' THEN
        sid = NULL;
    ELSE
        SELECT INTO sid id FROM reference_data.states WHERE iso=state AND country_id=cid;
        IF NOT FOUND THEN
            RAISE EXCEPTION 'state % not found', state;
        END IF;
    END IF;

    INSERT INTO collab.tax_definitions(country_id, state_id, status_id, date_created, date_modified, definition)
    VALUES(cid, sid, 1, now(), now(), definition) RETURNING id INTO new_id;

    RETURN new_id;
END;
$$ LANGUAGE plpgsql;
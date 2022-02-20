use regex::Regex;

fn execute_tests<const N: usize>(tests: [&str; N]) {
    for test in tests {
        let tree = match pg_parse::parse(test) {
            Ok(tree) => tree,
            Err(e) => panic!("Failed to parse: {}: \"{}\"", e, test),
        };

        // Compare converting this back to a string
        let sql = tree
            .iter()
            .map(|stmt| stmt.to_string())
            .collect::<Vec<_>>()
            .join("; ");
        assert_eq!(
            test, sql,
            "expected <> generated to_string:\n\n{:?}\n\n",
            tree
        );

        // Parse it back and compare the original trees
        let reparsed_tree = pg_parse::parse(&sql).unwrap();
        assert_eq!(format!("{:?}", tree), format!("{:?}", reparsed_tree));
    }
}

#[test]
fn it_correctly_converts_to_string_for_select_tests() {
    let tests = [
        "SELECT 1",
        "SELECT 1; SELECT 2",
        "SELECT 1 FROM t(1)",
        "SELECT a AS b FROM x WHERE y = 5 AND z = y",
        "SELECT FROM x WHERE y = 5 AND z = y",
        "SELECT a AS b FROM public.x WHERE y = 5 AND z = y",
        "SELECT DISTINCT a, b, * FROM c WHERE d = e",
        "SELECT DISTINCT ON (a) a, b FROM c",
        "SELECT * INTO films_recent FROM films WHERE date_prod >= '2002-01-01'",
        "SELECT current_timestamp",
        "SELECT current_time(2)",
        "SELECT memory_total_bytes, memory_swap_total_bytes - memory_swap_free_bytes AS swap, date_part(?, s.collected_at) AS collected_at FROM snapshots s INNER JOIN system_snapshots ON snapshot_id = s.id WHERE s.database_id = ? AND s.collected_at >= ? AND s.collected_at <= ? ORDER BY collected_at ASC",
        "SELECT * FROM a ORDER BY x ASC NULLS FIRST",
        "SELECT * FROM a ORDER BY x ASC NULLS LAST",
        "SELECT * FROM a ORDER BY x COLLATE \"tr_TR\" DESC NULLS LAST",
        "SELECT 'foo' COLLATE \"tr_TR\"",
        "SELECT id, name FROM table1 UNION (SELECT id, name FROM table2 ORDER BY name) ORDER BY id ASC",
        "SELECT a FROM kodsis EXCEPT SELECT a FROM application",
        "SELECT * FROM (VALUES ('anne', 'smith'), ('bob', 'jones'), ('joe', 'blow')) names(first, last)",
        "SELECT * FROM users WHERE name LIKE 'postgresql:%'",
        "SELECT * FROM users WHERE name NOT LIKE 'postgresql:%'",
        "SELECT * FROM users WHERE name ILIKE 'postgresql:%'",
        "SELECT * FROM users WHERE name NOT ILIKE 'postgresql:%'",
        "SELECT OVERLAY(m.name PLACING '******' FROM 3 FOR 6) AS tc_kimlik FROM tb_test m",
        "SELECT sum(price_cents) FROM products",
        "SELECT ARRAY(SELECT id FROM products)::bigint[]",
        "SELECT m.name AS mname, pname FROM manufacturers m, LATERAL get_product_names(m.id) pname",
        "SELECT m.name AS mname, pname FROM manufacturers m LEFT JOIN LATERAL get_product_names(m.id) pname ON true",
        "SELECT * FROM tb_test_main mh INNER JOIN LATERAL (SELECT ftnrm.* FROM test ftnrm WHERE ftnrm.hizmet_id = mh.id UNION ALL SELECT ftarc.* FROM test.test2 ftarc WHERE ftarc.hizmet_id = mh.id) ft ON true",
        "SELECT x, y FROM a CROSS JOIN b",
        "SELECT x, y FROM a NATURAL INNER JOIN b",
        "SELECT x, y FROM a LEFT JOIN b ON 1 > 0",
        "SELECT x, y FROM a RIGHT JOIN b ON 1 > 0",
        "SELECT x, y FROM a FULL JOIN b ON 1 > 0",
        "SELECT x, y FROM a INNER JOIN b USING (z)",
        "SELECT 2 + 2",
        "SELECT * FROM x WHERE y IS NULL",
        "SELECT * FROM x WHERE y IS NOT NULL",
        "SELECT count(*) FROM x WHERE y IS NOT NULL",
        "SELECT count(DISTINCT a) FROM x WHERE y IS NOT NULL",
        "SELECT CASE WHEN a.status = 1 THEN 'active' WHEN a.status = 2 THEN 'inactive' END FROM accounts a",
        "SELECT CASE 1 > 0 WHEN true THEN 'ok' ELSE NULL END",
        "SELECT CASE WHEN a.status = 1 THEN 'active' WHEN a.status = 2 THEN 'inactive' ELSE 'unknown' END FROM accounts a",
        "SELECT * FROM accounts WHERE status = CASE WHEN x = 1 THEN 'active' ELSE 'inactive' END",
        "SELECT CASE WHEN EXISTS (SELECT 1) THEN 1 ELSE 2 END",
        "SELECT (SELECT 'x')",
        "SELECT * FROM (SELECT generate_series(0, 100)) a",
        "SELECT * FROM x WHERE id IN (1, 2, 3)",
        "SELECT * FROM x WHERE id IN (SELECT id FROM account)",
        "SELECT * FROM x WHERE id NOT IN (1, 2, 3)",
        "SELECT * FROM x INNER JOIN (SELECT n FROM z) b ON a.id = b.id",
        "SELECT * FROM x WHERE y = z[?]",
        "SELECT (foo(1)).y",
        "SELECT proname, (SELECT regexp_split_to_array(proargtypes::text, ' '))[idx] AS argtype, proargnames[idx] AS argname FROM pg_proc",
        "SELECT COALESCE((SELECT customer.sp_person(n.id) AS sp_person).city_id, NULL::int) AS city_id FROM customer.tb_customer n",
        "SELECT * FROM x WHERE y = z[?][?]",
        "SELECT (k #= hstore('{id}'::text[], ARRAY[1::text])).* FROM test k",
        "SELECT * FROM x WHERE NOT y",
        "SELECT * FROM x WHERE x OR y",
        "SELECT 1 WHERE (1 = 1 OR 1 = 2) AND 1 = 2",
        "SELECT 1 WHERE (1 = 1 AND 2 = 2) OR 2 = 3",
        "SELECT 1 WHERE 1 = 1 OR 2 = 2 OR 2 = 3",
        "SELECT * FROM x WHERE x = ALL(?)",
        "SELECT * FROM x WHERE x = ANY(?)",
        "SELECT * FROM x WHERE x = COALESCE(y, ?)",
        "SELECT a, b, max(c) FROM c WHERE d = 1 GROUP BY a, b",
        "SELECT * FROM x LIMIT 50",
        "SELECT * FROM x OFFSET 50",
        "SELECT amount * 0.5",
        "SELECT * FROM x WHERE x BETWEEN '2016-01-01' AND '2016-02-02'",
        "SELECT * FROM x WHERE x NOT BETWEEN '2016-01-01' AND '2016-02-02'",
        "SELECT * FROM x WHERE x BETWEEN SYMMETRIC 20 AND 10",
        "SELECT * FROM x WHERE x NOT BETWEEN SYMMETRIC 20 AND 10",
        "SELECT NULLIF(id, 0) AS id FROM x",
        "SELECT NULL FROM x",
        "SELECT * FROM x WHERE y IS TRUE",
        "SELECT * FROM x WHERE y IS NOT TRUE",
        "SELECT * FROM x WHERE y IS FALSE",
        "SELECT * FROM x WHERE y IS NOT FALSE",
        "SELECT * FROM x WHERE y IS UNKNOWN",
        "SELECT * FROM x WHERE y IS NOT UNKNOWN",
        "SELECT * FROM crosstab('SELECT department, role, COUNT(id) FROM users GROUP BY department, role ORDER BY department, role', 'VALUES (''admin''::text), (''ordinary''::text)') AS (department varchar, admin int, ordinary int)",
        "SELECT * FROM crosstab('SELECT department, role, COUNT(id) FROM users GROUP BY department, role ORDER BY department, role', 'VALUES (''admin''::text), (''ordinary''::text)') ctab (department varchar, admin int, ordinary int)",
        "SELECT row_cols[0] AS dept, row_cols[1] AS sub, admin, ordinary FROM crosstab('SELECT ARRAY[department, sub] AS row_cols, role, COUNT(id) FROM users GROUP BY department, role ORDER BY department, role', 'VALUES (''admin''::text), (''ordinary''::text)') AS (row_cols varchar[], admin int, ordinary int)",
        "SELECT 1::int8",
        "SELECT CAST(1 + 3 AS int8)",
        "SELECT ?::regclass",
        "SELECT table_field::bool, table_field::boolean FROM t",
        "SELECT true, false",
        "SELECT 1::boolean, 0::boolean",
        "SELECT $5",
        "SELECT ?",
        "SELECT * FROM people FOR UPDATE OF name, email",
        "SELECT name::varchar(255) FROM people",
        "SELECT name::varchar FROM people",
        "SELECT age::numeric(5, 2) FROM people",
        "SELECT age::numeric FROM people",
        "SELECT m.name AS mname, pname FROM manufacturers m LEFT JOIN LATERAL get_product_names(m.id) pname ON true",
        "SELECT * FROM a CROSS JOIN (b CROSS JOIN c)",
        "SELECT 1 FOR UPDATE",
        "SELECT 1 FOR UPDATE NOWAIT",
        "SELECT 1 FOR UPDATE SKIP LOCKED",
        "SELECT rank(*) OVER ()",
        "SELECT rank(*) OVER (PARTITION BY id)",
        "SELECT rank(*) OVER (ORDER BY id)",
        "SELECT rank(*) OVER (PARTITION BY id, id2 ORDER BY id DESC, id2)",
        "SELECT rank(*) OVER named_window",
        "SELECT max(create_date::date) FILTER (WHERE cancel_date IS NULL) OVER (ORDER BY create_date DESC) FROM tb_x",
        "SELECT ROW(1 + 2)",
        "SELECT (3 + 3) OPERATOR(pg_catalog.*) 2",
        "SELECT 3 + (3 * 2)",
        "SELECT LIMIT ALL",
        "SELECT * FROM ROWS FROM (foo() AS (foo_res_a text COLLATE a, foo_res_b text))",
        "SELECT 1 FROM a.b.c",
        "SELECT sum(unique1) FILTER (WHERE unique1 IN (SELECT unique1 FROM onek WHERE unique1 < 100)) FROM tenk1",
        "SELECT customer_id, sum(amount) FROM payment GROUP BY customer_id HAVING sum(amount) > 200",
        "SELECT *, lag(emp_salary, 1) OVER (ORDER BY emp_salary ASC) AS previous_salary FROM employee",
        "SELECT *, lead(emp_salary, 1) OVER (ORDER BY emp_salary ASC) AS previous_salary FROM employee",
        "SELECT emp_id, emp_salary, emp_address, rank() OVER (PARTITION BY emp_address ORDER BY emp_salary DESC) FROM employee",
        "SELECT emp_id, emp_salary, emp_address, row_number() OVER (PARTITION BY emp_address ORDER BY emp_salary DESC) FROM employee",
        "SELECT emp_id, emp_salary, emp_address, dense_rank() OVER (PARTITION BY emp_address ORDER BY emp_salary DESC) FROM employee",
        "SELECT emp_id, emp_salary, emp_address, ntile(1) OVER (PARTITION BY emp_address ORDER BY emp_salary DESC) FROM employee",
        "SELECT emp_id, emp_salary, emp_address, percent_rank() OVER (PARTITION BY emp_address ORDER BY emp_salary DESC) FROM employee",
        "SELECT emp_id, emp_salary, emp_address, cume_dist() OVER (PARTITION BY emp_address ORDER BY emp_salary DESC) FROM employee",
        "SELECT emp_id, emp_salary, emp_address, first_value(emp_id) OVER (PARTITION BY emp_address ORDER BY emp_salary DESC) FROM employee",
        "SELECT emp_id, emp_salary, emp_address, last_value(emp_id) OVER (PARTITION BY emp_address ORDER BY emp_salary DESC) FROM employee",
        "SELECT emp_id, emp_salary, emp_address, nth_value(emp_salary, 2) OVER (PARTITION BY emp_address ORDER BY emp_salary DESC) FROM employee",
        "SELECT film_id, title, length INTO TEMPORARY short_film FROM film WHERE length < 60 ORDER BY title",
        "SELECT film_id, title, length INTO UNLOGGED short_film FROM film WHERE length < 60 ORDER BY title",
        "SELECT group_name, avg(price) FROM products INNER JOIN product_groups USING (group_id) GROUP BY group_name",
        "SELECT product_name, price, group_name, avg(price) OVER (PARTITION BY group_name) FROM products INNER JOIN product_groups USING (group_id)",
        "SELECT wf1() OVER (PARTITION BY c1 ORDER BY c2), wf2() OVER (PARTITION BY c1 ORDER BY c2) FROM table_name",
        "SELECT wf1() OVER w, wf2() OVER w FROM table_name WINDOW w AS (PARTITION BY c1 ORDER BY c2)",
        "SELECT product_name, group_name, price, row_number() OVER (PARTITION BY group_name ORDER BY price) FROM products INNER JOIN product_groups USING (group_id)",
        "SELECT product_name, group_name, price, rank() OVER (PARTITION BY group_name ORDER BY price) FROM products INNER JOIN product_groups USING (group_id)",
        "SELECT product_name, group_name, price, dense_rank() OVER (PARTITION BY group_name ORDER BY price) FROM products INNER JOIN product_groups USING (group_id)",
        "SELECT product_name, group_name, price, first_value(price) OVER (PARTITION BY group_name ORDER BY price) AS lowest_price_per_group FROM products INNER JOIN product_groups USING (group_id)",
        "SELECT product_name, group_name, price, last_value(price) OVER (PARTITION BY group_name ORDER BY price RANGE BETWEEN UNBOUNDED PRECEDING AND UNBOUNDED FOLLOWING) AS highest_price_per_group FROM products INNER JOIN product_groups USING (group_id)",
        "SELECT product_name, group_name, price, lag(price, 1) OVER (PARTITION BY group_name ORDER BY price) AS prev_price, price - lag(price, 1) OVER (PARTITION BY group_name ORDER BY price) AS cur_prev_diff FROM products INNER JOIN product_groups USING (group_id)",
        "SELECT product_name, group_name, price, lead(price, 1) OVER (PARTITION BY group_name ORDER BY price) AS next_price, price - lead(price, 1) OVER (PARTITION BY group_name ORDER BY price) AS cur_next_diff FROM products INNER JOIN product_groups USING (group_id)",
        "SELECT foo, bar FROM atable WHERE foo IS DISTINCT FROM bar",
        "SELECT foo, bar FROM atable WHERE foo IS NOT DISTINCT FROM bar",
        "SELECT t1.foo, t1.bar, t1.baz FROM t1 LEFT JOIN t2 ON t1.foo IS NOT DISTINCT FROM t2.foo AND t1.bar IS NOT DISTINCT FROM t2.bar AND t1.baz IS NOT DISTINCT FROM t2.baz WHERE t2.foo IS NULL",
        "SELECT country_name FROM countries WHERE (country_name SIMILAR TO 'New Zealand') = 't'",
        "SELECT country_name FROM countries WHERE country_name SIMILAR TO 'New Zealand' IS TRUE",
        "SELECT country_name FROM countries WHERE country_name SIMILAR TO 'New Zealand'",
        "SELECT country_name FROM countries WHERE country_name NOT SIMILAR TO 'New Zealand'",
        "SELECT location, sum(quantity) FROM sales GROUP BY ROLLUP (location)",
        "SELECT location, product, sum(quantity) FROM sales GROUP BY ROLLUP (location, product) ORDER BY location, product",
        "SELECT COALESCE(location, 'All locations') AS location, COALESCE(product, 'All products') AS product, sum(quantity) FROM sales GROUP BY ROLLUP (location, product) ORDER BY location, product",
        "SELECT COALESCE(location, 'All locations') AS location, COALESCE(product, 'All products') AS product, sum(quantity) FROM sales GROUP BY CUBE (location, product) ORDER BY location, product",
        "SELECT GROUPING(brand) AS grouping_brand, GROUPING(segment) AS grouping_segment, brand, segment, sum(quantity) FROM sales GROUP BY GROUPING SETS (brand, segment, ()) ORDER BY brand, segment",
        "SELECT GROUPING(brand) AS grouping_brand, GROUPING(segment) AS grouping_segment, brand, segment, sum(quantity) FROM sales GROUP BY GROUPING SETS (brand, segment, ()) HAVING GROUPING(brand) = 0 ORDER BY brand, segment",
        "SELECT film_id, title, length FROM film WHERE length > ALL (SELECT round(avg(length), 2) FROM film GROUP BY rating) ORDER BY length",
        "SELECT title, category_id FROM film INNER JOIN film_category USING (film_id) WHERE category_id = ANY (SELECT category_id FROM category WHERE name = 'Action' OR name = 'Drama')",
        "SELECT current_date",
        "SELECT current_time",
        "SELECT current_time(2)",
        "SELECT current_timestamp",
        "SELECT current_timestamp(0)",
        "SELECT localtime",
        "SELECT localtime(0)",
        "SELECT localtimestamp",
        "SELECT localtimestamp(2)",
        "SELECT current_catalog",
        "SELECT current_role",
        "SELECT current_schema",
        "SELECT current_user",
        "SELECT user",
        "SELECT session_user",
        "SELECT GREATEST(1, 2, 3, 4, 5)",
        "SELECT LEAST(1, 2, 3, 4, 5)",
    ];
    execute_tests(tests)
}

#[test]
fn it_correctly_converts_to_string_for_with_tests() {
    let tests = [
        "WITH kodsis AS (SELECT * FROM application), kodsis2 AS (SELECT * FROM application) SELECT * FROM kodsis UNION SELECT * FROM kodsis ORDER BY id DESC",
        "WITH t AS (SELECT random() AS x FROM generate_series(1, 3)) SELECT * FROM t",
        "WITH RECURSIVE search_graph(id, link, data, depth, path, cycle) AS (SELECT g.id, g.link, g.data, 1, ARRAY[ROW(g.f1, g.f2)], false FROM graph g UNION ALL SELECT g.id, g.link, g.data, sg.depth + 1, path || ROW(g.f1, g.f2), ROW(g.f1, g.f2) = ANY(path) FROM graph g, search_graph sg WHERE g.id = sg.link AND NOT cycle) SELECT id, data, link FROM search_graph",
        "WITH moved AS (DELETE FROM employees WHERE manager_name = 'Mary') INSERT INTO employees_of_mary SELECT * FROM moved",
        "WITH archived AS (DELETE FROM employees WHERE manager_name = 'Mary') UPDATE users SET archived = true WHERE users.id IN (SELECT user_id FROM moved)",
        "WITH archived AS (DELETE FROM employees WHERE manager_name = 'Mary' RETURNING user_id) UPDATE users SET archived = true FROM archived WHERE archived.user_id = id RETURNING id",
        "WITH archived AS (DELETE FROM employees WHERE manager_name = 'Mary') DELETE FROM users WHERE users.id IN (SELECT user_id FROM moved)",
    ];
    execute_tests(tests)
}

#[test]
fn it_correctly_converts_to_string_for_insert_tests() {
    let tests = [
        "INSERT INTO x (y, z) VALUES (1, 'abc')",
        "INSERT INTO x (\"user\") VALUES ('abc')",
        "INSERT INTO x (y, z) VALUES (1, 'abc') RETURNING id",
        "INSERT INTO x SELECT * FROM y",
        "INSERT INTO x (y, z) VALUES (1, 'abc') ON CONFLICT (y) DO UPDATE SET \"user\" = excluded.\"user\" RETURNING y",
        "INSERT INTO x (y, z) VALUES (1, 'abc') ON CONFLICT (y) DO NOTHING RETURNING y",
        "INSERT INTO distributors (did, dname) VALUES (10, 'Conrad International') ON CONFLICT (did) WHERE is_active DO NOTHING",
        "INSERT INTO distributors (did, dname) VALUES (9, 'Antwerp Design') ON CONFLICT ON CONSTRAINT distributors_pkey DO NOTHING",
        "INSERT INTO employees SELECT * FROM people WHERE 1 = 1 GROUP BY name HAVING count(name) > 1 ORDER BY name DESC LIMIT 10 OFFSET 15 FOR UPDATE",
        "INSERT INTO films VALUES ('T_601', 'Yojimbo', 106, DEFAULT, 'Drama', DEFAULT)",
        "INSERT INTO jackdanger_card_totals (id, amount_cents, created_at) SELECT series.i, random() * 1000, (SELECT '2015-08-25 00:00:00 -0700'::timestamp + (('2015-08-25 23:59:59 -0700'::timestamp - '2015-08-25 00:00:00 -0700'::timestamp) * random())) FROM generate_series(1, 10000) series(i)",
    ];
    execute_tests(tests)
}

#[test]
fn it_correctly_converts_to_string_for_update_tests() {
    let tests = [
        "UPDATE x SET y = 1 WHERE z = 'abc'",
        "UPDATE ONLY x table_x SET y = 1 WHERE z = 'abc' RETURNING y AS changed_y",
        "UPDATE foo SET a = ?, b = ?",
        "UPDATE x SET \"user\" = 'emin'",
    ];

    execute_tests(tests)
}

#[test]
fn it_correctly_converts_to_string_for_delete_tests() {
    let tests = [
        "DELETE FROM x WHERE y = 1",
        "DELETE FROM ONLY x table_x USING table_z WHERE y = 1 RETURNING *",
    ];

    execute_tests(tests)
}

#[test]
fn it_correctly_converts_to_string_for_create_cast_tests() {
    let tests = [
        "CREATE CAST (bigint AS int4) WITH FUNCTION int4(bigint) AS ASSIGNMENT",
        "CREATE CAST (bigint AS int4) WITHOUT FUNCTION AS IMPLICIT",
        "CREATE CAST (bigint AS int4) WITH INOUT AS ASSIGNMENT",
    ];

    execute_tests(tests)
}

#[test]
fn it_correctly_converts_to_string_for_create_domain_tests() {
    let tests = [
        "CREATE DOMAIN us_postal_code AS text CHECK (\"VALUE\" ~ E'^\\\\d{5}$' OR \"VALUE\" ~ E'^\\\\d{5}-\\\\d{4}$')",
    ];

    execute_tests(tests)
}

#[test]
fn it_correctly_converts_to_string_for_create_function_tests() {
    let tests = [
        "CREATE FUNCTION getfoo(int) RETURNS SETOF users AS $$SELECT * FROM \"users\" WHERE users.id = $1;$$ LANGUAGE sql",
        "CREATE OR REPLACE FUNCTION getfoo(int) RETURNS SETOF users AS $$SELECT * FROM \"users\" WHERE users.id = $1;$$ LANGUAGE sql",
        "CREATE OR REPLACE FUNCTION getfoo(int) RETURNS SETOF users AS $$SELECT * FROM \"users\" WHERE users.id = $1;$$ LANGUAGE sql IMMUTABLE",
        "CREATE OR REPLACE FUNCTION getfoo(int) RETURNS SETOF users AS $$SELECT * FROM \"users\" WHERE users.id = $1;$$ LANGUAGE sql IMMUTABLE RETURNS NULL ON NULL INPUT",
        "CREATE OR REPLACE FUNCTION getfoo(int) RETURNS SETOF users AS $$SELECT * FROM \"users\" WHERE users.id = $1;$$ LANGUAGE sql IMMUTABLE CALLED ON NULL INPUT",
        "CREATE OR REPLACE FUNCTION getfoo() RETURNS text AS $$SELECT name FROM \"users\" LIMIT 1$$ LANGUAGE sql IMMUTABLE CALLED ON NULL INPUT",
    ];

    execute_tests(tests)
}

#[test]
fn it_correctly_converts_to_string_for_create_schema_tests() {
    let tests = [
        "CREATE SCHEMA myschema",
        "CREATE SCHEMA AUTHORIZATION joe",
        "CREATE SCHEMA IF NOT EXISTS test AUTHORIZATION joe",
        "CREATE SCHEMA hollywood CREATE TABLE films (title text, release date, awards text[]) CREATE VIEW winners AS SELECT title, release FROM films WHERE awards IS NOT NULL",
    ];
    execute_tests(tests)
}

#[test]
fn it_correctly_converts_to_string_for_create_table_tests() {
    let tests = [
        "CREATE UNLOGGED TABLE cities (name text, population real, altitude double, identifier smallint, postal_code int, foreign_id bigint)",
        "CREATE TABLE IF NOT EXISTS distributors (name varchar(40) DEFAULT 'Luso Films', len interval hour to second(3), name varchar(40) DEFAULT 'Luso Films', did int DEFAULT nextval('distributors_serial'), stamp timestamp DEFAULT now() NOT NULL, stamptz timestamp with time zone, time time NOT NULL, timetz time with time zone, CONSTRAINT name_len PRIMARY KEY (name, len))",
        "CREATE TABLE types (a real, b double precision, c numeric(2, 3), d char(4), e char(5), f varchar(6), g varchar(7))",
        "CREATE TABLE types (a geometry(point) NOT NULL)",
        "CREATE TABLE tablename (colname int NOT NULL DEFAULT nextval('tablename_colname_seq'))",
        "CREATE TABLE capitals (state char(2)) INHERITS (cities)",
        "CREATE TEMPORARY TABLE temp AS SELECT c FROM t",
        "CREATE TABLE films2 AS SELECT * FROM films",
        "CREATE TEMPORARY TABLE films_recent ON COMMIT DROP AS SELECT * FROM films WHERE date_prod > $1",
        "CREATE TABLE like_constraint_rename_cache (LIKE constraint_rename_cache INCLUDING ALL)",
    ];
    execute_tests(tests)
}

#[test]
fn it_correctly_converts_to_string_for_create_view_tests() {
    let tests = [
        "CREATE OR REPLACE TEMPORARY VIEW view_a AS SELECT * FROM a(1) WITH CHECK OPTION",
        "CREATE VIEW view_a (a, b) AS WITH RECURSIVE view_a(a, b) AS (SELECT * FROM a(1)) SELECT a, b FROM view_a",

    ];
    execute_tests(tests)
}

#[test]
fn it_correctly_converts_to_string_for_create_misc_tests() {
    let tests = [
        "CREATE AGGREGATE aggregate1 (int4) (sfunc = sfunc1, stype = stype1)",
        "CREATE AGGREGATE aggregate1 (int4, bool) (sfunc = sfunc1, stype = stype1)",
        "CREATE AGGREGATE aggregate1 (*) (sfunc = sfunc1, stype = stype1)",
        "CREATE AGGREGATE aggregate1 (int4) (sfunc = sfunc1, stype = stype1, finalfunc_extra, mfinalfuncextra)",
        "CREATE AGGREGATE aggregate1 (int4) (sfunc = sfunc1, stype = stype1, finalfunc_modify = read_only, parallel = restricted)",
        "CREATE AGGREGATE percentile_disc (float8 ORDER BY anyelement) (sfunc = ordered_set_transition, stype = internal, finalfunc = percentile_disc_final, finalfunc_extra)",
        "CREATE OPERATOR + (procedure = plusfunc)",
        "CREATE OPERATOR + (procedure = plusfunc, leftarg = int4, rightarg = int4)",
        "CREATE OPERATOR + (procedure = plusfunc, hashes, merges)",
        "CREATE TYPE type1",
        "CREATE TYPE type1 AS (attr1 int4, attr2 bool)",
        "CREATE TYPE type1 AS (attr1 int4 COLLATE collation1, attr2 bool)",
        "CREATE TYPE type1 AS ENUM ('value1', 'value2', 'value3')",
        "CREATE TYPE type1 AS RANGE (subtype = int4)",
        "CREATE TYPE type1 AS RANGE (subtype = int4, receive = receive_func, passedbyvalue)",
        "CREATE TYPE type1 (input = input1, output = output1)",
        "CREATE TYPE type1 (input = input1, output = output1, passedbyvalue)",
        "CREATE TABLESPACE x LOCATION 'a'",
        "CREATE TABLESPACE x OWNER a LOCATION 'b' WITH (random_page_cost=42, seq_page_cost=3)",
    ];
    execute_tests(tests)
}

#[test]
fn it_correctly_converts_to_string_for_drop_tests() {
    let tests = [
        "DROP SERVER IF EXISTS foo",
        "DROP PUBLICATION mypublication",
        "DROP TYPE box",
        "DROP TABLESPACE mystuff",
        "DROP CONVERSION myname",
        "DROP SEQUENCE serial",
        "DROP MATERIALIZED VIEW order_summary",
        "DROP TRIGGER if_dist_exists ON films",
        "DROP RULE newrule ON mytable",
        "DROP CAST (text AS int)",
        "DROP OPERATOR FAMILY float_ops USING btree",
        "DROP AGGREGATE myavg(int), myavg(bigint)",
        "DROP COLLATION german",
        "DROP FOREIGN DATA WRAPPER dbi",
        "DROP ACCESS METHOD heptree",
        "DROP STATISTICS IF EXISTS accounting.users_uid_creation, public.grants_user_role",
        "DROP TEXT SEARCH DICTIONARY english",
        "DROP OPERATOR CLASS widget_ops USING btree",
        "DROP POLICY p1 ON my_table",
        "DROP SUBSCRIPTION mysub",
        "DROP TEXT SEARCH CONFIGURATION my_english",
        "DROP EVENT TRIGGER snitch",
        "DROP TEXT SEARCH PARSER my_parser",
        "DROP EXTENSION hstore",
        "DROP DOMAIN box",
        "DROP TEXT SEARCH TEMPLATE thesaurus",
        "DROP TRANSFORM FOR hstore LANGUAGE plpythonu",
        "DROP FOREIGN TABLE films, distributors",
        "DROP FUNCTION sqrt(int)",
        "DROP FUNCTION update_employee_salaries()",
        "DROP FUNCTION update_employee_salaries",
        "DROP TABLE IF EXISTS any_table CASCADE",
        "DROP TABLE IF EXISTS any_table",
        "DROP SCHEMA IF EXISTS any_schema",
        "DROP VIEW kinds",
    ];
    execute_tests(tests)
}

#[test]
fn it_correctly_converts_to_string_for_alter_obj_tests() {
    let tests = [
        "ALTER TABLE distributors DROP CONSTRAINT distributors_pkey, ADD CONSTRAINT distributors_pkey PRIMARY KEY USING INDEX dist_id_temp_idx, ADD CONSTRAINT zipchk CHECK (char_length(zipcode) = 5), ALTER COLUMN tstamp DROP DEFAULT, ALTER COLUMN tstamp TYPE timestamp with time zone USING 'epoch'::timestamp with time zone + (date_part('epoch', tstamp) * '1 second'::interval), ALTER COLUMN tstamp SET DEFAULT now(), ALTER COLUMN tstamp DROP DEFAULT, ALTER COLUMN tstamp SET STATISTICS -5, ADD COLUMN some_int int NOT NULL, DROP COLUMN IF EXISTS other_column CASCADE",
        "ALTER TABLE distributors ADD CONSTRAINT distfk FOREIGN KEY (address) REFERENCES addresses (address)",
        "ALTER TABLE distributors ADD CONSTRAINT distfk FOREIGN KEY (address) REFERENCES addresses (address) NOT VALID",
        "ALTER TABLE a ALTER COLUMN b SET DEFAULT 1",
        "ALTER TABLE a ALTER COLUMN b DROP DEFAULT",
        "ALTER TABLE distributors RENAME CONSTRAINT zipchk TO zip_check",
        "ALTER TABLE distributors ADD COLUMN address varchar(30)",
        "ALTER TABLE distributors DROP COLUMN address",
        "ALTER TABLE distributors ALTER COLUMN address TYPE varchar(80), ALTER COLUMN name TYPE varchar(100)",
        "ALTER TABLE foo ALTER COLUMN foo_timestamp TYPE timestamp with time zone USING 'epoch'::timestamp with time zone + (foo_timestamp * '1 second'::interval)",
        "ALTER TABLE foo ALTER COLUMN foo_timestamp DROP DEFAULT, ALTER COLUMN foo_timestamp TYPE timestamp with time zone USING 'epoch'::timestamp with time zone + (foo_timestamp * '1 second'::interval), ALTER COLUMN foo_timestamp SET DEFAULT now()",
        "ALTER TABLE distributors RENAME COLUMN address TO city",
        "ALTER TABLE distributors RENAME TO suppliers",
        "ALTER TABLE distributors ALTER COLUMN street SET NOT NULL",
        "ALTER TABLE distributors ALTER COLUMN street DROP NOT NULL",
        "ALTER TABLE distributors ADD CONSTRAINT zipchk CHECK (char_length(zipcode) = 5)",
        "ALTER TABLE distributors DROP CONSTRAINT zipchk",
        "ALTER TABLE ONLY distributors DROP CONSTRAINT zipchk",
        "ALTER TABLE distributors ADD CONSTRAINT distfk FOREIGN KEY (address) REFERENCES addresses (address) MATCH FULL",
        "ALTER TABLE distributors ADD CONSTRAINT dist_id_zipcode_key UNIQUE (dist_id, zipcode)",
        "ALTER TABLE distributors ADD PRIMARY KEY (dist_id)",
        "ALTER TABLE distributors SET TABLESPACE fasttablespace",
        "ALTER TABLE myschema.distributors SET SCHEMA yourschema",
         "ALTER TABLE distributors DROP CONSTRAINT distributors_pkey, ADD CONSTRAINT distributors_pkey PRIMARY KEY USING INDEX dist_id_temp_idx",
        "ALTER TABLESPACE index_space RENAME TO fast_raid",
        "ALTER TABLESPACE x SET (seq_page_cost=3)",
        "ALTER TABLESPACE x RESET (random_page_cost)",
        "ALTER TRIGGER emp_stamp ON emp RENAME TO emp_track_chgs",
        "ALTER CONVERSION iso_8859_1_to_utf8 RENAME TO latin1_to_unicode",
        "ALTER MATERIALIZED VIEW foo RENAME TO bar",
        "ALTER COLLATION \"de_DE\" RENAME TO german",
        "ALTER TYPE electronic_mail RENAME TO email",
        "ALTER DOMAIN zipcode RENAME CONSTRAINT zipchk TO zip_check",
        "ALTER AGGREGATE myavg(int) RENAME TO my_average",
        "ALTER FUNCTION sqrt(int) RENAME TO square_root",
        "ALTER RULE notify_all ON emp RENAME TO notify_me",
        "ALTER VIEW foo RENAME TO bar",
        "ALTER FUNCTION x(y) DEPENDS ON EXTENSION a",
        "ALTER FUNCTION x(y) NO DEPENDS ON EXTENSION a",
        "ALTER PROCEDURE x(y) DEPENDS ON EXTENSION a",
        "ALTER ROUTINE x(y) DEPENDS ON EXTENSION a",
        "ALTER TRIGGER x ON y DEPENDS ON EXTENSION a",
        "ALTER MATERIALIZED VIEW x DEPENDS ON EXTENSION a",
        "ALTER SYSTEM SET fsync TO OFF",
        "ALTER SYSTEM RESET fsync",
    ];
    execute_tests(tests)
}

#[test]
fn it_correctly_converts_to_string_for_index_tests() {
    let tests = [
        "CREATE UNIQUE INDEX CONCURRENTLY dist_id_temp_idx ON distributors (dist_id)",
        "ALTER INDEX distributors RENAME TO suppliers",
        "ALTER INDEX x DEPENDS ON EXTENSION a",
        "DROP INDEX title_idx",
    ];
    execute_tests(tests)
}

#[test]
fn it_correctly_converts_to_string_for_permission_tests() {
    let tests = [
        "GRANT select ON \"table\" TO \"user\"",
        "GRANT select, update, insert ON \"table\" TO \"user\"",
        "GRANT select ON ALL TABLES IN SCHEMA schema TO \"user\"",
        "GRANT select ON \"table\" TO user1, user2",
        "GRANT select ON \"table\" TO public",
        "GRANT select ON \"table\" TO CURRENT_USER",
        "GRANT select ON \"table\" TO SESSION_USER",
        "GRANT ALL ON \"table\" TO \"user\"",
        "GRANT select ON \"table\" TO \"user\" WITH GRANT OPTION",
        "GRANT select (\"column\") ON \"table\" TO \"user\"",
        "GRANT select (column1, column2) ON \"table\" TO \"user\"",
        "GRANT usage ON SEQUENCE sequence TO \"user\"",
        "GRANT usage ON ALL SEQUENCES IN SCHEMA schema TO \"user\"",
        "GRANT create ON DATABASE database TO \"user\"",
        "GRANT usage ON DOMAIN domain TO \"user\"",
        "GRANT usage ON FOREIGN DATA WRAPPER fdw TO \"user\"",
        "GRANT usage ON FOREIGN SERVER server TO \"user\"",
        "GRANT execute ON FUNCTION function TO \"user\"",
        "GRANT execute ON FUNCTION function() TO \"user\"",
        "GRANT execute ON FUNCTION function(string) TO \"user\"",
        "GRANT execute ON FUNCTION function(string, string, boolean) TO \"user\"",
        "GRANT execute ON ALL FUNCTIONS IN SCHEMA schema TO \"user\"",
        "GRANT usage ON LANGUAGE plpgsql TO \"user\"",
        "GRANT select ON LARGE OBJECT 1234 TO \"user\"",
        "GRANT create ON SCHEMA schema TO \"user\"",
        "GRANT create ON TABLESPACE tablespace TO \"user\"",
        "GRANT usage ON TYPE type TO \"user\"",
        "GRANT role TO \"user\"",
        "GRANT role1, role2 TO \"user\"",
        "GRANT role TO \"user\" WITH ADMIN OPTION",
        "DROP ROLE jonathan",
        "REVOKE ALL ON kinds FROM manuel",
        "REVOKE admins FROM joe",
        "REVOKE insert ON films FROM public",
    ];
    execute_tests(tests)
}

#[test]
fn it_correctly_converts_to_string_for_database_tests() {
    let tests = [
        "CREATE DATABASE x OWNER abc CONNECTION LIMIT 5",
        "CREATE DATABASE x ENCODING \"SQL_ASCII\"",
        "CREATE DATABASE x LC_COLLATE \"en_US.UTF-8\"",
        "CREATE DATABASE x LOCATION DEFAULT",
        "CREATE DATABASE x TABLESPACE abc",
        "CREATE DATABASE x TEMPLATE TRUE",
        "ALTER DATABASE x CONNECTION LIMIT 5",
        "ALTER DATABASE x ALLOW_CONNECTIONS FALSE",
        "ALTER DATABASE x IS_TEMPLATE TRUE",
        "ALTER DATABASE x TABLESPACE abc",
        "ALTER DATABASE x SET work_mem TO \"10MB\"",
    ];
    execute_tests(tests)
}

#[test]
fn it_correctly_converts_to_string_for_extension_tests() {
    let tests = [
        "ALTER EXTENSION x UPDATE",
        "ALTER EXTENSION x UPDATE TO \"1.2\"",
        "ALTER EXTENSION x ADD ACCESS METHOD a",
        "ALTER EXTENSION x DROP ACCESS METHOD a",
        "ALTER EXTENSION x ADD AGGREGATE a(b)",
        "ALTER EXTENSION x ADD CAST (a AS b)",
        "ALTER EXTENSION x ADD COLLATION a",
        "ALTER EXTENSION x ADD CONVERSION a",
        "ALTER EXTENSION x ADD DOMAIN a",
        "ALTER EXTENSION x ADD FUNCTION a(b)",
        "ALTER EXTENSION x ADD LANGUAGE a",
        "ALTER EXTENSION x ADD OPERATOR ~~(a, b)",
        "ALTER EXTENSION x ADD OPERATOR CLASS a USING b",
        "ALTER EXTENSION x ADD OPERATOR FAMILY a USING b",
        "ALTER EXTENSION x ADD PROCEDURE a(b)",
        "ALTER EXTENSION x ADD ROUTINE a(b)",
        "ALTER EXTENSION x ADD SCHEMA a",
        "ALTER EXTENSION x ADD EVENT TRIGGER a",
        "ALTER EXTENSION x ADD TABLE a",
        "ALTER EXTENSION x ADD TEXT SEARCH PARSER a",
        "ALTER EXTENSION x ADD TEXT SEARCH DICTIONARY a",
        "ALTER EXTENSION x ADD TEXT SEARCH TEMPLATE a",
        "ALTER EXTENSION x ADD TEXT SEARCH CONFIGURATION a",
        "ALTER EXTENSION x ADD SEQUENCE a",
        "ALTER EXTENSION x ADD VIEW a",
        "ALTER EXTENSION x ADD MATERIALIZED VIEW a",
        "ALTER EXTENSION x ADD FOREIGN TABLE a",
        "ALTER EXTENSION x ADD FOREIGN DATA WRAPPER a",
        "ALTER EXTENSION x ADD SERVER a",
        "ALTER EXTENSION x ADD TRANSFORM FOR a LANGUAGE b",
        "ALTER EXTENSION x ADD TYPE a",
        "CREATE EXTENSION x",
        "CREATE EXTENSION IF NOT EXISTS x CASCADE VERSION \"1.2\" SCHEMA a",
    ];
    execute_tests(tests)
}

#[test]
fn it_correctly_converts_to_string_for_multi_statements() {
    let tests = [
        "SELECT m.name AS mname, pname FROM manufacturers m LEFT JOIN LATERAL get_product_names(m.id) pname ON true; INSERT INTO manufacturers_daily (a, b) SELECT a, b FROM manufacturers",
        "SELECT m.name AS mname, pname FROM manufacturers m LEFT JOIN LATERAL get_product_names(m.id) pname ON true; UPDATE users SET name = 'bobby; drop tables'; INSERT INTO manufacturers_daily (a, b) SELECT a, b FROM manufacturers",
    ];
    execute_tests(tests)
}

#[test]
fn it_correctly_converts_to_string_for_xml() {
    let tests = [
        "SELECT xmltable.* FROM xmldata, xmltable(('//ROWS/ROW') PASSING data COLUMNS id int PATH '@id', ordinality FOR ORDINALITY, \"COUNTRY_NAME\" text, country_id text PATH 'COUNTRY_ID', size_sq_km double precision PATH 'SIZE[@unit = \"sq_km\"]', size_other text PATH 'concat(SIZE[@unit!=\"sq_km\"], \" \", SIZE[@unit!=\"sq_km\"]/@unit)', premier_name text PATH 'PREMIER_NAME' DEFAULT 'not specified')",
        "SELECT xmlcomment('hello')",
        "SELECT xmlconcat('<abc/>', '<bar>foo</bar>')",
        "SELECT xmlconcat('<?xml version=\"1.1\"?><foo/>', '<?xml version=\"1.1\" standalone=\"no\"?><bar/>')",
        "SELECT xmlelement(name foo)",
        "SELECT xmlelement(name foo, xmlattributes('xyz' AS bar))",
        "SELECT xmlelement(name foo, xmlattributes(current_date AS bar), 'cont', 'ent')",
        "SELECT xmlelement(name \"foo$bar\", xmlattributes('xyz' AS \"a&b\"))",
        "SELECT xmlelement(name test, xmlattributes(a, b)) FROM test",
        "SELECT xmlelement(name foo, xmlattributes('xyz' AS bar), xmlelement(name abc), xmlcomment('test'), xmlelement(name xyz))",
        "SELECT xmlforest('abc' AS foo, 123 AS bar)",
        "SELECT xmlforest(table_name, column_name) FROM information_schema.columns WHERE table_schema = 'pg_catalog'",
        "SELECT xmlpi(name php, 'echo \"hello world\";')",
        "SELECT xmlroot(xmlparse(document '<?xml version=\"1.1\"?><content>abc</content>'), version '1.0', STANDALONE YES)",
        "SELECT xmlagg(x) FROM test",
        "SELECT xmlagg(x ORDER BY y DESC) FROM test",
        "SELECT xmlagg(x) FROM (SELECT * FROM test ORDER BY y DESC) tab",
        "SELECT xml IS DOCUMENT FROM test",
        "SELECT xpath('/my:a/text()', '<my:a xmlns:my=\"http://example.com\">test</my:a>', ARRAY[ARRAY['my', 'http://example.com']])",
        "SELECT xpath('//mydefns:b/text()', '<a xmlns=\"http://example.com\"><b>test</b></a>', ARRAY[ARRAY['mydefns', 'http://example.com']])",

    ];
    execute_tests(tests)
}

#[test]
fn it_correctly_converts_to_string_for_everything_else() {
    let tests = [
        "BEGIN",
        "BEGIN ISOLATION LEVEL SERIALIZABLE",
        "BEGIN READ ONLY",
        "BEGIN ISOLATION LEVEL READ COMMITTED, DEFERRABLE",
        "START TRANSACTION READ ONLY",
        "ROLLBACK",
        "ROLLBACK AND CHAIN",
        "COMMIT",
        "COMMIT AND CHAIN",
        "SAVEPOINT \"x y\"",
        "ROLLBACK TO SAVEPOINT x",
        "RELEASE x",
        "SET statement_timeout TO 10000",
        "SET search_path TO my_schema, public",
        "SET LOCAL search_path TO my_schema, public",
        "SET \"user\" TO ?",
        "VACUUM",
        "VACUUM t",
        "VACUUM (FULL) t",
        "VACUUM (FREEZE) t",
        "VACUUM (VERBOSE) t",
        "VACUUM (ANALYZE) t",
        "VACUUM (FULL, FREEZE, VERBOSE, ANALYZE)",
        "VACUUM (ANALYZE) t(a, b)",
        "LOCK TABLE t",
        "LOCK TABLE t, u",
        "EXPLAIN SELECT a FROM b",
        "EXPLAIN (ANALYZE) SELECT a FROM b",
        "EXPLAIN (ANALYZE, BUFFERS) SELECT a FROM b",
        "COPY t FROM STDIN",
        "COPY t(c1, c2) FROM STDIN",
        "COPY t FROM PROGRAM '/bin/false'",
        "COPY t FROM '/dev/null'",
        "COPY t TO STDOUT",
        "COPY (SELECT 1 FROM foo) TO STDOUT",
        "COPY t FROM STDIN WITH (convert_selectively, some_str test, some_num 1, some_list (a), some_star *)",
        "DO $$BEGIN PERFORM * FROM information_schema.tables; END$$",
        "DO LANGUAGE plpgsql $$ BEGIN PERFORM * FROM information_schema.tables; END $$",
        "DO $$ BEGIN PERFORM * FROM information_schema.tables; END $$ LANGUAGE plpgsql",
        "DISCARD ALL",
        "DISCARD PLANS",
        "DISCARD SEQUENCES",
        "DISCARD TEMP",
        "COMMENT ON POLICY a ON b IS 'test'",
        "COMMENT ON PROCEDURE a() IS 'test'",
        "COMMENT ON ROUTINE a() IS 'test'",
        "COMMENT ON TRANSFORM FOR int4 LANGUAGE sql IS 'test'",
        "COMMENT ON OPERATOR CLASS a USING b IS 'test'",
        "COMMENT ON OPERATOR FAMILY a USING b IS 'test'",
        "COMMENT ON LARGE OBJECT 42 IS 'test'",
        "COMMENT ON CAST (int4 AS int8) IS 'test'",
        "LOAD 'test file'",
        "COPY manual_export TO STDOUT WITH (FORMAT CSV, HEADER)",
        "PREPARE fooplan(int, text, bool, numeric) AS INSERT INTO foo VALUES ($1, $2, $3, $4); EXECUTE fooplan(1, 'Hunter Valley', 't', 200.00)",
        "PREPARE usrrptplan(int) AS SELECT * FROM users u, logs l WHERE u.usrid = $1 AND u.usrid = l.usrid AND l.date = $2; EXECUTE usrrptplan(1, current_date)",
        "UPDATE foo SET dataval = myval WHERE CURRENT OF curs1",
    ];
    execute_tests(tests)
}

#[test]
fn it_correctly_converts_to_string_for_complex_cases() {
    let location = Regex::new(r",\slocation:\s(-)?[\d]+").unwrap();
    let tests = [
        include_str!("data/sql/table_1.sql"),
        include_str!("data/sql/view_1.sql"),
        include_str!("data/sql/func_1.sql"),
        include_str!("data/sql/func_2.sql"),
    ];

    // We compare the tree only for these
    for test in tests {
        let tree = match pg_parse::parse(test) {
            Ok(tree) => tree,
            Err(e) => panic!("Failed to parse: {}: \"{}\"", e, test),
        };

        // Convert back to a string
        let sql = tree
            .iter()
            .map(|stmt| stmt.to_string())
            .collect::<Vec<_>>()
            .join("; ");

        // Parse it back and compare the original tree
        let reparsed_tree = pg_parse::parse(&sql).unwrap();

        // We strip out the location from each first
        let original = format!("{:?}", tree);
        let reparsed = format!("{:?}", reparsed_tree);
        let original = location.replace_all(&original, "").to_string();
        let reparsed = location.replace_all(&reparsed, "").to_string();
        assert_eq!(original, reparsed);
    }
}
